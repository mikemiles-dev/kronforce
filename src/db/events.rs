use chrono::{DateTime, Utc};
use rusqlite::params;
use uuid::Uuid;

use super::Db;
use super::helpers::col;
use crate::db::models::*;
use crate::error::AppError;

impl Db {
    /// Inserts a new event record.
    pub fn insert_event(&self, event: &Event) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        conn.execute(
            "INSERT INTO events (id, kind, severity, message, job_id, agent_id, api_key_id, api_key_name, execution_id, details, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                event.id.to_string(),
                event.kind,
                event.severity.as_str(),
                event.message,
                event.job_id.map(|id| id.to_string()),
                event.agent_id.map(|id| id.to_string()),
                event.api_key_id.map(|id| id.to_string()),
                event.api_key_name,
                event.execution_id.map(|id| id.to_string()),
                event.details,
                event.timestamp.to_rfc3339(),
            ],
        ).map_err(AppError::Db)?;
        Ok(())
    }

    /// Convenience method to log a system event without API key attribution.
    pub fn log_event(
        &self,
        kind: &str,
        severity: EventSeverity,
        message: &str,
        job_id: Option<Uuid>,
        agent_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        self.log_event_full(kind, severity, message, job_id, agent_id, None, None, None)
    }

    /// Logs an audit event attributed to a specific API key.
    pub fn log_audit(
        &self,
        kind: &str,
        message: &str,
        job_id: Option<Uuid>,
        agent_id: Option<Uuid>,
        api_key: &ApiKey,
        details: Option<String>,
    ) -> Result<(), AppError> {
        self.log_event_full(
            kind,
            EventSeverity::Info,
            message,
            job_id,
            agent_id,
            Some(api_key.id),
            Some(api_key.name.clone()),
            details,
        )
    }

    /// Logs an event with all optional fields (API key, details, job/agent IDs).
    #[allow(clippy::too_many_arguments)]
    pub fn log_event_full(
        &self,
        kind: &str,
        severity: EventSeverity,
        message: &str,
        job_id: Option<Uuid>,
        agent_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
        api_key_name: Option<String>,
        details: Option<String>,
    ) -> Result<(), AppError> {
        let event = Event {
            id: Uuid::new_v4(),
            kind: kind.to_string(),
            severity,
            message: message.to_string(),
            job_id,
            agent_id,
            api_key_id,
            api_key_name,
            execution_id: None,
            details,
            timestamp: Utc::now(),
        };
        self.insert_event(&event)
    }

    /// Returns a paginated list of events, optionally filtered by a start timestamp.
    pub fn list_events(
        &self,
        since: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Event>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let sql = match since {
            Some(_) => {
                "SELECT id, kind, severity, message, job_id, agent_id, api_key_id, api_key_name, execution_id, details, timestamp FROM events WHERE timestamp >= ?3 ORDER BY timestamp DESC LIMIT ?1 OFFSET ?2"
            }
            None => {
                "SELECT id, kind, severity, message, job_id, agent_id, api_key_id, api_key_name, execution_id, details, timestamp FROM events ORDER BY timestamp DESC LIMIT ?1 OFFSET ?2"
            }
        };
        let mut stmt = conn.prepare(sql).map_err(AppError::Db)?;
        let rows = stmt
            .query_map(
                match since {
                    Some(s) => rusqlite::params_from_iter(vec![
                        limit.to_string(),
                        offset.to_string(),
                        s.to_string(),
                    ]),
                    None => rusqlite::params_from_iter(vec![limit.to_string(), offset.to_string()]),
                },
                |row| {
                    let id_str: String = col(row, "id")?;
                    let severity_str: String = col(row, "severity")?;
                    let job_id_str: Option<String> = col(row, "job_id")?;
                    let agent_id_str: Option<String> = col(row, "agent_id")?;
                    let api_key_id_str: Option<String> = col(row, "api_key_id")?;
                    let api_key_name: Option<String> = col(row, "api_key_name")?;
                    let execution_id_str: Option<String> = col(row, "execution_id")?;
                    let details: Option<String> = col(row, "details")?;
                    let ts_str: String = col(row, "timestamp")?;
                    Ok(Event {
                        id: Uuid::parse_str(&id_str).map_err(|_| {
                            rusqlite::Error::InvalidColumnType(
                                0,
                                "id".into(),
                                rusqlite::types::Type::Text,
                            )
                        })?,
                        kind: col(row, "kind")?,
                        severity: EventSeverity::from_str(&severity_str)
                            .unwrap_or(EventSeverity::Info),
                        message: col(row, "message")?,
                        job_id: job_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                        agent_id: agent_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                        api_key_id: api_key_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                        api_key_name,
                        execution_id: execution_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                        details,
                        timestamp: DateTime::parse_from_rfc3339(&ts_str)
                            .map_err(|_| {
                                rusqlite::Error::InvalidColumnType(
                                    10,
                                    "timestamp".into(),
                                    rusqlite::types::Type::Text,
                                )
                            })?
                            .with_timezone(&Utc),
                    })
                },
            )
            .map_err(AppError::Db)?;
        let mut events = Vec::new();
        for row in rows {
            events.push(row.map_err(AppError::Db)?);
        }
        Ok(events)
    }

    /// Returns the total number of events, optionally filtered by a start timestamp.
    pub fn count_events(&self, since: Option<&str>) -> Result<u32, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        match since {
            Some(s) => conn.query_row(
                "SELECT COUNT(*) FROM events WHERE timestamp >= ?1",
                params![s],
                |row| row.get(0),
            ),
            None => conn.query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0)),
        }
        .map_err(AppError::Db)
    }
}
