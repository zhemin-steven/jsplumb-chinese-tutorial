use axum::{extract::Path, response::{Html, IntoResponse}, Json};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use crate::errors::AppError;

#[derive(Clone, Default)]
pub struct AppState;

pub async fn index() -> impl IntoResponse {
    Html(include_str!("../static/index.html"))
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    pub id: Uuid,
    pub hostkey_required: bool,
}

pub async fn create_session(
    Json(_payload): Json<CreateSessionRequest>,
) -> Result<Json<CreateSessionResponse>, AppError> {
    let id = Uuid::new_v4();
    info!(%id, "created session (stub)");
    Ok(Json(CreateSessionResponse { id, hostkey_required: false }))
}

#[derive(Debug, Serialize)]
pub struct AcceptHostKeyResponse {
    pub accepted: bool,
}

pub async fn accept_hostkey(Path(id): Path<Uuid>) -> Result<Json<AcceptHostKeyResponse>, AppError> {
    info!(%id, "accepted host key (stub)");
    Ok(Json(AcceptHostKeyResponse { accepted: true }))
}

pub async fn list_encodings() -> Json<Vec<&'static str>> {
    Json(vec![
        "utf-8",
        "utf-16",
        "iso-8859-1",
        "windows-1252",
    ])
}
