use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde::Serialize;
use thiserror::Error;
use tracing::error;

#[derive(Debug, Serialize)]
pub struct ErrorPayload<'a> {
    pub error: &'a str,
}

#[derive(Debug, Error)]
pub enum AppErrorKind {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn new<S: Into<String>>(status: StatusCode, message: S) -> Self {
        Self { status, message: message.into() }
    }

    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self { status: StatusCode::INTERNAL_SERVER_ERROR, message: message.into() }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if self.status.as_u16() >= 500 {
            error!(error = %self.message, code = %self.status, "request failed");
        }
        let payload = serde_json::json!({
            "error": self.message,
        });
        (self.status, Json(payload)).into_response()
    }
}

pub type Result<T> = std::result::Result<T, ApiError>;
