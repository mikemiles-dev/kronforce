use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Reusable query filter builder for dynamic WHERE clauses with parameterized indices.
pub(super) struct QueryFilters {
    pub where_clauses: Vec<String>,
    pub params: Vec<String>,
}

impl QueryFilters {
    /// Creates an empty filter set with no conditions.
    pub fn new() -> Self {
        Self {
            where_clauses: Vec::new(),
            params: Vec::new(),
        }
    }

    /// Adds a `status = ?` filter clause.
    pub fn add_status(&mut self, status: &str) {
        self.add_eq("status", status);
    }

    /// Adds a `column = ?` equality filter.
    pub fn add_eq(&mut self, column: &str, value: &str) {
        self.params.push(value.to_string());
        self.where_clauses
            .push(format!("{} = ?{}", column, self.params.len()));
    }

    /// Adds a `column >= ?` greater-than-or-equal filter.
    pub fn add_gte(&mut self, column: &str, value: &str) {
        self.params.push(value.to_string());
        self.where_clauses
            .push(format!("{} >= ?{}", column, self.params.len()));
    }

    /// Adds a LIKE search across multiple columns joined with OR.
    pub fn add_search(&mut self, query: &str, columns: &[&str]) {
        let like = format!("%{}%", query);
        let conditions: Vec<String> = columns
            .iter()
            .map(|col| {
                self.params.push(like.clone());
                format!("{} LIKE ?{}", col, self.params.len())
            })
            .collect();
        self.where_clauses
            .push(format!("({})", conditions.join(" OR ")));
    }

    /// Adds LIMIT and OFFSET parameters, returning their 1-based parameter indices.
    pub fn add_limit_offset(&mut self, limit: u32, offset: u32) -> (usize, usize) {
        self.params.push(limit.to_string());
        let limit_idx = self.params.len();
        self.params.push(offset.to_string());
        let offset_idx = self.params.len();
        (limit_idx, offset_idx)
    }

    /// Builds the WHERE clause string (including the leading ` WHERE`), or empty if no filters.
    pub fn where_sql(&self) -> String {
        if self.where_clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", self.where_clauses.join(" AND "))
        }
    }

    /// Converts the accumulated parameters into a slice-compatible format for rusqlite.
    pub fn to_params(&self) -> Vec<&dyn rusqlite::types::ToSql> {
        self.params
            .iter()
            .map(|s| s as &dyn rusqlite::types::ToSql)
            .collect()
    }
}

pub(crate) fn parse_uuid(s: &str) -> rusqlite::Result<Uuid> {
    Uuid::parse_str(s).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

pub(crate) fn parse_datetime(s: &str) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })
}

pub(crate) fn parse_json<T: serde::de::DeserializeOwned>(s: &str) -> rusqlite::Result<T> {
    serde_json::from_str(s).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}
