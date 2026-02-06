use async_trait::async_trait;
use chrono::{DateTime, Utc};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::{fmt, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionData {
    pub account_uid: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug)]
pub enum SessionError {
    Redis(redis::RedisError),
    Serde(serde_json::Error),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::Redis(err) => write!(f, "redis error: {}", err),
            SessionError::Serde(err) => write!(f, "serde error: {}", err),
        }
    }
}

impl std::error::Error for SessionError {}

impl From<redis::RedisError> for SessionError {
    fn from(err: redis::RedisError) -> Self {
        SessionError::Redis(err)
    }
}

impl From<serde_json::Error> for SessionError {
    fn from(err: serde_json::Error) -> Self {
        SessionError::Serde(err)
    }
}

#[async_trait]
pub trait SessionService: Send + Sync {
    async fn create(&self, account_uid: Uuid) -> Result<String, SessionError>;
    async fn get(&self, session_id: &str) -> Result<Option<SessionData>, SessionError>;
    #[allow(dead_code)]
    async fn delete(&self, session_id: &str) -> Result<(), SessionError>;
}

pub struct RedisSessionService {
    conn: Arc<Mutex<MultiplexedConnection>>,
    ttl_seconds: u64,
    key_prefix: String,
}

impl RedisSessionService {
    pub async fn new(redis_url: &str, ttl_seconds: u64, key_prefix: String) -> Result<Self, SessionError> {
        let client = redis::Client::open(redis_url)?;
        let conn = client.get_multiplexed_async_connection().await?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            ttl_seconds,
            key_prefix,
        })
    }

    fn key(&self, session_id: &str) -> String {
        format!("{}:session:{}", self.key_prefix, session_id)
    }
}

#[async_trait]
impl SessionService for RedisSessionService {
    async fn create(&self, account_uid: Uuid) -> Result<String, SessionError> {
        let session_id = Uuid::new_v4().simple().to_string();
        let payload = SessionData {
            account_uid,
            created_at: Utc::now(),
        };
        let value = serde_json::to_string(&payload)?;

        let mut conn = self.conn.lock().await;
        let key = self.key(&session_id);
        conn.set_ex::<_, _, ()>(key, value, self.ttl_seconds).await?;
        Ok(session_id)
    }

    async fn get(&self, session_id: &str) -> Result<Option<SessionData>, SessionError> {
        let mut conn = self.conn.lock().await;
        let key = self.key(session_id);
        let value: Option<String> = conn.get(key).await?;
        let Some(value) = value else {
            return Ok(None);
        };
        let session = serde_json::from_str(&value)?;
        Ok(Some(session))
    }

    async fn delete(&self, session_id: &str) -> Result<(), SessionError> {
        let mut conn = self.conn.lock().await;
        let key = self.key(session_id);
        let _: () = conn.del(key).await?;
        Ok(())
    }
}
