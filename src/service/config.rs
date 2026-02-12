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
    fn strip_wrapping_quotes(value: &str) -> &str {
        if value.len() >= 2 {
            let bytes = value.as_bytes();
            let first = bytes[0];
            let last = bytes[value.len() - 1];
            if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
                return &value[1..value.len() - 1];
            }
        }
        value
    }

    fn env_nonempty(key: &str) -> Option<String> {
        env::var(key).ok().and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return None;
            }
            let normalized = Self::strip_wrapping_quotes(trimmed).trim();
            if normalized.is_empty() {
                None
            } else {
                Some(normalized.to_string())
            }
        })
    }

    fn env_u16(key: &str) -> Option<u16> {
        Self::env_nonempty(key).and_then(|value| value.parse::<u16>().ok())
    }

    fn env_u64(key: &str) -> Option<u64> {
        Self::env_nonempty(key).and_then(|value| value.parse::<u64>().ok())
    }

    fn env_bool(key: &str, default: bool) -> bool {
        Self::env_nonempty(key)
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(default)
    }

    fn env_lower_nonempty(key: &str) -> Option<String> {
        Self::env_nonempty(key).map(|value| value.to_ascii_lowercase())
    }

    pub fn new() -> Self {
        let port = Self::env_u16("PORT").unwrap_or(3333);
        let github_client_id = Self::env_nonempty("AUTH_GITHUB_CLIENT_ID");
        let github_client_secret = Self::env_nonempty("AUTH_GITHUB_CLIENT_SECRET");
        let github_redirect_url = Self::env_nonempty("AUTH_GITHUB_REDIRECT_URL");
        let github_mock_enabled = Self::env_bool("AUTH_GITHUB_MOCK_ENABLED", false);
        let github_authorize_url =
            Self::env_nonempty("AUTH_GITHUB_AUTHORIZE_URL").unwrap_or_else(|| {
                if github_mock_enabled {
                    "http://localhost:3333/__github_mock__/login/oauth/authorize".to_string()
                } else {
                    "https://github.com/login/oauth/authorize".to_string()
                }
            });
        let github_token_url = Self::env_nonempty("AUTH_GITHUB_TOKEN_URL").unwrap_or_else(|| {
            if github_mock_enabled {
                "http://github-mock/login/oauth/access_token".to_string()
            } else {
                "https://github.com/login/oauth/access_token".to_string()
            }
        });
        let github_api_base = Self::env_nonempty("AUTH_GITHUB_API_BASE").unwrap_or_else(|| {
            if github_mock_enabled {
                "http://github-mock".to_string()
            } else {
                "https://api.github.com".to_string()
            }
        });
        let redis_url = Self::env_nonempty("REDIS_URL");
        let session_ttl_seconds = Self::env_u64("SESSION_TTL_SECONDS").unwrap_or(60 * 60 * 24 * 7);
        let verify_email_token_ttl_seconds =
            Self::env_u64("VERIFY_EMAIL_TOKEN_TTL_SECONDS").unwrap_or(60 * 60);
        let cookie_secure = Self::env_bool("COOKIE_SECURE", false);
        let cookie_domain = Self::env_nonempty("COOKIE_DOMAIN");
        let session_key_prefix =
            Self::env_nonempty("SESSION_KEY_PREFIX").unwrap_or_else(|| "auth-api".to_string());

        let resend_api_key = Self::env_nonempty("RESEND_API_KEY");
        let email_from = Self::env_nonempty("EMAIL_FROM");
        let verify_email_url_base = Self::env_nonempty("VERIFY_EMAIL_URL_BASE");
        let email_provider = Self::env_lower_nonempty("EMAIL_PROVIDER");
        let smtp_host = Self::env_nonempty("SMTP_HOST");
        let smtp_port = Self::env_u16("SMTP_PORT");
        let smtp_username = Self::env_nonempty("SMTP_USERNAME");
        let smtp_password = Self::env_nonempty("SMTP_PASSWORD");
        let smtp_starttls = Self::env_bool("SMTP_STARTTLS", false);

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
                verify_email_token_ttl_seconds,
                cookie_secure,
                cookie_domain,
                session_key_prefix,
                resend_api_key,
                email_from,
                verify_email_url_base,
                email_provider,
                smtp_host,
                smtp_port,
                smtp_username,
                smtp_password,
                smtp_starttls,
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
