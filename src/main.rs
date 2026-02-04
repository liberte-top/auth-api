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
