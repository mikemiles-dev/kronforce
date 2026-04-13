//! Job version history handler.

use axum::Json;
use axum::extract::{Path, State};
use uuid::Uuid;

use super::AppState;
use crate::db::db_call;
use crate::error::AppError;

/// Returns version history for a job, newest first.
pub(crate) async fn list_job_versions(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let versions = db_call(&state.db, move |db| db.list_job_versions(id)).await?;
    Ok(Json(versions))
}
