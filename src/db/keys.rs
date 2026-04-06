use chrono::{DateTime, Utc};
use rusqlite::params;
use uuid::Uuid;

use super::Db;
use crate::db::models::*;
use crate::error::AppError;

impl Db {
    /// Inserts a new API key record.
    pub fn insert_api_key(&self, key: &ApiKey) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        conn.execute(
            "INSERT INTO api_keys (id, key_prefix, key_hash, name, role, created_at, last_used_at, active, allowed_groups_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                key.id.to_string(),
                key.key_prefix,
                key.key_hash,
                key.name,
                key.role.as_str(),
                key.created_at.to_rfc3339(),
                key.last_used_at.map(|t| t.to_rfc3339()),
                key.active as i32,
                key.allowed_groups.as_ref().map(|g| serde_json::to_string(g).unwrap_or_default()),
            ],
        ).map_err(AppError::Db)?;
        Ok(())
    }

    /// Looks up an active API key by its SHA-256 hash.
    pub fn get_api_key_by_hash(&self, hash: &str) -> Result<Option<ApiKey>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let mut stmt = conn
            .prepare("SELECT id, key_prefix, key_hash, name, role, created_at, last_used_at, active, allowed_groups_json, ip_allowlist FROM api_keys WHERE key_hash = ?1 AND active = 1")
            .map_err(AppError::Db)?;
        let mut rows = stmt
            .query_map(params![hash], ApiKey::from_row)
            .map_err(AppError::Db)?;
        match rows.next() {
            Some(Ok(key)) => Ok(Some(key)),
            Some(Err(e)) => Err(AppError::Db(e)),
            None => Ok(None),
        }
    }

    /// Returns all API keys (both active and revoked), newest first.
    pub fn list_api_keys(&self) -> Result<Vec<ApiKey>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let mut stmt = conn
            .prepare("SELECT id, key_prefix, key_hash, name, role, created_at, last_used_at, active, allowed_groups_json, ip_allowlist FROM api_keys ORDER BY created_at DESC")
            .map_err(AppError::Db)?;
        let rows = stmt.query_map([], ApiKey::from_row).map_err(AppError::Db)?;
        let mut keys = Vec::new();
        for row in rows {
            keys.push(row.map_err(AppError::Db)?);
        }
        Ok(keys)
    }

    /// Updates the last-used timestamp for an API key.
    pub fn update_api_key_last_used(&self, id: Uuid, at: DateTime<Utc>) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        conn.execute(
            "UPDATE api_keys SET last_used_at = ?1 WHERE id = ?2",
            params![at.to_rfc3339(), id.to_string()],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    /// Returns the number of active API keys.
    pub fn count_api_keys(&self) -> Result<u32, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        conn.query_row(
            "SELECT COUNT(*) FROM api_keys WHERE active = 1",
            [],
            |row| row.get(0),
        )
        .map_err(AppError::Db)
    }

    /// Deletes (revokes) an API key. Alias for `revoke_api_key`.
    pub fn delete_api_key(&self, id: Uuid) -> Result<(), AppError> {
        self.revoke_api_key(id)
    }

    /// Soft-deletes an API key by setting it inactive.
    pub fn revoke_api_key(&self, id: Uuid) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        conn.execute(
            "UPDATE api_keys SET active = 0 WHERE id = ?1",
            params![id.to_string()],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }
}
