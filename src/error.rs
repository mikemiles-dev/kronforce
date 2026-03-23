use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("internal: {0}")]
    Internal(String),
    #[error("agent error: {0}")]
    AgentError(String),
    #[error("agent unavailable: {0}")]
    AgentUnavailable(String),
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
            AppError::AgentError(m) => (StatusCode::BAD_GATEWAY, m.clone()),
            AppError::AgentUnavailable(m) => (StatusCode::SERVICE_UNAVAILABLE, m.clone()),
            AppError::Db(e) => {
                tracing::error!("database error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal error".to_string(),
                )
            }
        };
        (status, Json(json!({"error": msg}))).into_response()
    }
}
