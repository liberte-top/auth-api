use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    Json,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use cookie::time::Duration;
use serde::{Deserialize, Serialize};

use crate::{
    service::accounts::{GetOrCreateByProviderSubjectInput},
    state::AppState,
};

#[derive(Deserialize)]
pub struct GithubCallbackQuery {
    code: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Serialize)]
pub struct GithubAuthResponse {
    pub account_uid: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub provider_subject: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    message: String,
}

#[derive(Deserialize)]
struct GithubTokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct GithubUserResponse {
    id: u64,
    login: String,
    email: Option<String>,
}

struct GithubOAuthConfig {
    client_id: String,
    client_secret: String,
    redirect_url: String,
    authorize_url: String,
    token_url: String,
    api_base: String,
}

pub fn routes(state: std::sync::Arc<AppState>) -> axum::Router {
    axum::Router::new()
        .route("/api/v1/auth/github", axum::routing::get(start_github_auth))
        .route(
            "/api/v1/auth/github/callback",
            axum::routing::get(github_callback),
        )
        .with_state(state)
}

fn github_config(
    state: &std::sync::Arc<AppState>,
) -> Result<GithubOAuthConfig, (StatusCode, Json<ErrorResponse>)> {
    let config = state.config().values();
    let Some(client_id) = &config.github_client_id else {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                message: "AUTH_GITHUB_CLIENT_ID is not set".to_string(),
            }),
        ));
    };
    let Some(client_secret) = &config.github_client_secret else {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                message: "AUTH_GITHUB_CLIENT_SECRET is not set".to_string(),
            }),
        ));
    };
    let Some(redirect_url) = &config.github_redirect_url else {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                message: "AUTH_GITHUB_REDIRECT_URL is not set".to_string(),
            }),
        ));
    };

    Ok(GithubOAuthConfig {
        client_id: client_id.clone(),
        client_secret: client_secret.clone(),
        redirect_url: redirect_url.clone(),
        authorize_url: config.github_authorize_url.clone(),
        token_url: config.github_token_url.clone(),
        api_base: config.github_api_base.clone(),
    })
}

async fn start_github_auth(State(state): State<std::sync::Arc<AppState>>) -> impl IntoResponse {
    let config = match github_config(&state) {
        Ok(config) => config,
        Err(response) => return response.into_response(),
    };
    let authorize_url = &config.authorize_url;
    let delimiter = if authorize_url.contains('?') { "&" } else { "?" };
    let url = format!(
        "{}{}client_id={}&redirect_uri={}&scope=read:user%20user:email",
        authorize_url,
        delimiter,
        urlencoding::encode(&config.client_id),
        urlencoding::encode(&config.redirect_url)
    );
    Redirect::temporary(&url).into_response()
}

async fn github_callback(
    State(state): State<std::sync::Arc<AppState>>,
    Query(query): Query<GithubCallbackQuery>,
) -> impl IntoResponse {
    if let Some(error) = query.error {
        let message = if let Some(desc) = query.error_description {
            format!("github oauth error: {} ({})", error, desc)
        } else {
            format!("github oauth error: {}", error)
        };
        return (StatusCode::BAD_REQUEST, Json(ErrorResponse { message })).into_response();
    }

    let Some(code) = query.code else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                message: "missing code".to_string(),
            }),
        )
            .into_response();
    };

    let config = match github_config(&state) {
        Ok(config) => config,
        Err(response) => return response.into_response(),
    };

    let client = reqwest::Client::new();
    let token = match client
        .post(&config.token_url)
        .header("Accept", "application/json")
        .form(&[
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("code", code.as_str()),
            ("redirect_uri", config.redirect_url.as_str()),
        ])
        .send()
        .await
    {
        Ok(response) => response,
        Err(err) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    message: format!("token request failed: {}", err),
                }),
            )
                .into_response();
        }
    };

    let token_response = match token.json::<GithubTokenResponse>().await {
        Ok(payload) => payload,
        Err(err) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    message: format!("token response parse failed: {}", err),
                }),
            )
                .into_response();
        }
    };

    let user_response = match client
        .get(format!("{}/user", config.api_base.trim_end_matches('/')))
        .header("Authorization", format!("Bearer {}", token_response.access_token))
        .header("User-Agent", "auth-api")
        .send()
        .await
    {
        Ok(response) => response,
        Err(err) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    message: format!("user request failed: {}", err),
                }),
            )
                .into_response();
        }
    };

    let user = match user_response.json::<GithubUserResponse>().await {
        Ok(payload) => payload,
        Err(err) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    message: format!("user response parse failed: {}", err),
                }),
            )
                .into_response();
        }
    };

    let input = GetOrCreateByProviderSubjectInput {
        provider: "github".to_string(),
        provider_subject: user.id.to_string(),
        account_type: "user".to_string(),
        username: None,
        email: None,
        created_by: None,
    };

    let account = match state.accounts().get_or_create_by_provider_subject(input).await {
        Ok(model) => model,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    message: format!("account upsert failed: {}", err),
                }),
            )
                .into_response();
        }
    };

    let session_id = match state.sessions().create(account.uid).await {
        Ok(value) => value,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    message: format!("session create failed: {}", err),
                }),
            )
                .into_response();
        }
    };

    let mut cookie = Cookie::new("sid", session_id);
    cookie.set_http_only(true);
    cookie.set_path("/");
    cookie.set_same_site(SameSite::Lax);
    cookie.set_max_age(Duration::seconds(
        state.config().values().session_ttl_seconds as i64,
    ));
    if state.config().values().cookie_secure {
        cookie.set_secure(true);
    }
    if let Some(domain) = &state.config().values().cookie_domain {
        cookie.set_domain(domain.to_string());
    }

    let response = GithubAuthResponse {
        account_uid: account.uid.to_string(),
        username: account.username,
        email: account.email,
        provider_subject: user.id.to_string(),
    };
    let jar = CookieJar::new().add(cookie);
    (StatusCode::OK, jar, Json(response)).into_response()
}
