mod http;
mod ws;
mod ssh;
mod security;
mod config;
mod errors;

use axum::{Router, routing::{get, post}};
use std::{net::SocketAddr, time::Duration};
use tower::limit::RateLimitLayer;
use tower_http::{trace::TraceLayer, limit::RequestBodyLimitLayer, services::ServeDir};
use tracing::{info, Level};

use crate::{config::Config, http as http_mod, ws as ws_mod, security::cors_layer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // init tracing subscriber
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cfg = Config::from_env();

    let static_dir = ServeDir::new("static");

    let app = Router::new()
        .route("/", get(http_mod::index))
        .nest_service("/static", static_dir)
        // API routes
        .route("/api/session", post(http_mod::create_session))
        .route("/api/session/:id/accept-hostkey", post(http_mod::accept_hostkey))
        .route("/api/encodings", get(http_mod::list_encodings))
        .route("/api/ws", get(ws_mod::ws_handler))
        // layers
        .layer(TraceLayer::new_for_http())
        .layer(RequestBodyLimitLayer::new(1 * 1024 * 1024))
        .layer(RateLimitLayer::new(200, Duration::from_secs(1)))
        .layer(cors_layer(&cfg));

    let addr = SocketAddr::from(([0, 0, 0, 0], cfg.port));
    info!("listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
