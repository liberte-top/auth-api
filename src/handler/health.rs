use axum::{routing::get, Json, Router};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct Health {
    pub status: &'static str,
}

#[utoipa::path(
    get,
    path = "/api/v1/health",
    responses(
        (status = 200, description = "Service health", body = Health)
    )
)]
pub async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

pub fn routes() -> Router {
    Router::new().route("/api/v1/health", get(health))
}
