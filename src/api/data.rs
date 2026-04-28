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
            export["variables"] = serde_json::to_value(&variables).unwrap_or(serde_json::json!([]));

            // Connections with decrypted configs
            let connections = db.list_connections()?;
            export["connections"] =
                serde_json::to_value(&connections).unwrap_or(serde_json::json!([]));

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

/// Import data from an export JSON. Admin only.
/// Imports jobs, variables, connections, groups, and settings.
/// Skips items that already exist (by name/id) rather than overwriting.
pub(crate) async fn import_data(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_manage_keys()
    {
        return Err(AppError::Forbidden(
            "admin role required for data import".into(),
        ));
    }

    let db = state.db.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut imported = serde_json::Map::new();
        let mut skipped = serde_json::Map::new();

        // Import groups
        if let Some(groups) = payload.get("groups").and_then(|v| v.as_array()) {
            let mut count = 0;
            for g in groups {
                if let Some(name) = g.as_str() {
                    if name != "Default" {
                        let _ = db.add_custom_group(name);
                        count += 1;
                    }
                }
            }
            imported.insert("groups".into(), serde_json::json!(count));
        }

        // Import variables
        if let Some(vars) = payload.get("variables").and_then(|v| v.as_array()) {
            let mut count = 0u32;
            let mut skip = 0u32;
            for v in vars {
                if let Ok(var) = serde_json::from_value::<crate::db::models::Variable>(v.clone()) {
                    if var.value == "********" {
                        skip += 1;
                        continue;
                    }
                    match db.get_variable(&var.name) {
                        Ok(Some(_)) => {
                            skip += 1;
                        }
                        _ => {
                            if db.insert_variable(&var).is_ok() {
                                count += 1;
                            } else {
                                skip += 1;
                            }
                        }
                    }
                }
            }
            imported.insert("variables".into(), serde_json::json!(count));
            if skip > 0 {
                skipped.insert("variables".into(), serde_json::json!(skip));
            }
        }

        // Import connections
        if let Some(conns) = payload.get("connections").and_then(|v| v.as_array()) {
            let mut count = 0u32;
            let mut skip = 0u32;
            for c in conns {
                if let Ok(conn) = serde_json::from_value::<crate::db::models::Connection>(c.clone())
                {
                    match db.get_connection(&conn.name) {
                        Ok(Some(_)) => {
                            skip += 1;
                        }
                        _ => {
                            if db.insert_connection(&conn).is_ok() {
                                count += 1;
                            } else {
                                skip += 1;
                            }
                        }
                    }
                }
            }
            imported.insert("connections".into(), serde_json::json!(count));
            if skip > 0 {
                skipped.insert("connections".into(), serde_json::json!(skip));
            }
        }

        // Import jobs
        if let Some(jobs) = payload.get("jobs").and_then(|v| v.as_array()) {
            let mut count = 0u32;
            let mut skip = 0u32;
            for j in jobs {
                if let Ok(job) = serde_json::from_value::<crate::db::models::Job>(j.clone()) {
                    match db.get_job(job.id) {
                        Ok(Some(_)) => {
                            skip += 1;
                        }
                        _ => {
                            if db.insert_job(&job).is_ok() {
                                count += 1;
                            } else {
                                skip += 1;
                            }
                        }
                    }
                }
            }
            imported.insert("jobs".into(), serde_json::json!(count));
            if skip > 0 {
                skipped.insert("jobs".into(), serde_json::json!(skip));
            }
        }

        // Import settings
        if let Some(settings) = payload.get("settings").and_then(|v| v.as_object()) {
            let mut count = 0u32;
            for (k, v) in settings {
                if let Some(val) = v.as_str() {
                    if db.set_setting(k, val).is_ok() {
                        count += 1;
                    }
                }
            }
            imported.insert("settings".into(), serde_json::json!(count));
        }

        Ok::<_, AppError>(serde_json::json!({
            "status": "ok",
            "imported": imported,
            "skipped": skipped,
        }))
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))??;

    Ok(Json(result))
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
