use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/api/v1/health", get(health));

    let port = std::env::var("PORT").unwrap_or_else(|_| "3333".to_string());
    let bind_addr = format!("0.0.0.0:{port}");

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|_| panic!("failed to bind to {}", bind_addr));

    axum::serve(listener, app)
        .await
        .expect("server error");
}
