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
    pub verify_email_token_ttl_seconds: u64,
    pub cookie_secure: bool,
    pub cookie_domain: Option<String>,
    pub session_key_prefix: String,

    // Optional email delivery (cold-start friendly). When set, registration will send a
    // verification email via Resend.
    pub resend_api_key: Option<String>,
    pub email_from: Option<String>,
    pub verify_email_url_base: Option<String>,
    pub email_provider: Option<String>,

    pub smtp_host: Option<String>,
    pub smtp_port: Option<u16>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_starttls: bool,
}
