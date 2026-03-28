use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use tracing::error;

/// Application-wide error type that maps to HTTP status codes.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Resource was not found (404).
    #[error("not found: {0}")]
    NotFound(String),
    /// Request conflicts with existing state (409).
    #[error("conflict: {0}")]
    Conflict(String),
    /// Client sent an invalid request (400).
    #[error("bad request: {0}")]
    BadRequest(String),
    /// Unexpected server-side failure (500).
    #[error("internal: {0}")]
    Internal(String),
    /// Missing or invalid authentication credentials (401).
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    /// Authenticated but insufficient permissions (403).
    #[error("forbidden: {0}")]
    Forbidden(String),
    /// Remote agent returned an error (502).
    #[error("agent error: {0}")]
    AgentError(String),
    /// Remote agent is unreachable (503).
    #[error("agent unavailable: {0}")]
    AgentUnavailable(String),
    /// SQLite database error.
    #[error(transparent)]
    Db(#[from] rusqlite::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            AppError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            AppError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
            AppError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m.clone()),
            AppError::Forbidden(m) => (StatusCode::FORBIDDEN, m.clone()),
            AppError::AgentError(m) => (StatusCode::BAD_GATEWAY, m.clone()),
            AppError::AgentUnavailable(m) => (StatusCode::SERVICE_UNAVAILABLE, m.clone()),
            AppError::Db(e) => {
                error!("database error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal error".to_string(),
                )
            }
        };
        (status, Json(json!({"error": msg}))).into_response()
    }
}
