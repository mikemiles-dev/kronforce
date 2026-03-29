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

/// Sends a test notification using the currently configured notification channel.
pub(crate) async fn test_notification(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = send_test(&state.db).await;
    match result {
        Ok(msg) => Ok(Json(serde_json::json!({ "status": "ok", "message": msg }))),
        Err(e) => Ok(Json(serde_json::json!({ "status": "error", "message": e }))),
    }
}
