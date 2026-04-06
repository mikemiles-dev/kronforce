//! SQLite database layer.
//!
//! Provides the `Db` handle and query methods for jobs, executions, agents,
//! events, API keys, settings, variables, and the agent job queue.

mod agents;
pub mod audit;
mod events;
mod executions;
mod helpers;
mod jobs;
mod keys;
pub mod models;
mod queue;
mod sessions;
mod settings;
pub mod templates;
mod variables;

use chrono::Utc;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;

use tracing::{debug, error, info};

use crate::error::AppError;

// Migrations loaded from migrations/*.sql files, embedded at compile time by build.rs
include!(concat!(env!("OUT_DIR"), "/migrations.rs"));

/// SQLite database handle backed by an r2d2 connection pool.
#[derive(Clone)]
pub struct Db {
    pool: Pool<SqliteConnectionManager>,
}

impl Db {
    /// Opens (or creates) the SQLite database at `path` with a connection pool.
    /// WAL mode, foreign keys, and busy_timeout are set on every connection.
    /// For `:memory:` databases (tests), pool size is forced to 1.
    pub fn open(path: &str) -> Result<Self, AppError> {
        Self::open_with_pool_size(path, if path == ":memory:" { 1 } else { 8 }, 5)
    }

    /// Opens the database with explicit pool configuration.
    pub fn open_with_pool_size(
        path: &str,
        pool_size: u32,
        timeout_secs: u64,
    ) -> Result<Self, AppError> {
        let manager = SqliteConnectionManager::file(path).with_init(|c| {
            c.execute_batch(
                "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;",
            )
        });
        let pool = Pool::builder()
            .max_size(if path == ":memory:" { 1 } else { pool_size })
            .connection_timeout(std::time::Duration::from_secs(timeout_secs))
            .build(manager)
            .map_err(|e| AppError::Internal(format!("failed to create connection pool: {e}")))?;
        Ok(Self { pool })
    }

    /// Applies all pending schema migrations in order.
    pub fn migrate(&self) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;

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

        info!("database schema version: {}", current_version);

        for (version, description, sql) in MIGRATIONS {
            if *version <= current_version {
                continue;
            }
            info!("applying migration v{}: {}", version, description);
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
                        debug!(
                            "migration v{}: skipping (already applied): {}",
                            version, err_str
                        );
                    } else {
                        error!("migration v{} failed: {}", version, e);
                        return Err(AppError::Db(e));
                    }
                }
            }
            conn.execute(
                "INSERT INTO schema_version (version, applied_at, description) VALUES (?1, ?2, ?3)",
                params![version, Utc::now().to_rfc3339(), description],
            )
            .map_err(AppError::Db)?;
            info!("migration v{} applied", version);
        }

        if current_version < MIGRATIONS.last().map_or(0, |(v, _, _)| *v) {
            info!(
                "database migrated to v{}",
                MIGRATIONS.last().map_or(0, |(v, _, _)| *v)
            );
        }

        Ok(())
    }
}

impl Db {
    /// Returns health information about the database.
    pub fn health_check(&self) -> Option<crate::api::DbHealth> {
        let conn = self.pool.get().ok()?;
        // Verify DB is accessible
        let ok = conn.query_row("SELECT 1", [], |_| Ok(())).is_ok();

        // Get DB file size
        let db_path: Option<String> = conn
            .query_row("PRAGMA database_list", [], |row| row.get(2))
            .ok();
        let size_bytes = db_path
            .as_ref()
            .and_then(|p| std::fs::metadata(p).ok())
            .map(|m| m.len());
        let wal_size_bytes = db_path
            .as_ref()
            .and_then(|p| std::fs::metadata(format!("{p}-wal")).ok())
            .map(|m| m.len());

        Some(crate::api::DbHealth {
            ok,
            size_bytes,
            wal_size_bytes,
            pool_size: self.pool.max_size(),
        })
    }

    /// Deletes all user data (jobs, executions, variables, templates, events, etc.).
    pub fn delete_all_data(&self) -> Result<serde_json::Value, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let jobs: i64 = conn
            .query_row("SELECT COUNT(*) FROM jobs", [], |r| r.get(0))
            .unwrap_or(0);
        conn.execute_batch(
            "DELETE FROM executions;
             DELETE FROM jobs;
             DELETE FROM variables;
             DELETE FROM job_templates;
             DELETE FROM events;
             DELETE FROM audit_log;
             DELETE FROM oidc_sessions;
             DELETE FROM oidc_auth_states;
             DELETE FROM job_versions;",
        )
        .map_err(AppError::Db)?;
        Ok(serde_json::json!({"deleted": true, "jobs_removed": jobs}))
    }

    /// Forces a WAL checkpoint, flushing all WAL data to the main database file.
    pub fn checkpoint(&self) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(AppError::Db)?;
        Ok(())
    }

    /// Runs the given closure inside a SQLite transaction, committing on success.
    pub fn with_transaction<F, T>(&self, f: F) -> Result<T, AppError>
    where
        F: FnOnce(&rusqlite::Transaction) -> Result<T, AppError>,
    {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
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
