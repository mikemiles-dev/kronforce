use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;

use super::AppState;
use super::auth::AuthUser;
use crate::db::db_call;
use crate::db::templates::JobTemplate;
use crate::error::AppError;

/// Returns all job templates.
pub(crate) async fn list_templates(
    State(state): State<AppState>,
) -> Result<Json<Vec<JobTemplate>>, AppError> {
    let templates = db_call(&state.db, move |db| db.list_templates()).await?;
    Ok(Json(templates))
}

/// Returns a single template by name.
pub(crate) async fn get_template(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<JobTemplate>, AppError> {
    let template = db_call(&state.db, move |db| db.get_template(&name)).await?;
    match template {
        Some(t) => Ok(Json(t)),
        None => Err(AppError::NotFound("template not found".into())),
    }
}

#[derive(Deserialize)]
pub(crate) struct SaveTemplateRequest {
    name: String,
    description: Option<String>,
    snapshot: serde_json::Value,
}

/// Creates or updates a job template.
pub(crate) async fn save_template(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<SaveTemplateRequest>,
) -> Result<(axum::http::StatusCode, Json<JobTemplate>), AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_write()
    {
        return Err(AppError::Forbidden("write access required".into()));
    }

    if req.name.is_empty() {
        return Err(AppError::BadRequest("template name required".into()));
    }

    let template = JobTemplate {
        name: req.name,
        description: req.description,
        snapshot: req.snapshot,
        created_by: auth.0.as_ref().map(|k| k.name.clone()),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let t = template.clone();
    db_call(&state.db, move |db| db.save_template(&t)).await?;

    Ok((axum::http::StatusCode::CREATED, Json(template)))
}

/// Deletes a template by name.
pub(crate) async fn delete_template(
    State(state): State<AppState>,
    Path(name): Path<String>,
    auth: AuthUser,
) -> Result<axum::http::StatusCode, AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_write()
    {
        return Err(AppError::Forbidden("write access required".into()));
    }

    let deleted = db_call(&state.db, move |db| db.delete_template(&name)).await?;
    if !deleted {
        return Err(AppError::NotFound("template not found".into()));
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}
