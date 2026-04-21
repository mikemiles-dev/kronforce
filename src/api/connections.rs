//! Connection management API handlers.

use axum::Json;
use axum::extract::{Path, State};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::AppState;
use super::auth::AuthUser;
use crate::db::db_call;
use crate::db::models::{Connection, ConnectionType, mask_config, merge_config_preserving_secrets};
use crate::error::AppError;

fn require_write(auth: &AuthUser) -> Result<(), AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_write()
    {
        return Err(AppError::Forbidden(
            "write access required (admin or operator role)".into(),
        ));
    }
    Ok(())
}

fn validate_connection_name(name: &str) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::BadRequest(
            "connection name cannot be empty".into(),
        ));
    }
    if name.len() > 100 {
        return Err(AppError::BadRequest(
            "connection name exceeds 100 character limit".into(),
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(AppError::BadRequest(
            "connection name must contain only alphanumeric characters, underscores, and hyphens"
                .into(),
        ));
    }
    Ok(())
}

/// Response with masked sensitive fields.
#[derive(Serialize)]
pub(crate) struct ConnectionResponse {
    name: String,
    conn_type: ConnectionType,
    description: Option<String>,
    config: serde_json::Value,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

fn to_response(conn: Connection) -> ConnectionResponse {
    ConnectionResponse {
        name: conn.name,
        conn_type: conn.conn_type,
        description: conn.description,
        config: mask_config(&conn.config),
        created_at: conn.created_at,
        updated_at: conn.updated_at,
    }
}

/// Returns all connections with masked sensitive fields.
pub(crate) async fn list_connections(
    State(state): State<AppState>,
) -> Result<Json<Vec<ConnectionResponse>>, AppError> {
    let conns = db_call(&state.db, move |db| db.list_connections()).await?;
    Ok(Json(conns.into_iter().map(to_response).collect()))
}

/// Returns a single connection by name with masked sensitive fields.
pub(crate) async fn get_connection(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ConnectionResponse>, AppError> {
    let conn = db_call(&state.db, move |db| db.get_connection(&name))
        .await?
        .ok_or_else(|| AppError::NotFound("connection not found".into()))?;
    Ok(Json(to_response(conn)))
}

#[derive(Deserialize)]
pub(crate) struct CreateConnectionRequest {
    name: String,
    conn_type: ConnectionType,
    description: Option<String>,
    config: serde_json::Value,
}

/// Creates a new connection.
pub(crate) async fn create_connection(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateConnectionRequest>,
) -> Result<(axum::http::StatusCode, Json<ConnectionResponse>), AppError> {
    require_write(&auth)?;
    validate_connection_name(&req.name)?;

    let now = Utc::now();
    let conn = Connection {
        name: req.name,
        conn_type: req.conn_type,
        description: req.description,
        config: req.config,
        created_at: now,
        updated_at: now,
    };

    let conn_clone = conn.clone();
    db_call(&state.db, move |db| db.insert_connection(&conn_clone)).await?;

    let actor_id = auth.0.as_ref().map(|k| k.id);
    let actor_name = auth.0.as_ref().map(|k| k.name.clone());
    let audit_name = conn.name.clone();
    let _ = db_call(&state.db, move |db| {
        db.record_audit(
            "connection.created",
            "connection",
            Some(&audit_name),
            actor_id,
            actor_name.as_deref(),
            None,
        )
    })
    .await;

    Ok((axum::http::StatusCode::CREATED, Json(to_response(conn))))
}

#[derive(Deserialize)]
pub(crate) struct UpdateConnectionRequest {
    conn_type: Option<ConnectionType>,
    description: Option<String>,
    config: Option<serde_json::Value>,
}

/// Updates an existing connection, preserving masked sensitive values.
pub(crate) async fn update_connection(
    State(state): State<AppState>,
    Path(name): Path<String>,
    auth: AuthUser,
    Json(req): Json<UpdateConnectionRequest>,
) -> Result<Json<ConnectionResponse>, AppError> {
    require_write(&auth)?;

    let name_clone = name.clone();
    let existing = db_call(&state.db, move |db| db.get_connection(&name_clone))
        .await?
        .ok_or_else(|| AppError::NotFound("connection not found".into()))?;

    let config = match req.config {
        Some(new_config) => merge_config_preserving_secrets(&existing.config, &new_config),
        None => existing.config.clone(),
    };

    let updated = Connection {
        name: existing.name.clone(),
        conn_type: req.conn_type.unwrap_or(existing.conn_type),
        description: req.description.or(existing.description),
        config,
        created_at: existing.created_at,
        updated_at: Utc::now(),
    };

    let name_for_db = name.clone();
    let updated_clone = updated.clone();
    db_call(&state.db, move |db| {
        db.update_connection(&name_for_db, &updated_clone)
    })
    .await?;

    let actor_id = auth.0.as_ref().map(|k| k.id);
    let actor_name = auth.0.as_ref().map(|k| k.name.clone());
    let _ = db_call(&state.db, move |db| {
        db.record_audit(
            "connection.updated",
            "connection",
            Some(&name),
            actor_id,
            actor_name.as_deref(),
            None,
        )
    })
    .await;

    Ok(Json(to_response(updated)))
}

/// Deletes a connection by name.
pub(crate) async fn delete_connection(
    State(state): State<AppState>,
    Path(name): Path<String>,
    auth: AuthUser,
) -> Result<axum::http::StatusCode, AppError> {
    require_write(&auth)?;
    let name_clone = name.clone();
    let deleted = db_call(&state.db, move |db| db.delete_connection(&name_clone)).await?;
    if !deleted {
        return Err(AppError::NotFound("connection not found".into()));
    }

    let actor_id = auth.0.as_ref().map(|k| k.id);
    let actor_name = auth.0.as_ref().map(|k| k.name.clone());
    let _ = db_call(&state.db, move |db| {
        db.record_audit(
            "connection.deleted",
            "connection",
            Some(&name),
            actor_id,
            actor_name.as_deref(),
            None,
        )
    })
    .await;

    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
pub(crate) struct TestResult {
    success: bool,
    message: String,
}

/// Test a connection's connectivity.
pub(crate) async fn test_connection(
    State(state): State<AppState>,
    Path(name): Path<String>,
    auth: AuthUser,
) -> Result<Json<TestResult>, AppError> {
    require_write(&auth)?;

    let conn = db_call(&state.db, move |db| db.get_connection(&name))
        .await?
        .ok_or_else(|| AppError::NotFound("connection not found".into()))?;

    let result = test_connectivity(&conn).await;

    Ok(Json(result))
}

async fn test_connectivity(conn: &Connection) -> TestResult {
    let config = &conn.config;

    match conn.conn_type {
        ConnectionType::Postgres | ConnectionType::Mysql => {
            let conn_str = config
                .get("connection_string")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if conn_str.is_empty() {
                return TestResult {
                    success: false,
                    message: "connection_string is empty".into(),
                };
            }
            // Try a simple query via CLI
            let cmd = if conn.conn_type == ConnectionType::Postgres {
                format!("psql '{}' -c 'SELECT 1' 2>&1 | head -5", conn_str)
            } else {
                format!("mysql '{}' -e 'SELECT 1' 2>&1 | head -5", conn_str)
            };
            match tokio::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .output()
                .await
            {
                Ok(out) => {
                    if out.status.success() {
                        TestResult {
                            success: true,
                            message: "Connection successful".into(),
                        }
                    } else {
                        TestResult {
                            success: false,
                            message: String::from_utf8_lossy(&out.stderr).trim().to_string(),
                        }
                    }
                }
                Err(e) => TestResult {
                    success: false,
                    message: format!("failed to execute: {e}"),
                },
            }
        }
        ConnectionType::Http => {
            let url = config
                .get("base_url")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if url.is_empty() {
                return TestResult {
                    success: false,
                    message: "base_url is empty".into(),
                };
            }
            match reqwest::Client::new()
                .head(url)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
            {
                Ok(resp) => TestResult {
                    success: resp.status().is_success() || resp.status().is_redirection(),
                    message: format!(
                        "HTTP {} {}",
                        resp.status().as_u16(),
                        resp.status().canonical_reason().unwrap_or("")
                    ),
                },
                Err(e) => TestResult {
                    success: false,
                    message: format!("request failed: {e}"),
                },
            }
        }
        ConnectionType::Redis => {
            let url = config
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("redis://localhost:6379");
            let cmd = format!("redis-cli -u '{}' PING 2>&1 | head -1", url);
            match tokio::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .output()
                .await
            {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    TestResult {
                        success: stdout.trim() == "PONG",
                        message: stdout.trim().to_string(),
                    }
                }
                Err(e) => TestResult {
                    success: false,
                    message: format!("failed to execute: {e}"),
                },
            }
        }
        _ => {
            // For types without a quick test, try TCP connect to host:port
            let host = config
                .get("host")
                .or_else(|| config.get("broker"))
                .and_then(|v| v.as_str())
                .unwrap_or("localhost");
            let port = config.get("port").and_then(|v| v.as_u64()).unwrap_or(0);

            if port == 0 {
                return TestResult {
                    success: false,
                    message: "no host/port configured for TCP test".into(),
                };
            }

            let addr = format!("{}:{}", host, port);
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                tokio::net::TcpStream::connect(&addr),
            )
            .await
            {
                Ok(Ok(_)) => TestResult {
                    success: true,
                    message: format!("TCP connection to {} successful", addr),
                },
                Ok(Err(e)) => TestResult {
                    success: false,
                    message: format!("TCP connection failed: {e}"),
                },
                Err(_) => TestResult {
                    success: false,
                    message: format!("TCP connection to {} timed out", addr),
                },
            }
        }
    }
}
