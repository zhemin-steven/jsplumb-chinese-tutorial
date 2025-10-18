use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderValue, Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;

mod config;
mod errors;
mod security;
mod ws;

use crate::{config::Config, errors::ApiError};

#[derive(Clone)]
struct AppState {
    config: Arc<Config>,
}

#[tokio::main]
async fn main() {
    // Load .env if present and then read env vars
    let config = Config::from_env();

    // init tracing
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| format!("{}", config.default_rust_log()).into());

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .compact()
        .init();

    info!("starting server on {}", config.addr);

    let state = AppState {
        config: Arc::new(config.clone()),
    };

    // Build CORS layer from allowlist
    let cors = build_cors(&config);

    let api_router = Router::new()
        .route("/ws", get(ws::ws_handler))
        .route("/session", post(create_session))
        .route("/session/:id/accept-hostkey", post(accept_hostkey))
        .route("/encodings", get(list_encodings))
        .with_state(state.clone());

    let app = Router::new()
        .route("/health", get(health))
        .nest("/api", api_router)
        .fallback(handler_404)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors),
        );

    let listener = tokio::net::TcpListener::bind(config.addr)
        .await
        .expect("failed to bind address");

    axum::serve(listener, app).await.expect("server failed");
}

fn build_cors(config: &Config) -> CorsLayer {
    if config.allow_origins.iter().any(|o| o == "*" || o.to_lowercase() == "any") {
        CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers(Any)
            .allow_origin(Any)
    } else {
        let origins: Vec<HeaderValue> = config
            .allow_origins
            .iter()
            .filter_map(|o| HeaderValue::from_str(o).ok())
            .collect();
        CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers(Any)
            .allow_origin(origins)
    }
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({"status":"ok"}))
}

#[derive(Debug, Deserialize)]
struct CreateSessionRequest {
    #[serde(default)]
    host: String,
    #[serde(default)]
    port: Option<u16>,
    #[serde(default)]
    username: Option<String>,
}

#[derive(Debug, Serialize)]
struct CreateSessionResponse {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

async fn create_session(
    State(_state): State<AppState>,
    Json(_req): Json<CreateSessionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let resp = CreateSessionResponse {
        id: "sess_stub_1".to_string(),
        message: Some("session created (stub)".to_string()),
    };
    Ok((StatusCode::CREATED, Json(resp)))
}

#[derive(Debug, Deserialize)]
struct AcceptHostKeyRequest {
    #[serde(default)]
    accept: bool,
    #[serde(default)]
    fingerprint: Option<String>,
}

async fn accept_hostkey(
    State(_state): State<AppState>,
    Path(id): Path<String>,
    Json(_req): Json<AcceptHostKeyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    info!(session_id = %id, "accept-hostkey (stub)");
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
struct EncodingsResponse {
    encodings: Vec<&'static str>,
}

async fn list_encodings() -> Result<impl IntoResponse, ApiError> {
    let resp = EncodingsResponse {
        encodings: vec![
            "utf-8",
            "gbk",
            "gb2312",
            "big5",
            "shift_jis",
            "euc-jp",
            "iso-8859-1",
        ],
    };
    Ok(Json(resp))
}

async fn handler_404() -> impl IntoResponse {
    let err = ApiError::new(StatusCode::NOT_FOUND, "not found");
    err.into_response()
}
