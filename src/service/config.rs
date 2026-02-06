use std::{env, sync::Arc};

use crate::config::Config;

pub trait ConfigService: Send + Sync {
    fn port(&self) -> u16;
    fn values(&self) -> &Config;
}

pub struct ConfigServiceImpl {
    config: Arc<Config>,
}

impl ConfigServiceImpl {
    pub fn new() -> Self {
        let port = env::var("PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(3333);
        let github_client_id = env::var("AUTH_GITHUB_CLIENT_ID").ok();
        let github_client_secret = env::var("AUTH_GITHUB_CLIENT_SECRET").ok();
        let github_redirect_url = env::var("AUTH_GITHUB_REDIRECT_URL").ok();
        let github_mock_enabled = env::var("AUTH_GITHUB_MOCK_ENABLED")
            .ok()
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let github_authorize_url = env::var("AUTH_GITHUB_AUTHORIZE_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| {
                if github_mock_enabled {
                    "http://localhost:3333/__github_mock__/login/oauth/authorize".to_string()
                } else {
                    "https://github.com/login/oauth/authorize".to_string()
                }
            });
        let github_token_url = env::var("AUTH_GITHUB_TOKEN_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| {
                if github_mock_enabled {
                    "http://github-mock/login/oauth/access_token".to_string()
                } else {
                    "https://github.com/login/oauth/access_token".to_string()
                }
            });
        let github_api_base = env::var("AUTH_GITHUB_API_BASE")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| {
                if github_mock_enabled {
                    "http://github-mock".to_string()
                } else {
                    "https://api.github.com".to_string()
                }
            });
        let redis_url = env::var("REDIS_URL").ok();
        let session_ttl_seconds = env::var("SESSION_TTL_SECONDS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(60 * 60 * 24 * 7);
        let cookie_secure = env::var("COOKIE_SECURE")
            .ok()
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let cookie_domain = env::var("COOKIE_DOMAIN").ok().and_then(|value| {
            if value.trim().is_empty() {
                None
            } else {
                Some(value)
            }
        });
        let session_key_prefix = env::var("SESSION_KEY_PREFIX")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "auth-api".to_string());

        Self {
            config: Arc::new(Config {
                port,
                github_client_id,
                github_client_secret,
                github_redirect_url,
                github_authorize_url,
                github_token_url,
                github_api_base,
                redis_url,
                session_ttl_seconds,
                cookie_secure,
                cookie_domain,
                session_key_prefix,
            }),
        }
    }
}

impl ConfigService for ConfigServiceImpl {
    fn port(&self) -> u16 {
        self.config.port
    }

    fn values(&self) -> &Config {
        &self.config
    }
}
