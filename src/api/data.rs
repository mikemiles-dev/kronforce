use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;

use super::AppState;
use super::auth::AuthUser;
use crate::db::db_call;
use crate::error::AppError;

#[derive(Deserialize)]
pub(crate) struct ExportQuery {
    /// When true, includes decrypted secret variable values, connection configs,
    /// and API key metadata (name, role, permissions — not raw keys or hashes).
    include_secrets: Option<bool>,
}

/// Exports all data for backup/migration. Admin only.
/// With `?include_secrets=true`, includes decrypted secrets and connections.
pub(crate) async fn export_data(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ExportQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_manage_keys()
    {
        return Err(AppError::Forbidden(
            "admin role required for data export".into(),
        ));
    }

    let include_secrets = query.include_secrets.unwrap_or(false);

    let db = state.db.clone();
    let data = tokio::task::spawn_blocking(move || {
        let jobs = db.list_jobs(None, None, None, 10000, 0)?;
        let variables = db.list_variables()?;
        let templates = db.list_templates()?;
        let agents = db.list_agents()?;
        let groups = db.get_distinct_groups()?;
        let settings = db.get_all_settings()?;

        let mut export = serde_json::json!({
            "export_version": 2,
            "exported_at": chrono::Utc::now().to_rfc3339(),
            "jobs": jobs,
            "templates": templates,
            "agents": agents,
            "groups": groups,
            "settings": settings,
        });

        if include_secrets {
            // Variables with decrypted secret values
            export["variables"] = serde_json::to_value(&variables)
                .unwrap_or(serde_json::json!([]));

            // Connections with decrypted configs
            let connections = db.list_connections()?;
            export["connections"] = serde_json::to_value(&connections)
                .unwrap_or(serde_json::json!([]));

            // API keys (metadata only — name, role, permissions, NOT hashes)
            let keys = db.list_api_keys()?;
            let key_meta: Vec<serde_json::Value> = keys
                .iter()
                .map(|k| {
                    serde_json::json!({
                        "name": k.name,
                        "role": k.role,
                        "active": k.active,
                        "allowed_groups": k.allowed_groups,
                        "ip_allowlist": k.ip_allowlist,
                        "expires_at": k.expires_at,
                        "created_at": k.created_at,
                    })
                })
                .collect();
            export["api_keys"] = serde_json::json!(key_meta);
        } else {
            // Mask secret variable values
            let masked: Vec<serde_json::Value> = variables
                .iter()
                .map(|v| {
                    if v.secret {
                        serde_json::json!({
                            "name": v.name,
                            "value": "********",
                            "secret": true,
                            "updated_at": v.updated_at,
                        })
                    } else {
                        serde_json::to_value(v).unwrap_or(serde_json::json!({}))
                    }
                })
                .collect();
            export["variables"] = serde_json::json!(masked);
        }

        Ok::<_, AppError>(export)
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
