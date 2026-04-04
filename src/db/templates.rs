use rusqlite::params;

use super::Db;
use crate::error::AppError;

/// A saved job template.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JobTemplate {
    pub name: String,
    pub description: Option<String>,
    pub snapshot: serde_json::Value,
    pub created_by: Option<String>,
    pub created_at: String,
}

impl Db {
    /// Returns all job templates ordered by name.
    pub fn list_templates(&self) -> Result<Vec<JobTemplate>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let mut stmt = conn
            .prepare("SELECT name, description, snapshot_json, created_by, created_at FROM job_templates ORDER BY name")
            .map_err(AppError::Db)?;
        let rows = stmt
            .query_map([], |row| {
                let snapshot_str: String = row.get(2)?;
                Ok(JobTemplate {
                    name: row.get(0)?,
                    description: row.get(1)?,
                    snapshot: serde_json::from_str(&snapshot_str).unwrap_or_default(),
                    created_by: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(AppError::Db)?;
        let mut templates = Vec::new();
        for row in rows {
            templates.push(row.map_err(AppError::Db)?);
        }
        Ok(templates)
    }

    /// Gets a single template by name.
    pub fn get_template(&self, name: &str) -> Result<Option<JobTemplate>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let result = conn.query_row(
            "SELECT name, description, snapshot_json, created_by, created_at FROM job_templates WHERE name = ?1",
            params![name],
            |row| {
                let snapshot_str: String = row.get(2)?;
                Ok(JobTemplate {
                    name: row.get(0)?,
                    description: row.get(1)?,
                    snapshot: serde_json::from_str(&snapshot_str).unwrap_or_default(),
                    created_by: row.get(3)?,
                    created_at: row.get(4)?,
                })
            },
        );
        match result {
            Ok(t) => Ok(Some(t)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Db(e)),
        }
    }

    /// Saves a job template. Replaces if a template with the same name exists.
    pub fn save_template(&self, template: &JobTemplate) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let snapshot_str = serde_json::to_string(&template.snapshot)
            .map_err(|e| AppError::Internal(format!("serialize: {e}")))?;
        conn.execute(
            "INSERT OR REPLACE INTO job_templates (name, description, snapshot_json, created_by, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                template.name,
                template.description,
                snapshot_str,
                template.created_by,
                template.created_at,
            ],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    /// Deletes a template by name.
    pub fn delete_template(&self, name: &str) -> Result<bool, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let changed = conn
            .execute("DELETE FROM job_templates WHERE name = ?1", params![name])
            .map_err(AppError::Db)?;
        Ok(changed > 0)
    }
}
