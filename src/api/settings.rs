use axum::Json;
use axum::extract::State;

use super::AppState;
use super::auth::AuthUser;
use crate::db::db_call;
use crate::error::AppError;
use crate::executor::notifications::send_test;

/// Returns all system settings as a key-value map.
pub(crate) async fn get_settings(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let settings = db_call(&state.db, move |db| db.get_all_settings()).await?;
    Ok(Json(serde_json::json!(settings)))
}

/// Updates one or more settings from a key-value map.
pub(crate) async fn update_settings(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_write()
    {
        return Err(AppError::Forbidden(
            "write access required (admin or operator role)".into(),
        ));
    }
    db_call(&state.db, move |db| {
        for (key, value) in &body {
            db.set_setting(key, value)?;
        }
        Ok(())
    })
    .await?;

    let actor_key_id = auth.0.as_ref().map(|k| k.id);
    let actor_key_name = auth.0.as_ref().map(|k| k.name.clone());
    let db_audit = state.db.clone();
    let _ = db_call(&db_audit, move |db| {
        db.record_audit(
            "settings.updated",
            "settings",
            None,
            actor_key_id,
            actor_key_name.as_deref(),
            None,
        )
    })
    .await;

    Ok(Json(serde_json::json!({ "status": "ok" })))
}

/// Compacts the SQLite database by running VACUUM and truncating the WAL.
/// Admin only. Holds an exclusive write lock for the duration.
pub(crate) async fn vacuum_database(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let key = auth
        .0
        .as_ref()
        .ok_or_else(|| AppError::Unauthorized("authentication required".into()))?;
    if !key.role.can_manage_keys() {
        return Err(AppError::Forbidden("admin role required".into()));
    }

    let size_before = state
        .db
        .health_check()
        .and_then(|h| h.size_bytes)
        .unwrap_or(0);
    let started = std::time::Instant::now();
    db_call(&state.db, move |db| db.vacuum()).await?;
    let elapsed_ms = started.elapsed().as_millis() as u64;
    let size_after = state
        .db
        .health_check()
        .and_then(|h| h.size_bytes)
        .unwrap_or(0);

    let actor_key_id = Some(key.id);
    let actor_key_name = Some(key.name.clone());
    let db_audit = state.db.clone();
    let _ = db_call(&db_audit, move |db| {
        db.record_audit(
            "settings.vacuum",
            "settings",
            None,
            actor_key_id,
            actor_key_name.as_deref(),
            None,
        )
    })
    .await;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "size_before": size_before,
        "size_after": size_after,
        "elapsed_ms": elapsed_ms,
    })))
}

/// Sends a test notification using the currently configured notification channel.
pub(crate) async fn test_notification(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_write()
    {
        return Err(AppError::Forbidden(
            "write access required (admin or operator role)".into(),
        ));
    }
    let result = send_test(&state.db).await;
    match result {
        Ok(msg) => Ok(Json(serde_json::json!({ "status": "ok", "message": msg }))),
        Err(e) => Ok(Json(serde_json::json!({ "status": "error", "message": e }))),
    }
}
