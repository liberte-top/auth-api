use crate::state::AppState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use axum_extra::extract::cookie::CookieJar;
use serde::Serialize;

#[derive(Serialize)]
struct ErrorResponse {
    message: String,
}

#[derive(Serialize)]
pub struct MeResponse {
    pub account_uid: String,
    pub username: Option<String>,
    pub email: Option<String>,
}

pub fn routes(state: std::sync::Arc<AppState>) -> axum::Router {
    axum::Router::new()
        .route("/api/v1/me", axum::routing::get(me))
        .with_state(state)
}

async fn me(State(state): State<std::sync::Arc<AppState>>, jar: CookieJar) -> impl IntoResponse {
    let Some(cookie) = jar.get("sid") else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                message: "missing session".to_string(),
            }),
        )
            .into_response();
    };

    let session = match state.sessions().get(cookie.value()).await {
        Ok(value) => value,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    message: format!("session lookup failed: {}", err),
                }),
            )
                .into_response();
        }
    };

    let Some(session) = session else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                message: "invalid session".to_string(),
            }),
        )
            .into_response();
    };

    let account = match state.accounts().get(session.account_uid).await {
        Ok(value) => value,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    message: format!("account lookup failed: {}", err),
                }),
            )
                .into_response();
        }
    };

    let Some(account) = account else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                message: "account not found".to_string(),
            }),
        )
            .into_response();
    };

    let response = MeResponse {
        account_uid: account.uid.to_string(),
        username: account.username,
        email: account.email,
    };
    (StatusCode::OK, Json(response)).into_response()
}
