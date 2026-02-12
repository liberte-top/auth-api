use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::{
    repo::{account_authorizations::AccountAuthorizationsRepo, accounts::AccountsRepo},
    service::{
        accounts::AccountsService, auth::AuthService, config::ConfigService,
        session::SessionService, verification::VerificationService,
    },
};

pub trait DatabaseClient: Send + Sync {
    fn conn(&self) -> &DatabaseConnection;
}

pub struct SeaOrmDatabaseClient {
    conn: DatabaseConnection,
}

impl SeaOrmDatabaseClient {
    pub async fn new() -> Self {
        let conn = crate::db::connect()
            .await
            .expect("database connection failed");
        crate::schema::apply(&conn)
            .await
            .expect("schema apply failed");
        Self { conn }
    }
}

impl DatabaseClient for SeaOrmDatabaseClient {
    fn conn(&self) -> &DatabaseConnection {
        &self.conn
    }
}

pub struct AppState {
    db: Arc<dyn DatabaseClient>,
    accounts_repo: Arc<dyn AccountsRepo>,
    accounts: Arc<dyn AccountsService>,
    sessions: Arc<dyn SessionService>,
    auth: Arc<dyn AuthService>,
    verification: Arc<dyn VerificationService>,
    #[allow(dead_code)]
    account_authorizations_repo: Arc<dyn AccountAuthorizationsRepo>,
    config: Arc<dyn ConfigService>,
}

impl AppState {
    pub async fn new() -> Arc<Self> {
        let db = Arc::new(SeaOrmDatabaseClient::new().await);
        let accounts_repo = Arc::new(crate::repo::accounts::SeaOrmAccountsRepo::new(db.clone()));
        let account_credentials_repo = Arc::new(
            crate::repo::account_credentials::SeaOrmAccountCredentialsRepo::new(db.clone()),
        );
        let account_authorizations_repo = Arc::new(
            crate::repo::account_authorizations::SeaOrmAccountAuthorizationsRepo::new(db.clone()),
        );
        let accounts = Arc::new(crate::service::accounts::AccountsServiceImpl::new(
            db.clone(),
            accounts_repo.clone(),
            account_credentials_repo.clone(),
        ));
        let config = Arc::new(crate::service::config::ConfigServiceImpl::new());
        let redis_url = config
            .values()
            .redis_url
            .clone()
            .expect("REDIS_URL is not set");
        let sessions = Arc::new(
            crate::service::session::RedisSessionService::new(
                &redis_url,
                config.values().session_ttl_seconds,
                config.values().session_key_prefix.clone(),
            )
            .await
            .expect("redis connection failed"),
        );
        let verification = Arc::new(crate::service::verification::VerificationServiceImpl::new(
            account_authorizations_repo.clone(),
            config.values().verify_email_token_ttl_seconds,
        ));
        let auth = Arc::new(crate::service::auth::AuthServiceImpl::new(
            db.clone(),
            accounts_repo.clone(),
            account_credentials_repo.clone(),
            account_authorizations_repo.clone(),
            sessions.clone(),
            verification.clone(),
        ));

        Arc::new(Self {
            db,
            accounts_repo,
            accounts,
            sessions,
            auth,
            verification,
            account_authorizations_repo,
            config,
        })
    }

    pub fn db(&self) -> &dyn DatabaseClient {
        self.db.as_ref()
    }

    pub fn accounts(&self) -> &dyn AccountsService {
        self.accounts.as_ref()
    }

    pub fn sessions(&self) -> &dyn SessionService {
        self.sessions.as_ref()
    }

    pub fn accounts_repo(&self) -> &dyn AccountsRepo {
        self.accounts_repo.as_ref()
    }

    #[allow(dead_code)]
    pub fn account_authorizations_repo(&self) -> &dyn AccountAuthorizationsRepo {
        self.account_authorizations_repo.as_ref()
    }

    pub fn auth(&self) -> &dyn AuthService {
        self.auth.as_ref()
    }

    pub fn verification(&self) -> &dyn VerificationService {
        self.verification.as_ref()
    }

    pub fn config(&self) -> &dyn ConfigService {
        self.config.as_ref()
    }
}
