use async_trait::async_trait;
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::{
    entities::account_authorizations, repo::account_authorizations::AccountAuthorizationsRepo,
};

const TOKEN_TYPE_VERIFY_EMAIL: &str = "auth:verify_email";

#[derive(Debug)]
pub struct VerificationToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct VerificationError {
    pub code: &'static str,
    pub message: String,
}

impl VerificationError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[async_trait]
pub trait VerificationService: Send + Sync {
    async fn create_email_verification(
        &self,
        account_id: i64,
    ) -> Result<VerificationToken, VerificationError>;
    async fn verify_email_token(&self, token: &str) -> Result<i64, VerificationError>;
    fn email_verification_type(&self) -> &'static str;
}

pub struct VerificationServiceImpl {
    authorizations_repo: Arc<dyn AccountAuthorizationsRepo>,
    ttl_seconds: u64,
}

impl VerificationServiceImpl {
    pub fn new(authorizations_repo: Arc<dyn AccountAuthorizationsRepo>, ttl_seconds: u64) -> Self {
        Self {
            authorizations_repo,
            ttl_seconds,
        }
    }

    fn generate_token() -> String {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    }

    fn hash_token(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[async_trait]
impl VerificationService for VerificationServiceImpl {
    async fn create_email_verification(
        &self,
        account_id: i64,
    ) -> Result<VerificationToken, VerificationError> {
        let token = Self::generate_token();
        let token_hash = Self::hash_token(&token);
        let expires_at = Utc::now() + Duration::seconds(self.ttl_seconds as i64);

        let active = self
            .authorizations_repo
            .find_active_by_account_and_type(account_id, TOKEN_TYPE_VERIFY_EMAIL)
            .await
            .map_err(|err| VerificationError::new("db_error", err.to_string()))?;

        if let Some(existing) = active {
            let _ = self
                .authorizations_repo
                .revoke_by_id(existing.id)
                .await
                .map_err(|err| VerificationError::new("db_error", err.to_string()))?;
        }

        let model = account_authorizations::ActiveModel {
            account_id: sea_orm::Set(account_id),
            token_hash: sea_orm::Set(token_hash),
            token_type: sea_orm::Set(TOKEN_TYPE_VERIFY_EMAIL.to_string()),
            expires_at: sea_orm::Set(Some(expires_at.into())),
            revoked_at: sea_orm::Set(None),
            created_at: sea_orm::Set(Utc::now().into()),
            updated_at: sea_orm::Set(Utc::now().into()),
            ..Default::default()
        };

        self.authorizations_repo
            .insert(model)
            .await
            .map_err(|err| VerificationError::new("db_error", err.to_string()))?;

        Ok(VerificationToken { token, expires_at })
    }

    async fn verify_email_token(&self, token: &str) -> Result<i64, VerificationError> {
        let token_hash = Self::hash_token(token);
        let record = self
            .authorizations_repo
            .find_active_by_token_hash(&token_hash)
            .await
            .map_err(|err| VerificationError::new("db_error", err.to_string()))?;

        let Some(record) = record else {
            return Err(VerificationError::new(
                "invalid_token",
                "verification token is invalid",
            ));
        };

        if record.token_type != TOKEN_TYPE_VERIFY_EMAIL {
            return Err(VerificationError::new(
                "invalid_token",
                "verification token type mismatch",
            ));
        }

        self.authorizations_repo
            .revoke_by_id(record.id)
            .await
            .map_err(|err| VerificationError::new("db_error", err.to_string()))?;

        Ok(record.account_id)
    }

    fn email_verification_type(&self) -> &'static str {
        TOKEN_TYPE_VERIFY_EMAIL
    }
}
