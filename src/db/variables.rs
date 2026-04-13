use chrono::{DateTime, Utc};
use rusqlite::params;

use super::Db;
use super::helpers::col;
use crate::db::models::Variable;
use crate::error::AppError;

fn parse_variable(row: &rusqlite::Row) -> rusqlite::Result<Variable> {
    let updated_str: String = col(row, "updated_at")?;
    let secret_int: i32 = col(row, "secret").unwrap_or(0);
    let is_secret = secret_int != 0;
    let raw_value: String = col(row, "value")?;
    // Decrypt secret values that are stored encrypted
    let value = if is_secret {
        crate::crypto::decrypt(&raw_value).unwrap_or(raw_value)
    } else {
        raw_value
    };
    Ok(Variable {
        name: col(row, "name")?,
        value,
        updated_at: DateTime::parse_from_rfc3339(&updated_str)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        secret: is_secret,
    })
}

impl Db {
    /// Returns all global variables ordered by name.
    pub fn list_variables(&self) -> Result<Vec<Variable>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let mut stmt = conn
            .prepare("SELECT name, value, updated_at, secret FROM variables ORDER BY name")
            .map_err(AppError::Db)?;
        let rows = stmt.query_map([], parse_variable).map_err(AppError::Db)?;
        let mut vars = Vec::new();
        for row in rows {
            vars.push(row.map_err(AppError::Db)?);
        }
        Ok(vars)
    }

    /// Looks up a global variable by name.
    pub fn get_variable(&self, name: &str) -> Result<Option<Variable>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let result = conn.query_row(
            "SELECT name, value, updated_at, secret FROM variables WHERE name = ?1",
            params![name],
            parse_variable,
        );
        match result {
            Ok(var) => Ok(Some(var)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Db(e)),
        }
    }

    /// Inserts a new global variable. Secret values are encrypted at rest.
    pub fn insert_variable(&self, var: &Variable) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let stored_value = if var.secret {
            crate::crypto::encrypt(&var.value)
        } else {
            var.value.clone()
        };
        conn.execute(
            "INSERT INTO variables (name, value, updated_at, secret) VALUES (?1, ?2, ?3, ?4)",
            params![
                var.name,
                stored_value,
                var.updated_at.to_rfc3339(),
                var.secret as i32
            ],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    /// Updates a variable's value. Encrypts if the variable is marked secret.
    pub fn update_variable(&self, name: &str, value: &str) -> Result<bool, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        // Check if variable is secret
        let is_secret: bool = conn
            .query_row(
                "SELECT secret FROM variables WHERE name = ?1",
                params![name],
                |row| col::<i32>(row, "secret"),
            )
            .map(|v| v != 0)
            .unwrap_or(false);
        let stored_value = if is_secret {
            crate::crypto::encrypt(value)
        } else {
            value.to_string()
        };
        let now = Utc::now().to_rfc3339();
        let changed = conn
            .execute(
                "UPDATE variables SET value = ?1, updated_at = ?2 WHERE name = ?3",
                params![stored_value, now, name],
            )
            .map_err(AppError::Db)?;
        Ok(changed > 0)
    }

    /// Deletes a variable by name. Returns true if the variable existed.
    pub fn delete_variable(&self, name: &str) -> Result<bool, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let changed = conn
            .execute("DELETE FROM variables WHERE name = ?1", params![name])
            .map_err(AppError::Db)?;
        Ok(changed > 0)
    }

    /// Creates or updates a variable by name.
    pub fn upsert_variable(&self, name: &str, value: &str) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO variables (name, value, updated_at) VALUES (?1, ?2, ?3) ON CONFLICT(name) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![name, value, now],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    /// Returns all variables as a name-to-value map for template substitution.
    pub fn get_all_variables_map(
        &self,
    ) -> Result<std::collections::HashMap<String, String>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let mut stmt = conn
            .prepare("SELECT name, value FROM variables")
            .map_err(AppError::Db)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((col::<String>(row, "name")?, col::<String>(row, "value")?))
            })
            .map_err(AppError::Db)?;
        let mut map = std::collections::HashMap::new();
        for row in rows {
            let (k, v) = row.map_err(AppError::Db)?;
            map.insert(k, v);
        }
        Ok(map)
    }
}
