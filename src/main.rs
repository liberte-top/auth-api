use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

mod db;
mod entities;
mod schema;

use entities::accounts;
use entities::DbState;

#[derive(Serialize, ToSchema)]
struct Health {
    status: &'static str,
}

#[utoipa::path(
    get,
    path = "/api/v1/health",
    responses(
        (status = 200, description = "Service health", body = Health)
    )
)]
async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

#[derive(Deserialize, ToSchema)]
struct CreateAccount {
    account_type: String,
    username: String,
    email: Option<String>,
    phone: Option<String>,
    created_by: Option<Uuid>,
}

#[derive(Deserialize, ToSchema)]
struct UpdateAccount {
    username: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    updated_by: Option<Uuid>,
}

#[derive(Serialize, ToSchema)]
struct AccountResponse {
    uid: Uuid,
    account_type: String,
    username: String,
    email: Option<String>,
    phone: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
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
async fn create_account(
    State(state): State<DbState>,
    Json(payload): Json<CreateAccount>,
) -> Result<(StatusCode, Json<AccountResponse>), StatusCode> {
    let now = Utc::now();

    let model = accounts::ActiveModel {
        uid: Set(Uuid::new_v4()),
        account_type: Set(payload.account_type),
        username: Set(payload.username),
        email: Set(payload.email),
        phone: Set(payload.phone),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
        created_by: Set(payload.created_by),
        updated_by: Set(payload.created_by),
        ..Default::default()
    };

    let inserted = model
        .insert(&state.conn)
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
async fn get_account(
    State(state): State<DbState>,
    Path(uid): Path<String>,
) -> Result<Json<AccountResponse>, StatusCode> {
    let uid = Uuid::parse_str(&uid).map_err(|_| StatusCode::BAD_REQUEST)?;
    let account = accounts::Entity::find()
        .filter(accounts::Column::Uid.eq(uid))
        .filter(accounts::Column::DeletedAt.is_null())
        .one(&state.conn)
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
async fn update_account(
    State(state): State<DbState>,
    Path(uid): Path<String>,
    Json(payload): Json<UpdateAccount>,
) -> Result<Json<AccountResponse>, StatusCode> {
    let uid = Uuid::parse_str(&uid).map_err(|_| StatusCode::BAD_REQUEST)?;
    let account = accounts::Entity::find()
        .filter(accounts::Column::Uid.eq(uid))
        .filter(accounts::Column::DeletedAt.is_null())
        .one(&state.conn)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(model) = account else {
        return Err(StatusCode::NOT_FOUND);
    };

    let now = Utc::now();
    let mut active: accounts::ActiveModel = model.into();
    if let Some(username) = payload.username {
        active.username = Set(username);
    }
    if let Some(email) = payload.email {
        active.email = Set(Some(email));
    }
    if let Some(phone) = payload.phone {
        active.phone = Set(Some(phone));
    }
    active.updated_at = Set(now.into());
    active.updated_by = Set(payload.updated_by);

    let updated = active
        .update(&state.conn)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(updated.into()))
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
async fn delete_account(
    State(state): State<DbState>,
    Path(uid): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let uid = Uuid::parse_str(&uid).map_err(|_| StatusCode::BAD_REQUEST)?;
    let account = accounts::Entity::find()
        .filter(accounts::Column::Uid.eq(uid))
        .filter(accounts::Column::DeletedAt.is_null())
        .one(&state.conn)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(model) = account else {
        return Err(StatusCode::NOT_FOUND);
    };

    let now = Utc::now();
    let mut active: accounts::ActiveModel = model.into();
    active.deleted_at = Set(Some(now.into()));
    active.updated_at = Set(now.into());

    active
        .update(&state.conn)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(OpenApi)]
#[openapi(
    paths(health, create_account, get_account, update_account, delete_account),
    components(schemas(Health, CreateAccount, UpdateAccount, AccountResponse)),
    tags(
        (name = "health", description = "Health check"),
        (name = "accounts", description = "Accounts")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    let conn = match db::connect().await {
        Ok(conn) => conn,
        Err(err) => {
            eprintln!("database connection failed: {}", err);
            std::process::exit(1);
        }
    };
    if let Err(err) = schema::apply(&conn).await {
        eprintln!("schema apply failed: {}", err);
        std::process::exit(1);
    }
    let state = DbState { conn };

    let app = Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/accounts", post(create_account))
        .route("/api/v1/accounts/:uid", get(get_account))
        .route("/api/v1/accounts/:uid", patch(update_account))
        .route("/api/v1/accounts/:uid", delete(delete_account))
        .with_state(state)
        .merge(SwaggerUi::new("/api/docs").url("/api/openapi.json", ApiDoc::openapi()));
    let addr = SocketAddr::from(([0, 0, 0, 0], 3333));

    eprintln!("starting server on {}", addr);
    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("bind failed: {}", err);
            std::process::exit(1);
        }
    };
    eprintln!("bound on {}", addr);

    if let Err(err) = axum::serve(listener, app).await {
        eprintln!("serve error: {}", err);
        std::process::exit(1);
    }

    eprintln!("serve exited unexpectedly");
    std::process::exit(1);
}
