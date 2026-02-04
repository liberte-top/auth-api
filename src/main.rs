use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[derive(Serialize)]
struct Health {
    status: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/api/v1/health", get(health));
    let addr = SocketAddr::from(([0, 0, 0, 0], 3333));
    println!("listening on {}", addr);

    let listener = TcpListener::bind(addr).await.expect("bind failed");
    if let Err(err) = axum::serve(listener, app).await {
        eprintln!("server error: {}", err);
        std::process::exit(1);
    }
}
