mod agents;
mod events;
mod executions;
mod helpers;
mod jobs;
mod keys;
pub mod models;
mod queue;
mod settings;
mod variables;

use std::sync::{Arc, Mutex};

use chrono::Utc;
use rusqlite::{Connection, params};

use crate::error::AppError;

// Migrations loaded from migrations/*.sql files, embedded at compile time by build.rs
include!(concat!(env!("OUT_DIR"), "/migrations.rs"));

/// SQLite database handle with connection pooling via `Arc<Mutex>`.
#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl Db {
    /// Opens (or creates) the SQLite database at `path` with WAL mode enabled.
    pub fn open(path: &str) -> Result<Self, AppError> {
        let conn = Connection::open(path).map_err(AppError::Db)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(AppError::Db)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Applies all pending schema migrations in order.
    pub fn migrate(&self) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();

        // Schema versioning table
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL,
                description TEXT NOT NULL
            );",
        )
        .map_err(AppError::Db)?;

        let current_version: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        tracing::info!("database schema version: {}", current_version);

        for (version, description, sql) in MIGRATIONS {
            if *version <= current_version {
                continue;
            }
            tracing::info!("applying migration v{}: {}", version, description);
            // Execute each statement separately (ALTER TABLE can't be batched with others that might fail)
            for stmt in sql.split(';') {
                let stmt = stmt.trim();
                if stmt.is_empty() {
                    continue;
                }
                if let Err(e) = conn.execute_batch(stmt) {
                    // Ignore "duplicate column" errors for idempotency
                    let err_str = e.to_string();
                    if err_str.contains("duplicate column") || err_str.contains("already exists") {
                        tracing::debug!(
                            "migration v{}: skipping (already applied): {}",
                            version,
                            err_str
                        );
                    } else {
                        tracing::error!("migration v{} failed: {}", version, e);
                        return Err(AppError::Db(e));
                    }
                }
            }
            conn.execute(
                "INSERT INTO schema_version (version, applied_at, description) VALUES (?1, ?2, ?3)",
                params![version, Utc::now().to_rfc3339(), description],
            )
            .map_err(AppError::Db)?;
            tracing::info!("migration v{} applied", version);
        }

        if current_version < MIGRATIONS.last().map_or(0, |(v, _, _)| *v) {
            tracing::info!(
                "database migrated to v{}",
                MIGRATIONS.last().map_or(0, |(v, _, _)| *v)
            );
        }

        Ok(())
    }
}

impl Db {
    /// Runs the given closure inside a SQLite transaction, committing on success.
    pub fn with_transaction<F, T>(&self, f: F) -> Result<T, AppError>
    where
        F: FnOnce(&rusqlite::Transaction) -> Result<T, AppError>,
    {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().map_err(AppError::Db)?;
        let result = f(&tx)?;
        tx.commit().map_err(AppError::Db)?;
        Ok(result)
    }
}

/// Generic async wrapper for database operations. Handles clone, spawn_blocking, and error mapping.
pub async fn db_call<F, T>(db: &Db, f: F) -> Result<T, AppError>
where
    F: FnOnce(&Db) -> Result<T, AppError> + Send + 'static,
    T: Send + 'static,
{
    let db = db.clone();
    tokio::task::spawn_blocking(move || f(&db))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}
