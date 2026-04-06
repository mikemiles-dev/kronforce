use axum::Json;
use axum::extract::State;

use super::AppState;
use super::auth::AuthUser;
use crate::db::db_call;
use crate::error::AppError;

/// Exports all data for the current tenant (jobs, executions, variables, templates, events).
pub(crate) async fn export_data(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_manage_keys()
    {
        return Err(AppError::Forbidden("admin role required for data export".into()));
    }

    let db = state.db.clone();
    let data = tokio::task::spawn_blocking(move || {
        let jobs = db.list_jobs(None, None, None, 10000, 0)?;
        let variables = db.list_variables()?;
        let templates = db.list_templates()?;
        let agents = db.list_agents()?;
        let groups = db.get_distinct_groups()?;

        Ok::<_, AppError>(serde_json::json!({
            "export_version": 1,
            "exported_at": chrono::Utc::now().to_rfc3339(),
            "jobs": jobs,
            "variables": variables,
            "templates": templates,
            "agents": agents,
            "groups": groups,
        }))
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))??;

    Ok(Json(data))
}

/// Deletes all data: jobs, executions, variables, templates, events, audit log.
/// Requires admin role and confirmation header.
pub(crate) async fn delete_all_data(
    State(state): State<AppState>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, AppError> {
    // Require admin
    if let Some(key) = req.extensions().get::<crate::db::models::ApiKey>() {
        if !key.role.can_manage_keys() {
            return Err(AppError::Forbidden("admin role required".into()));
        }
    } else {
        return Err(AppError::Unauthorized("authentication required".into()));
    }

    // Require confirmation header to prevent accidental deletion
    let confirm = req
        .headers()
        .get("x-confirm-delete")
        .and_then(|v| v.to_str().ok());
    if confirm != Some("yes-delete-all-data") {
        return Err(AppError::BadRequest(
            "set header X-Confirm-Delete: yes-delete-all-data to confirm".into(),
        ));
    }

    let db = state.db.clone();
    let result = db_call(&db, move |db| db.delete_all_data()).await?;

    Ok(Json(result))
}
