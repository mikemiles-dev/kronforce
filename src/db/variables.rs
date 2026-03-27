use chrono::{DateTime, Utc};
use rusqlite::params;

use super::Db;
use crate::error::AppError;
use crate::models::Variable;

fn parse_variable(row: &rusqlite::Row) -> rusqlite::Result<Variable> {
    let updated_str: String = row.get(2)?;
    Ok(Variable {
        name: row.get(0)?,
        value: row.get(1)?,
        updated_at: DateTime::parse_from_rfc3339(&updated_str)
            .unwrap()
            .with_timezone(&Utc),
    })
}

impl Db {
    pub fn list_variables(&self) -> Result<Vec<Variable>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT name, value, updated_at FROM variables ORDER BY name")
            .map_err(AppError::Db)?;
        let rows = stmt.query_map([], parse_variable).map_err(AppError::Db)?;
        let mut vars = Vec::new();
        for row in rows {
            vars.push(row.map_err(AppError::Db)?);
        }
        Ok(vars)
    }

    pub fn get_variable(&self, name: &str) -> Result<Option<Variable>, AppError> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT name, value, updated_at FROM variables WHERE name = ?1",
            params![name],
            parse_variable,
        );
        match result {
            Ok(var) => Ok(Some(var)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Db(e)),
        }
    }

    pub fn insert_variable(&self, var: &Variable) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO variables (name, value, updated_at) VALUES (?1, ?2, ?3)",
            params![var.name, var.value, var.updated_at.to_rfc3339()],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    pub fn update_variable(&self, name: &str, value: &str) -> Result<bool, AppError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        let changed = conn
            .execute(
                "UPDATE variables SET value = ?1, updated_at = ?2 WHERE name = ?3",
                params![value, now, name],
            )
            .map_err(AppError::Db)?;
        Ok(changed > 0)
    }

    pub fn delete_variable(&self, name: &str) -> Result<bool, AppError> {
        let conn = self.conn.lock().unwrap();
        let changed = conn
            .execute("DELETE FROM variables WHERE name = ?1", params![name])
            .map_err(AppError::Db)?;
        Ok(changed > 0)
    }

    pub fn upsert_variable(&self, name: &str, value: &str) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO variables (name, value, updated_at) VALUES (?1, ?2, ?3) ON CONFLICT(name) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![name, value, now],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    pub fn get_all_variables_map(
        &self,
    ) -> Result<std::collections::HashMap<String, String>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT name, value FROM variables")
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
}
