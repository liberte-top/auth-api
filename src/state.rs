use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::{
    repo::accounts::AccountsRepo,
    service::{accounts::AccountsService, config::ConfigService, session::SessionService},
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
    config: Arc<dyn ConfigService>,
}

impl AppState {
    pub async fn new() -> Arc<Self> {
        let db = Arc::new(SeaOrmDatabaseClient::new().await);
        let accounts_repo = Arc::new(crate::repo::accounts::SeaOrmAccountsRepo::new(db.clone()));
        let account_credentials_repo =
            Arc::new(crate::repo::account_credentials::SeaOrmAccountCredentialsRepo::new());
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

        Arc::new(Self {
            db,
            accounts_repo,
            accounts,
            sessions,
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

    pub fn config(&self) -> &dyn ConfigService {
        self.config.as_ref()
    }
}
