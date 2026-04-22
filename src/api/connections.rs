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
        ConnectionType::Sqlite => {
            let path = config
                .get("connection_string")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if path.is_empty() {
                return TestResult {
                    success: false,
                    message: "database path is empty".into(),
                };
            }
            if std::path::Path::new(path).exists() {
                TestResult {
                    success: true,
                    message: format!("file exists: {}", path),
                }
            } else {
                TestResult {
                    success: false,
                    message: format!("file not found: {}", path),
                }
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
        _ => {
            // For all other types: extract host:port and do a TCP connect test.
            // This works for Postgres, MySQL, Redis, Kafka, MQTT, RabbitMQ, FTP, SFTP, SSH, SMTP, S3.
            let (host, port) = extract_host_port(config, conn.conn_type);

            if host.is_empty() {
                return TestResult {
                    success: false,
                    message: "no host configured — check connection settings".into(),
                };
            }
            if port == 0 {
                return TestResult {
                    success: false,
                    message: format!("no port configured for host '{}'", host),
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
                    message: format!("connection to {} failed: {e}", addr),
                },
                Err(_) => TestResult {
                    success: false,
                    message: format!("connection to {} timed out (5s)", addr),
                },
            }
        }
    }
}

/// Extract host and port from connection config, parsing URLs when necessary.
fn extract_host_port(config: &serde_json::Value, conn_type: ConnectionType) -> (String, u16) {
    // Try explicit host/port fields first
    let explicit_host = config
        .get("host")
        .or_else(|| config.get("broker"))
        .and_then(|v| v.as_str());
    let explicit_port = config
        .get("port")
        .and_then(|v| v.as_u64())
        .map(|p| p as u16);

    if let Some(h) = explicit_host {
        let default_port = default_port_for_type(conn_type);
        return (h.to_string(), explicit_port.unwrap_or(default_port));
    }

    // Try parsing from connection_string or url fields
    let url_str = config
        .get("connection_string")
        .or_else(|| config.get("url"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if url_str.is_empty() {
        return (String::new(), 0);
    }

    // Try parsing as a URL (handles postgres://, redis://, amqp://, etc.)
    if let Ok(parsed) = url::Url::parse(url_str) {
        let host = parsed.host_str().unwrap_or("").to_string();
        let port = parsed
            .port()
            .unwrap_or_else(|| default_port_for_type(conn_type));
        return (host, port);
    }

    // Try host:port format (e.g., Kafka broker "localhost:9092")
    if let Some((h, p)) = url_str.split_once(':')
        && let Ok(port) = p.parse::<u16>()
    {
        return (h.to_string(), port);
    }

    (url_str.to_string(), default_port_for_type(conn_type))
}

fn default_port_for_type(conn_type: ConnectionType) -> u16 {
    match conn_type {
        ConnectionType::Postgres => 5432,
        ConnectionType::Mysql => 3306,
        ConnectionType::Redis => 6379,
        ConnectionType::Kafka => 9092,
        ConnectionType::Mqtt => 1883,
        ConnectionType::Rabbitmq => 5672,
        ConnectionType::Ftp => 21,
        ConnectionType::Sftp | ConnectionType::Ssh => 22,
        ConnectionType::Smtp => 587,
        ConnectionType::Mongodb => 27017,
        ConnectionType::S3 | ConnectionType::Http => 443,
        ConnectionType::Sqlite => 0, // No network
    }
}
