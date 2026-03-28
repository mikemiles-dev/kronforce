use chrono::{DateTime, Utc};
use rusqlite::params;
use uuid::Uuid;

use super::Db;
use super::helpers::*;
use crate::error::AppError;
use crate::models::*;

impl Db {
    pub fn insert_api_key(&self, key: &ApiKey) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO api_keys (id, key_prefix, key_hash, name, role, created_at, last_used_at, active) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                key.id.to_string(),
                key.key_prefix,
                key.key_hash,
                key.name,
                key.role.as_str(),
                key.created_at.to_rfc3339(),
                key.last_used_at.map(|t| t.to_rfc3339()),
                key.active as i32,
            ],
        ).map_err(AppError::Db)?;
        Ok(())
    }

    pub fn get_api_key_by_hash(&self, hash: &str) -> Result<Option<ApiKey>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, key_prefix, key_hash, name, role, created_at, last_used_at, active FROM api_keys WHERE key_hash = ?1 AND active = 1")
            .map_err(AppError::Db)?;
        let mut rows = stmt
            .query_map(params![hash], row_to_api_key)
            .map_err(AppError::Db)?;
        match rows.next() {
            Some(Ok(key)) => Ok(Some(key)),
            Some(Err(e)) => Err(AppError::Db(e)),
            None => Ok(None),
        }
    }

    pub fn list_api_keys(&self) -> Result<Vec<ApiKey>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, key_prefix, key_hash, name, role, created_at, last_used_at, active FROM api_keys ORDER BY created_at DESC")
            .map_err(AppError::Db)?;
        let rows = stmt.query_map([], row_to_api_key).map_err(AppError::Db)?;
        let mut keys = Vec::new();
        for row in rows {
            keys.push(row.map_err(AppError::Db)?);
        }
        Ok(keys)
    }

    pub fn update_api_key_last_used(&self, id: Uuid, at: DateTime<Utc>) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE api_keys SET last_used_at = ?1 WHERE id = ?2",
            params![at.to_rfc3339(), id.to_string()],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    pub fn count_api_keys(&self) -> Result<u32, AppError> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM api_keys WHERE active = 1",
            [],
            |row| row.get(0),
        )
        .map_err(AppError::Db)
    }

    pub fn delete_api_key(&self, id: Uuid) -> Result<(), AppError> {
        self.revoke_api_key(id)
    }

    pub fn revoke_api_key(&self, id: Uuid) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE api_keys SET active = 0 WHERE id = ?1",
            params![id.to_string()],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }
}
