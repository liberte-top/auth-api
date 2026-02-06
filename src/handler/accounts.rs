use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    entities::accounts,
    service::accounts::{CreateAccountInput, UpdateAccountInput},
    state::AppState,
};

#[derive(Deserialize, ToSchema)]
pub struct CreateAccount {
    pub account_type: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub created_by: Option<Uuid>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateAccount {
    pub username: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub updated_by: Option<Uuid>,
}

#[derive(Serialize, ToSchema)]
pub struct AccountResponse {
    pub uid: Uuid,
    pub account_type: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl From<accounts::Model> for AccountResponse {
    fn from(model: accounts::Model) -> Self {
        Self {
            uid: model.uid,
            account_type: model.account_type,
            username: model.username,
            email: model.email,
            phone: model.phone,
            created_at: model.created_at.with_timezone(&Utc),
            updated_at: model.updated_at.with_timezone(&Utc),
            deleted_at: model.deleted_at.map(|dt| dt.with_timezone(&Utc)),
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/accounts",
    request_body = CreateAccount,
    responses(
        (status = 201, description = "Created", body = AccountResponse),
        (status = 400, description = "Invalid payload")
    )
)]
pub async fn create_account(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateAccount>,
) -> Result<(StatusCode, Json<AccountResponse>), StatusCode> {
    let input = CreateAccountInput {
        account_type: payload.account_type,
        username: payload.username,
        email: payload.email,
        phone: payload.phone,
        created_by: payload.created_by,
    };

    let inserted = state
        .accounts()
        .create(input)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok((StatusCode::CREATED, Json(inserted.into())))
}

#[utoipa::path(
    get,
    path = "/api/v1/accounts/{uid}",
    params(
        ("uid" = String, Path, description = "Account uid")
    ),
    responses(
        (status = 200, description = "Account", body = AccountResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_account(
    State(state): State<Arc<AppState>>,
    Path(uid): Path<String>,
) -> Result<Json<AccountResponse>, StatusCode> {
    let uid = Uuid::parse_str(&uid).map_err(|_| StatusCode::BAD_REQUEST)?;
    let account = state
        .accounts()
        .get(uid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match account {
        Some(model) => Ok(Json(model.into())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

#[utoipa::path(
    patch,
    path = "/api/v1/accounts/{uid}",
    request_body = UpdateAccount,
    params(
        ("uid" = String, Path, description = "Account uid")
    ),
    responses(
        (status = 200, description = "Updated", body = AccountResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_account(
    State(state): State<Arc<AppState>>,
    Path(uid): Path<String>,
    Json(payload): Json<UpdateAccount>,
) -> Result<Json<AccountResponse>, StatusCode> {
    let uid = Uuid::parse_str(&uid).map_err(|_| StatusCode::BAD_REQUEST)?;
    let input = UpdateAccountInput {
        username: payload.username,
        email: payload.email,
        phone: payload.phone,
        updated_by: payload.updated_by,
    };

    let updated = state
        .accounts()
        .update(uid, input)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    match updated {
        Some(model) => Ok(Json(model.into())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/accounts/{uid}",
    params(
        ("uid" = String, Path, description = "Account uid")
    ),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_account(
    State(state): State<Arc<AppState>>,
    Path(uid): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let uid = Uuid::parse_str(&uid).map_err(|_| StatusCode::BAD_REQUEST)?;
    let deleted = state
        .accounts()
        .delete(uid)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    match deleted {
        Some(_) => Ok(StatusCode::NO_CONTENT),
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/v1/accounts", post(create_account))
        .route("/api/v1/accounts/:uid", get(get_account))
        .route("/api/v1/accounts/:uid", patch(update_account))
        .route("/api/v1/accounts/:uid", delete(delete_account))
        .with_state(state)
}
