use chrono::{DateTime, Utc};
use rusqlite::params;

use super::Db;
use super::helpers::col;
use crate::db::models::{Connection, ConnectionType};
use crate::error::AppError;

fn parse_connection(row: &rusqlite::Row) -> rusqlite::Result<Connection> {
    let conn_type_str: String = col(row, "conn_type")?;
    let config_raw: String = col(row, "config")?;
    let created_str: String = col(row, "created_at")?;
    let updated_str: String = col(row, "updated_at")?;

    let conn_type: ConnectionType =
        serde_json::from_str(&format!("\"{}\"", conn_type_str)).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(e),
            )
        })?;

    // Decrypt config — propagate errors instead of silently returning empty
    let config_json = crate::crypto::decrypt(&config_raw).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::<dyn std::error::Error + Send + Sync>::from(e),
        )
    })?;
    let config: serde_json::Value = serde_json::from_str(&config_json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(e),
        )
    })?;

    Ok(Connection {
        name: col(row, "name")?,
        conn_type,
        description: col::<Option<String>>(row, "description")?,
        config,
        created_at: DateTime::parse_from_rfc3339(&created_str)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        updated_at: DateTime::parse_from_rfc3339(&updated_str)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
    })
}

impl Db {
    /// Returns all connections ordered by name, with decrypted configs.
    pub fn list_connections(&self) -> Result<Vec<Connection>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let mut stmt = conn
            .prepare(
                "SELECT name, conn_type, description, config, created_at, updated_at FROM connections ORDER BY name",
            )
            .map_err(AppError::Db)?;
        let rows = stmt.query_map([], parse_connection).map_err(AppError::Db)?;
        let mut conns = Vec::new();
        for row in rows {
            conns.push(row.map_err(AppError::Db)?);
        }
        Ok(conns)
    }

    /// Looks up a connection by name.
    pub fn get_connection(&self, name: &str) -> Result<Option<Connection>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let result = conn.query_row(
            "SELECT name, conn_type, description, config, created_at, updated_at FROM connections WHERE name = ?1",
            params![name],
            parse_connection,
        );
        match result {
            Ok(c) => Ok(Some(c)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Db(e)),
        }
    }

    /// Inserts a new connection. The config is encrypted at rest.
    pub fn insert_connection(&self, connection: &Connection) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let config_json = serde_json::to_string(&connection.config)
            .map_err(|e| AppError::Internal(format!("json error: {e}")))?;
        let encrypted = crate::crypto::encrypt(&config_json);
        let conn_type_str = connection.conn_type.to_string();
        conn.execute(
            "INSERT INTO connections (name, conn_type, description, config, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                connection.name,
                conn_type_str,
                connection.description,
                encrypted,
                connection.created_at.to_rfc3339(),
                connection.updated_at.to_rfc3339(),
            ],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    /// Updates an existing connection. Config is re-encrypted.
    pub fn update_connection(&self, name: &str, connection: &Connection) -> Result<bool, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let config_json = serde_json::to_string(&connection.config)
            .map_err(|e| AppError::Internal(format!("json error: {e}")))?;
        let encrypted = crate::crypto::encrypt(&config_json);
        let conn_type_str = connection.conn_type.to_string();
        let now = Utc::now().to_rfc3339();
        let changed = conn
            .execute(
                "UPDATE connections SET conn_type = ?1, description = ?2, config = ?3, updated_at = ?4 WHERE name = ?5",
                params![conn_type_str, connection.description, encrypted, now, name],
            )
            .map_err(AppError::Db)?;
        Ok(changed > 0)
    }

    /// Deletes a connection by name.
    pub fn delete_connection(&self, name: &str) -> Result<bool, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let changed = conn
            .execute("DELETE FROM connections WHERE name = ?1", params![name])
            .map_err(AppError::Db)?;
        Ok(changed > 0)
    }
}
