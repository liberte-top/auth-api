#[derive(Clone)]
pub struct Config {
    pub port: u16,
    pub github_client_id: Option<String>,
    pub github_client_secret: Option<String>,
    pub github_redirect_url: Option<String>,
    pub github_authorize_url: String,
    pub github_token_url: String,
    pub github_api_base: String,
    pub redis_url: Option<String>,
    pub session_ttl_seconds: u64,
    pub cookie_secure: bool,
    pub cookie_domain: Option<String>,
    pub session_key_prefix: String,
}
