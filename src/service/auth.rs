use argon2::{password_hash::PasswordHash, Argon2, PasswordHasher, PasswordVerifier};
use async_trait::async_trait;
use chrono::Utc;
use rand::RngCore;
use sea_orm::TransactionTrait;
use std::sync::Arc;

use crate::{
    entities::{account_credentials, accounts},
    repo::{
        account_authorizations::AccountAuthorizationsRepo,
        account_credentials::AccountCredentialsRepo, accounts::AccountsRepo,
    },
    service::{session::SessionService, verification::VerificationService},
    state::DatabaseClient,
};

const PROVIDER_PASSWORD: &str = "password";

#[derive(Debug)]
pub struct AuthError {
    pub code: &'static str,
    pub message: String,
}

impl AuthError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug)]
pub struct RegisterOutput {
    pub account: accounts::Model,
    pub verify_token: String,
    pub verify_expires_at: chrono::DateTime<Utc>,
}

#[derive(Debug)]
pub struct LoginOutput {
    pub account: accounts::Model,
    pub session_id: String,
}

#[async_trait]
pub trait AuthService: Send + Sync {
    async fn register(
        &self,
        email: &str,
        username: Option<&str>,
        password: &str,
    ) -> Result<RegisterOutput, AuthError>;
    async fn login(&self, identifier: &str, password: &str) -> Result<LoginOutput, AuthError>;
}

pub struct AuthServiceImpl {
    db: Arc<dyn DatabaseClient>,
    accounts_repo: Arc<dyn AccountsRepo>,
    credentials_repo: Arc<dyn AccountCredentialsRepo>,
    authorizations_repo: Arc<dyn AccountAuthorizationsRepo>,
    sessions: Arc<dyn SessionService>,
    verification: Arc<dyn VerificationService>,
}

impl AuthServiceImpl {
    pub fn new(
        db: Arc<dyn DatabaseClient>,
        accounts_repo: Arc<dyn AccountsRepo>,
        credentials_repo: Arc<dyn AccountCredentialsRepo>,
        authorizations_repo: Arc<dyn AccountAuthorizationsRepo>,
        sessions: Arc<dyn SessionService>,
        verification: Arc<dyn VerificationService>,
    ) -> Self {
        Self {
            db,
            accounts_repo,
            credentials_repo,
            authorizations_repo,
            sessions,
            verification,
        }
    }

    fn normalize_email(email: &str) -> Result<String, AuthError> {
        let value = email.trim().to_lowercase();
        if value.is_empty() || !value.contains('@') {
            return Err(AuthError::new("invalid_email", "invalid email"));
        }
        Ok(value)
    }

    fn normalize_username(username: &str) -> Result<String, AuthError> {
        let value = username.trim().to_lowercase();
        if value.is_empty() {
            return Err(AuthError::new("invalid_username", "invalid username"));
        }
        Ok(value)
    }

    fn validate_password(password: &str) -> Result<(), AuthError> {
        if password.len() < 8 {
            return Err(AuthError::new(
                "invalid_password",
                "password must be at least 8 characters",
            ));
        }
        let mut has_upper = false;
        let mut has_lower = false;
        let mut has_digit = false;
        let mut has_special = false;
        for ch in password.chars() {
            if ch.is_ascii_uppercase() {
                has_upper = true;
            } else if ch.is_ascii_lowercase() {
                has_lower = true;
            } else if ch.is_ascii_digit() {
                has_digit = true;
            } else {
                has_special = true;
            }
        }
        if !(has_upper && has_lower && has_digit && has_special) {
            return Err(AuthError::new(
                "invalid_password",
                "password must include upper, lower, digit, and special character",
            ));
        }
        Ok(())
    }

