//! Group management handlers: list, create, bulk-assign, rename.

use axum::Json;
use axum::extract::State;
use serde::Deserialize;
use uuid::Uuid;

use super::{AppState, AuthUser, DEFAULT_GROUP_NAME};
use super::{normalize_group, persist_group};
use crate::db::db_call;
use crate::error::AppError;

/// Returns the list of distinct group names across all jobs.
pub(crate) async fn list_groups(
    State(state): State<AppState>,
) -> Result<Json<Vec<String>>, AppError> {
    let groups = db_call(&state.db, |db| db.get_distinct_groups()).await?;
    Ok(Json(groups))
}

/// Request body for creating a new empty group.
#[derive(Deserialize)]
pub(crate) struct CreateGroupRequest {
    name: String,
}

/// Creates a new empty group (persisted in settings).
pub(crate) async fn create_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateGroupRequest>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_write()
    {
        return Err(AppError::Forbidden(
            "write access required (admin or operator role)".into(),
        ));
    }
    let name = normalize_group(Some(req.name))?.unwrap_or_else(|| DEFAULT_GROUP_NAME.to_string());
    let name_clone = name.clone();
    db_call(&state.db, move |db| db.add_custom_group(&name_clone)).await?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(serde_json::json!({"name": name})),
    ))
}

/// Request body for bulk group assignment.
#[derive(Deserialize)]
pub(crate) struct BulkGroupRequest {
    job_ids: Vec<Uuid>,
    group: Option<String>,
}

/// Assigns a group to multiple jobs at once.
pub(crate) async fn bulk_set_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<BulkGroupRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_write()
    {
        return Err(AppError::Forbidden(
            "write access required (admin or operator role)".into(),
        ));
    }
    let group = normalize_group(req.group)?;
    persist_group(&state.db, &group).await;
    let ids = req.job_ids;
    let count = db_call(&state.db, move |db| {
        db.bulk_set_group(&ids, group.as_deref())
    })
    .await?;
    Ok(Json(serde_json::json!({"updated": count})))
}

/// Request body for renaming a group.
#[derive(Deserialize)]
pub(crate) struct RenameGroupRequest {
    old_name: String,
    new_name: String,
}

/// Renames all jobs from one group to another.
pub(crate) async fn rename_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<RenameGroupRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_write()
    {
        return Err(AppError::Forbidden(
            "write access required (admin or operator role)".into(),
        ));
    }
    let new_name =
        normalize_group(Some(req.new_name))?.unwrap_or_else(|| DEFAULT_GROUP_NAME.to_string());
    let old_name = req.old_name;
    let old_clone = old_name.clone();
    let new_clone = new_name.clone();
    let count = db_call(&state.db, move |db| db.rename_group(&old_clone, &new_clone)).await?;

    let actor_id = auth.0.as_ref().map(|k| k.id);
    let actor_name = auth.0.as_ref().map(|k| k.name.clone());
    let details = format!("renamed '{}' to '{}'", old_name, new_name);
    let _ = db_call(&state.db, move |db| {
        db.record_audit(
            "group.renamed",
            "group",
            None,
            actor_id,
            actor_name.as_deref(),
            Some(&details),
        )
    })
    .await;

    Ok(Json(serde_json::json!({"updated": count})))
}
