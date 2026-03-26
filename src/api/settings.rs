use axum::extract::State;
use axum::Json;

use super::AppState;
use crate::error::AppError;

pub(crate) async fn get_settings(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = state.db.clone();
    let settings = tokio::task::spawn_blocking(move || db.get_all_settings())
        .await
        .unwrap()?;
    Ok(Json(serde_json::json!(settings)))
}

pub(crate) async fn update_settings(
    State(state): State<AppState>,
    Json(body): Json<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        for (key, value) in &body {
            db.set_setting(key, value)?;
        }
        Ok::<(), AppError>(())
    })
    .await
    .unwrap()?;
    Ok(Json(serde_json::json!({ "status": "ok" })))
}

pub(crate) async fn test_notification(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = crate::notifications::send_test(&state.db).await;
    match result {
        Ok(msg) => Ok(Json(serde_json::json!({ "status": "ok", "message": msg }))),
        Err(e) => Ok(Json(serde_json::json!({ "status": "error", "message": e }))),
    }
}
