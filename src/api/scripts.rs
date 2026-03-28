use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;

use super::auth::AuthUser;
use super::{AppState, log_and_notify};
use crate::error::AppError;
use crate::models::*;
use crate::scripts::{ScriptFull, ScriptInfo};

#[derive(Deserialize)]
pub(crate) struct SaveScriptRequest {
    code: String,
}

pub(crate) async fn list_scripts(
    State(state): State<AppState>,
) -> Result<Json<Vec<ScriptInfo>>, AppError> {
    let store = state.script_store.clone();
    let scripts = tokio::task::spawn_blocking(move || store.list())
        .await
        .unwrap()?;
    Ok(Json(scripts))
}

pub(crate) async fn get_script(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ScriptFull>, AppError> {
    let store = state.script_store.clone();
    let script = tokio::task::spawn_blocking(move || store.get(&name))
        .await
        .unwrap()?;
    Ok(Json(script))
}

pub(crate) async fn save_script(
    State(state): State<AppState>,
    Path(name): Path<String>,
    auth: AuthUser,
    Json(req): Json<SaveScriptRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let store = state.script_store.clone();
    let name2 = name.clone();
    let code = req.code.clone();
    tokio::task::spawn_blocking(move || store.save(&name2, &code))
        .await
        .unwrap()?;

    log_and_notify(
        &state.db,
        &state.scheduler_tx,
        "script.saved",
        EventSeverity::Info,
        &format!("Script '{}' saved", name),
        None,
        None,
        &auth,
        None,
    )
    .await;

    Ok(Json(serde_json::json!({"status": "ok", "name": name})))
}

pub(crate) async fn delete_script(
    State(state): State<AppState>,
    Path(name): Path<String>,
    auth: AuthUser,
) -> Result<axum::http::StatusCode, AppError> {
    let store = state.script_store.clone();
    let name2 = name.clone();
    tokio::task::spawn_blocking(move || store.delete(&name2))
        .await
        .unwrap()?;

    log_and_notify(
        &state.db,
        &state.scheduler_tx,
        "script.deleted",
        EventSeverity::Warning,
        &format!("Script '{}' deleted", name),
        None,
        None,
        &auth,
        None,
    )
    .await;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
