use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use cookie::time::Duration;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use crate::state::AppState;

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
}

fn error_response(status: StatusCode, code: &str, message: impl Into<String>) -> Response {
    (
        status,
        Json(ErrorResponse {
            code: code.to_string(),
            message: message.into(),
        }),
    )
        .into_response()
}

#[derive(Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub email: String,
    pub username: Option<String>,
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct RegisterResponse {
    pub account_uid: String,
    pub email: String,
    pub username: Option<String>,
    pub verification_required: bool,
    pub verification_expires_at: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    pub identifier: String,
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct LoginResponse {
    pub account_uid: String,
    pub username: Option<String>,
    pub email: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct VerifyEmailRequest {
    pub token: String,
}

#[derive(Serialize, ToSchema)]
pub struct VerifyEmailResponse {
    pub status: String,
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/verify-email", post(verify_email))
        .with_state(state)
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "Created", body = RegisterResponse),
        (status = 400, description = "Invalid payload", body = ErrorResponse)
    ),
    tag = "auth"
)]
pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterRequest>,
) -> Response {
    let output = match state
        .auth()
        .register(
            &payload.email,
            payload.username.as_deref(),
            &payload.password,
        )
        .await
    {
        Ok(output) => output,
        Err(err) => {
            return error_response(StatusCode::BAD_REQUEST, err.code, err.message);
        }
    };

    // Best-effort delivery: registration stays non-blocking for local/dev and smoke workflows.
    if let Err(err) = crate::service::email::try_send_verification_email(
        state.config().values(),
        &payload.email,
        &output.verify_token,
    )
    .await
    {
        eprintln!("warning: failed to send verification email: {}", err);
    }

    let response = RegisterResponse {
        account_uid: output.account.uid.to_string(),
        email: output.account.email.unwrap_or_default(),
        username: output.account.username,
        verification_required: true,
        verification_expires_at: Some(output.verify_expires_at.to_rfc3339()),
    };

    (StatusCode::CREATED, Json(response)).into_response()
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Logged in", body = LoginResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Invalid credentials", body = ErrorResponse),
        (status = 403, description = "Email not verified", body = ErrorResponse)
    ),
    tag = "auth"
)]
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Response {
    let output = match state
        .auth()
        .login(&payload.identifier, &payload.password)
        .await
    {
        Ok(output) => output,
        Err(err) => {
            let status = match err.code {
                "email_not_verified" => StatusCode::FORBIDDEN,
                "invalid_credentials" => StatusCode::UNAUTHORIZED,
                _ => StatusCode::BAD_REQUEST,
            };
            return error_response(status, err.code, err.message);
        }
    };

    let mut cookie = Cookie::new("sid", output.session_id);
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

    let response = LoginResponse {
        account_uid: output.account.uid.to_string(),
        username: output.account.username,
        email: output.account.email,
    };

    let jar = CookieJar::new().add(cookie);
    (StatusCode::OK, jar, Json(response)).into_response()
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    responses(
        (status = 204, description = "Logged out"),
        (status = 500, description = "Session delete failed", body = ErrorResponse)
    ),
    tag = "auth"
)]
pub async fn logout(State(state): State<Arc<AppState>>, jar: CookieJar) -> Response {
    let Some(cookie) = jar.get("sid") else {
        return StatusCode::NO_CONTENT.into_response();
    };

    if let Err(err) = state.sessions().delete(cookie.value()).await {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "session_error",
            err.to_string(),
        );
    }

    let mut cleared = Cookie::new("sid", "");
    cleared.set_http_only(true);
    cleared.set_path("/");
    cleared.set_same_site(SameSite::Lax);
    cleared.set_max_age(Duration::seconds(0));
    if state.config().values().cookie_secure {
        cleared.set_secure(true);
    }
    if let Some(domain) = &state.config().values().cookie_domain {
        cleared.set_domain(domain.to_string());
    }

    let jar = jar.add(cleared);
    (StatusCode::NO_CONTENT, jar).into_response()
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/verify-email",
    request_body = VerifyEmailRequest,
    responses(
        (status = 200, description = "Verified", body = VerifyEmailResponse),
        (status = 400, description = "Invalid or expired token", body = ErrorResponse)
    ),
    tag = "auth"
)]
pub async fn verify_email(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<VerifyEmailRequest>,
) -> Response {
    match state
        .verification()
        .verify_email_token(&payload.token)
        .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(VerifyEmailResponse {
                status: "ok".to_string(),
            }),
        )
            .into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, err.code, err.message),
    }
}
