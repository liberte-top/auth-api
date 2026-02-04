use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

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

#[derive(OpenApi)]
#[openapi(
    paths(health),
    components(schemas(Health)),
    tags((name = "health", description = "Health check"))
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/api/v1/health", get(health))
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
