use chrono::Utc;
use rusqlite::params;

use super::Db;
use crate::error::AppError;

impl Db {
    /// Returns the value of a setting by key, or None if not set.
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let result = conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        );
        match result {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Db(e)),
        }
    }

    /// Creates or updates a setting by key.
    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        ).map_err(AppError::Db)?;
        Ok(())
    }

    /// Returns all settings as a key-value map.
    pub fn get_all_settings(&self) -> Result<std::collections::HashMap<String, String>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let mut stmt = conn
            .prepare("SELECT key, value FROM settings")
            .map_err(AppError::Db)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(AppError::Db)?;
        let mut map = std::collections::HashMap::new();
        for row in rows {
            let (k, v) = row.map_err(AppError::Db)?;
            map.insert(k, v);
        }
        Ok(map)
    }

    /// Deletes finished executions older than the specified retention period.
    pub fn purge_old_executions(&self, retention_days: i64) -> Result<u32, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let cutoff = (Utc::now() - chrono::Duration::days(retention_days)).to_rfc3339();
        let deleted = conn
            .execute(
                "DELETE FROM executions WHERE finished_at IS NOT NULL AND finished_at < ?1",
                params![cutoff],
            )
            .map_err(AppError::Db)?;
        Ok(deleted as u32)
    }

    /// Deletes events older than the specified retention period.
    pub fn purge_old_events(&self, retention_days: i64) -> Result<u32, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let cutoff = (Utc::now() - chrono::Duration::days(retention_days)).to_rfc3339();
        let deleted = conn
            .execute("DELETE FROM events WHERE timestamp < ?1", params![cutoff])
            .map_err(AppError::Db)?;
        Ok(deleted as u32)
    }

    /// Deletes completed queue items older than the specified retention period.
    pub fn purge_old_queue_items(&self, retention_days: i64) -> Result<u32, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let cutoff = (Utc::now() - chrono::Duration::days(retention_days)).to_rfc3339();
        let deleted = conn
            .execute(
                "DELETE FROM job_queue WHERE status = 'completed' AND created_at < ?1",
                params![cutoff],
            )
            .map_err(AppError::Db)?;
        Ok(deleted as u32)
    }
}
