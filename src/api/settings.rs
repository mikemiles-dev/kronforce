use axum::Json;
use axum::extract::State;

use super::AppState;
use crate::db::db_call;
use crate::error::AppError;
use crate::notifications::send_test;

pub(crate) async fn get_settings(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let settings = db_call(&state.db, move |db| db.get_all_settings()).await?;
    Ok(Json(serde_json::json!(settings)))
}

pub(crate) async fn update_settings(
    State(state): State<AppState>,
    Json(body): Json<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, AppError> {
    db_call(&state.db, move |db| {
        for (key, value) in &body {
            db.set_setting(key, value)?;
        }
        Ok(())
    })
    .await?;
    Ok(Json(serde_json::json!({ "status": "ok" })))
}

pub(crate) async fn test_notification(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = send_test(&state.db).await;
    match result {
        Ok(msg) => Ok(Json(serde_json::json!({ "status": "ok", "message": msg }))),
        Err(e) => Ok(Json(serde_json::json!({ "status": "error", "message": e }))),
    }
}
