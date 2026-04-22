//! Connection resolution: merges named connection credentials into task fields at execution time.

use crate::db::Db;
use crate::db::models::TaskType;
use crate::error::AppError;

/// Resolve connection references in a task. If any `connection` field is set,
/// fetch the connection config and merge its values into the task JSON.
/// Inline fields take precedence over connection fields.
pub fn resolve_connections(task: &TaskType, db: &Db) -> Result<Option<TaskType>, AppError> {
    let json_str =
        serde_json::to_string(task).map_err(|e| AppError::Internal(format!("json: {e}")))?;

    // Quick check: does this task reference any connection?
    if !json_str.contains("\"connection\"") {
        return Ok(None);
    }

    let mut task_json: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| AppError::Internal(format!("json: {e}")))?;

    let conn_name = task_json
        .get("connection")
        .and_then(|v| v.as_str())
        .map(String::from);

    let Some(name) = conn_name else {
        return Ok(None);
    };

    if name.is_empty() {
        return Ok(None);
    }

    let conn = db
        .get_connection(&name)?
        .ok_or_else(|| AppError::BadRequest(format!("connection '{}' not found", name)))?;

    let config = &conn.config;

    // Merge connection config into task JSON.
    // Connection values are defaults — existing non-null task fields are NOT overwritten.
    if let (Some(task_obj), Some(config_obj)) = (task_json.as_object_mut(), config.as_object()) {
        for (key, conn_val) in config_obj {
            // Map connection config field names to task field names
            let task_key = match key.as_str() {
                // SQL connections: host/port/database/username/password → connection_string
                "connection_string" => "connection_string",
                // FTP/SFTP
                "host" => "host",
                "port" => "port",
                "username" => "username",
                "password" => "password",
                // HTTP
                "base_url" => {
                    // Merge base_url into url if url looks like a path
                    if let Some(existing_url) = task_obj.get("url").and_then(|v| v.as_str())
                        && existing_url.starts_with('/')
                        && let Some(base) = conn_val.as_str()
                    {
                        let full = format!("{}{}", base.trim_end_matches('/'), existing_url);
                        task_obj.insert("url".to_string(), serde_json::Value::String(full));
                    }
                    continue;
                }
                // HTTP auth fields — handled after the loop
                "auth_type" | "token" | "header_name" | "header_value" => continue,
                // Kafka/MQTT/RabbitMQ/Redis
                "broker" => "broker",
                "url" => "url",
                "client_id" => "client_id",
                // Everything else: direct mapping
                other => other,
            };

            // Only set if the task doesn't already have a non-null, non-empty value
            let should_set = match task_obj.get(task_key) {
                None => true,
                Some(serde_json::Value::Null) => true,
                Some(serde_json::Value::String(s)) if s.is_empty() => true,
                _ => false,
            };

            if should_set {
                task_obj.insert(task_key.to_string(), conn_val.clone());
            }
        }
        // Inject HTTP auth headers once after all fields are merged
        if config_obj.contains_key("auth_type") {
            inject_http_auth(task_obj, config_obj);
        }
    }

    // Deserialize back to TaskType
    let resolved: TaskType = serde_json::from_value(task_json)
        .map_err(|e| AppError::Internal(format!("failed to resolve connection: {e}")))?;

    Ok(Some(resolved))
}

/// Inject HTTP auth headers from connection config into the task's headers map.
fn inject_http_auth(
    task_obj: &mut serde_json::Map<String, serde_json::Value>,
    config: &serde_json::Map<String, serde_json::Value>,
) {
    let auth_type = config
        .get("auth_type")
        .and_then(|v| v.as_str())
        .unwrap_or("none");

    if auth_type == "none" {
        return;
    }

    // Get or create the headers map
    let headers = task_obj
        .entry("headers")
        .or_insert_with(|| serde_json::json!({}));

    let headers_obj = match headers.as_object_mut() {
        Some(h) => h,
        None => return,
    };

    // Don't overwrite existing Authorization header
    if headers_obj.contains_key("Authorization") || headers_obj.contains_key("authorization") {
        return;
    }

    match auth_type {
        "bearer" => {
            if let Some(token) = config.get("token").and_then(|v| v.as_str()) {
                headers_obj.insert(
                    "Authorization".to_string(),
                    serde_json::Value::String(format!("Bearer {}", token)),
                );
            }
        }
        "basic" => {
            let user = config
                .get("username")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let pass = config
                .get("password")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let encoded = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                format!("{}:{}", user, pass),
            );
            headers_obj.insert(
                "Authorization".to_string(),
                serde_json::Value::String(format!("Basic {}", encoded)),
            );
        }
        "header" => {
            let name = config
                .get("header_name")
                .and_then(|v| v.as_str())
                .unwrap_or("X-API-Key");
            let value = config
                .get("header_value")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            headers_obj.insert(
                name.to_string(),
                serde_json::Value::String(value.to_string()),
            );
        }
        _ => {}
    }
}
