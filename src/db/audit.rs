use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

use super::Db;
use super::helpers::*;
use crate::error::AppError;

use serde::Serialize;

/// A single audit log entry.
#[derive(Debug, Clone, Serialize)]
pub struct AuditEntry {
    pub id: Uuid,
    pub timestamp: chrono::DateTime<Utc>,
    pub actor_key_id: Option<Uuid>,
    pub actor_key_name: Option<String>,
    pub operation: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub details: Option<String>,
}

impl Db {
    /// Records an audit log entry for a sensitive operation.
    #[allow(clippy::too_many_arguments)]
    pub fn record_audit(
        &self,
        operation: &str,
        resource_type: &str,
        resource_id: Option<&str>,
        actor_key_id: Option<Uuid>,
        actor_key_name: Option<&str>,
        details: Option<&str>,
    ) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        conn.execute(
            "INSERT INTO audit_log (id, timestamp, actor_key_id, actor_key_name, operation, resource_type, resource_id, details) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                Uuid::new_v4().to_string(),
                Utc::now().to_rfc3339(),
                actor_key_id.map(|id| id.to_string()),
                actor_key_name,
                operation,
                resource_type,
                resource_id,
                details,
            ],
        ).map_err(AppError::Db)?;
        Ok(())
    }

    /// Returns a paginated list of audit log entries, newest first, with optional filters.
    pub fn list_audit_log(
        &self,
        operation: Option<&str>,
        actor: Option<&str>,
        since: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AuditEntry>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let mut f = QueryFilters::new();
        if let Some(op) = operation {
            f.add_eq("operation", op);
        }
        if let Some(a) = actor {
            f.add_search(a, &["actor_key_name"]);
        }
        if let Some(s) = since {
            f.add_gte("timestamp", s);
        }
        let (li, oi) = f.add_limit_offset(limit, offset);
        let sql = format!(
            "SELECT id, timestamp, actor_key_id, actor_key_name, operation, resource_type, resource_id, details FROM audit_log{} ORDER BY timestamp DESC LIMIT ?{} OFFSET ?{}",
            f.where_sql(),
            li,
            oi
        );
        let mut stmt = conn.prepare(&sql).map_err(AppError::Db)?;
        let rows = stmt
            .query_map(f.to_params().as_slice(), |row| {
                let id_str: String = row.get(0)?;
                let ts_str: String = row.get(1)?;
                let actor_id_str: Option<String> = row.get(2)?;
                Ok(AuditEntry {
                    id: parse_uuid(&id_str)?,
                    timestamp: parse_datetime(&ts_str)?,
                    actor_key_id: actor_id_str.map(|s| parse_uuid(&s)).transpose()?,
                    actor_key_name: row.get(3)?,
                    operation: row.get(4)?,
                    resource_type: row.get(5)?,
                    resource_id: row.get(6)?,
                    details: row.get(7)?,
                })
            })
            .map_err(AppError::Db)?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(AppError::Db)?);
        }
        Ok(entries)
    }

    /// Returns the total count of audit log entries matching the given filters.
    pub fn count_audit_log(
        &self,
        operation: Option<&str>,
        actor: Option<&str>,
        since: Option<&str>,
    ) -> Result<u32, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let mut f = QueryFilters::new();
        if let Some(op) = operation {
            f.add_eq("operation", op);
        }
        if let Some(a) = actor {
            f.add_search(a, &["actor_key_name"]);
        }
        if let Some(s) = since {
            f.add_gte("timestamp", s);
        }
        let sql = format!("SELECT COUNT(*) FROM audit_log{}", f.where_sql());
        let mut stmt = conn.prepare(&sql).map_err(AppError::Db)?;
        stmt.query_row(f.to_params().as_slice(), |row| row.get(0))
            .map_err(AppError::Db)
    }

    /// Deletes audit log entries older than the given number of days.
    pub fn purge_old_audit_log(&self, days: i64) -> Result<u32, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let cutoff = (Utc::now() - chrono::Duration::days(days)).to_rfc3339();
        let count = conn
            .execute(
                "DELETE FROM audit_log WHERE timestamp < ?1",
                params![cutoff],
            )
            .map_err(AppError::Db)?;
        Ok(count as u32)
    }
}