    fn hash_password(password: &str) -> Result<String, AuthError> {
        let mut salt = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut salt);
        let salt = argon2::password_hash::SaltString::encode_b64(&salt)
            .map_err(|err| AuthError::new("password_hash_failed", err.to_string()))?;
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|err| AuthError::new("password_hash_failed", err.to_string()))?
            .to_string();
        Ok(hash)
    }

    fn verify_password(hash: &str, password: &str) -> Result<(), AuthError> {
        let parsed = PasswordHash::new(hash)
            .map_err(|_| AuthError::new("invalid_credentials", "invalid credentials"))?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .map_err(|_| AuthError::new("invalid_credentials", "invalid credentials"))
    }

    async fn ensure_email_available(&self, email: &str) -> Result<(), AuthError> {
        let existing = self
            .accounts_repo
            .find_by_email(email)
            .await
            .map_err(|err| AuthError::new("db_error", err.to_string()))?;
        if existing.is_some() {
            return Err(AuthError::new("email_taken", "email already registered"));
        }
        Ok(())
    }

    async fn ensure_username_available(&self, username: &str) -> Result<(), AuthError> {
        let existing = self
            .accounts_repo
            .find_by_username(username)
            .await
            .map_err(|err| AuthError::new("db_error", err.to_string()))?;
        if existing.is_some() {
            return Err(AuthError::new(
                "username_taken",
                "username already registered",
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl AuthService for AuthServiceImpl {
    async fn register(
        &self,
        email: &str,
        username: Option<&str>,
        password: &str,
    ) -> Result<RegisterOutput, AuthError> {
        let email = Self::normalize_email(email)?;
        let username = match username {
            Some(value) => Some(Self::normalize_username(value)?),
            None => None,
        };
        Self::validate_password(password)?;
        self.ensure_email_available(&email).await?;
        if let Some(value) = &username {
            self.ensure_username_available(value).await?;
        }

        let password_hash = Self::hash_password(password)?;
        let db = self.db.conn();
        let accounts_repo = self.accounts_repo.clone();
        let credentials_repo = self.credentials_repo.clone();

        let account = db
            .transaction(|txn| {
                let accounts_repo = accounts_repo.clone();
                let credentials_repo = credentials_repo.clone();
                let email = email.clone();
                let username = username.clone();
                let password_hash = password_hash.clone();
                Box::pin(async move {
                    let account_model = accounts::ActiveModel {
                        uid: sea_orm::Set(uuid::Uuid::new_v4()),
                        account_type: sea_orm::Set("user".to_string()),
                        username: sea_orm::Set(username.clone()),
                        email: sea_orm::Set(Some(email.clone())),
                        phone: sea_orm::Set(None),
                        created_by: sea_orm::Set(None),
                        updated_by: sea_orm::Set(None),
                        ..Default::default()
                    };

                    let account = accounts_repo.insert_with_txn(txn, account_model).await?;

                    let credential_model = account_credentials::ActiveModel {
                        account_id: sea_orm::Set(account.id),
                        provider: sea_orm::Set(PROVIDER_PASSWORD.to_string()),
                        provider_subject: sea_orm::Set(Some(email.clone())),
                        password_hash: sea_orm::Set(Some(password_hash)),
                        metadata: sea_orm::Set(None),
                        created_by: sea_orm::Set(None),
                        updated_by: sea_orm::Set(None),
                        ..Default::default()
                    };

                    credentials_repo
                        .insert_with_txn(txn, credential_model)
                        .await?;
                    Ok::<_, sea_orm::DbErr>(account)
                })
            })
            .await
            .map_err(|err| AuthError::new("db_error", err.to_string()))?;

        let verification = self
            .verification
            .create_email_verification(account.id)
            .await
            .map_err(|err| AuthError::new(err.code, err.message))?;

        Ok(RegisterOutput {
            account,
            verify_token: verification.token,
            verify_expires_at: verification.expires_at,
        })
    }

    async fn login(&self, identifier: &str, password: &str) -> Result<LoginOutput, AuthError> {
        let normalized = identifier.trim().to_lowercase();
        if normalized.is_empty() {
            return Err(AuthError::new("invalid_credentials", "invalid credentials"));
        }

        let account = if normalized.contains('@') {
            self.accounts_repo
                .find_by_email(&normalized)
                .await
                .map_err(|err| AuthError::new("db_error", err.to_string()))?
        } else {
            self.accounts_repo
                .find_by_username(&normalized)
                .await
                .map_err(|err| AuthError::new("db_error", err.to_string()))?
        };

        let Some(account) = account else {
            return Err(AuthError::new("invalid_credentials", "invalid credentials"));
        };

        let credential = self
            .credentials_repo
            .find_by_account_and_provider(account.id, PROVIDER_PASSWORD)
            .await
            .map_err(|err| AuthError::new("db_error", err.to_string()))?;

        let Some(credential) = credential else {
            return Err(AuthError::new("invalid_credentials", "invalid credentials"));
        };

        let Some(hash) = credential.password_hash else {
            return Err(AuthError::new("invalid_credentials", "invalid credentials"));
        };

        Self::verify_password(&hash, password)?;

        let pending_verification = self
            .authorizations_repo
            .find_active_by_account_and_type(
                account.id,
                self.verification.email_verification_type(),
            )
            .await
            .map_err(|err| AuthError::new("db_error", err.to_string()))?;

        if pending_verification.is_some() {
            return Err(AuthError::new(
                "email_not_verified",
                "email verification required",
            ));
        }

        let session_id = self
            .sessions
            .create(account.uid)
            .await
            .map_err(|err| AuthError::new("session_error", err.to_string()))?;

        Ok(LoginOutput {
            account,
            session_id,
        })
    }
}
