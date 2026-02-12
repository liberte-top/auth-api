use axum::Router;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod config;
mod db;
mod entities;
mod handler;
mod openapi;
mod repo;
mod schema;
mod service;
mod state;

use openapi::ApiDoc;
use state::AppState;

#[tokio::main]
async fn main() {
    let state = AppState::new().await;
    let _ = state.db().conn();
    let _ = state.accounts_repo();
    let _ = state.config().values();

    let app = Router::new()
        .merge(handler::health::routes())
        .merge(handler::accounts::routes(state.clone()))
        .merge(handler::auth::github::routes(state.clone()))
        .merge(handler::auth::password::routes(state.clone()))
        .merge(handler::session::routes(state.clone()))
        .merge(SwaggerUi::new("/api/docs").url("/api/openapi.json", ApiDoc::openapi()));
    let addr = SocketAddr::from(([0, 0, 0, 0], state.config().port()));

    eprintln!("starting server on {}", addr);
    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("bind failed: {}", err);
            std::process::exit(1);
        }
    };
    eprintln!("bound on {}", addr);

    axum::serve(listener, app).await.expect("serve error");
}
