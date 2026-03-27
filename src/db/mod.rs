mod agents;
mod events;
mod executions;
mod helpers;
mod jobs;
mod keys;
mod queue;
mod settings;
mod variables;

use std::sync::{Arc, Mutex};

use chrono::Utc;
use rusqlite::{Connection, params};

use crate::error::AppError;

#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl Db {
    pub fn open(path: &str) -> Result<Self, AppError> {
        let conn = Connection::open(path).map_err(AppError::Db)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(AppError::Db)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

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

        let migrations: Vec<(i64, &str, &str)> = vec![
            (1, "Initial schema: jobs, executions, agents", "
                CREATE TABLE IF NOT EXISTS jobs (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL UNIQUE,
                    description TEXT,
                    command TEXT,
                    schedule_json TEXT NOT NULL,
                    status TEXT NOT NULL DEFAULT 'scheduled',
                    timeout_secs INTEGER,
                    depends_on_json TEXT NOT NULL DEFAULT '[]',
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS executions (
                    id TEXT PRIMARY KEY,
                    job_id TEXT NOT NULL REFERENCES jobs(id),
                    status TEXT NOT NULL,
                    exit_code INTEGER,
                    stdout TEXT NOT NULL DEFAULT '',
                    stderr TEXT NOT NULL DEFAULT '',
                    stdout_truncated INTEGER NOT NULL DEFAULT 0,
                    stderr_truncated INTEGER NOT NULL DEFAULT 0,
                    started_at TEXT,
                    finished_at TEXT,
                    triggered_by_json TEXT NOT NULL,
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                );
                CREATE INDEX IF NOT EXISTS idx_executions_job_id ON executions(job_id);
                CREATE INDEX IF NOT EXISTS idx_executions_status ON executions(status);
                CREATE INDEX IF NOT EXISTS idx_executions_started_at ON executions(started_at);
                CREATE TABLE IF NOT EXISTS agents (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL UNIQUE,
                    tags_json TEXT NOT NULL DEFAULT '[]',
                    hostname TEXT NOT NULL,
                    address TEXT NOT NULL,
                    port INTEGER NOT NULL,
                    status TEXT NOT NULL DEFAULT 'online',
                    last_heartbeat TEXT,
                    registered_at TEXT NOT NULL
                );
            "),
            (2, "Add job targeting and agent dispatch", "
                ALTER TABLE jobs ADD COLUMN target_json TEXT;
                ALTER TABLE executions ADD COLUMN agent_id TEXT;
            "),
            (3, "Migrate status names", "
                UPDATE jobs SET status = 'scheduled' WHERE status IN ('active', 'enabled');
                UPDATE jobs SET status = 'paused' WHERE status = 'disabled';
                UPDATE jobs SET status = 'unscheduled' WHERE status = 'completed';
            "),
            (4, "Add events and API keys", "
                CREATE TABLE IF NOT EXISTS events (
                    id TEXT PRIMARY KEY,
                    kind TEXT NOT NULL,
                    severity TEXT NOT NULL DEFAULT 'info',
                    message TEXT NOT NULL,
                    job_id TEXT,
                    agent_id TEXT,
                    timestamp TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
                CREATE TABLE IF NOT EXISTS api_keys (
                    id TEXT PRIMARY KEY,
                    key_prefix TEXT NOT NULL,
                    key_hash TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    role TEXT NOT NULL DEFAULT 'viewer',
                    created_at TEXT NOT NULL,
                    last_used_at TEXT,
                    active INTEGER NOT NULL DEFAULT 1
                );
            "),
            (5, "Add run_as, audit fields, task snapshots", "
                ALTER TABLE jobs ADD COLUMN run_as TEXT;
                ALTER TABLE jobs ADD COLUMN created_by TEXT;
                ALTER TABLE events ADD COLUMN api_key_id TEXT;
                ALTER TABLE events ADD COLUMN api_key_name TEXT;
                ALTER TABLE events ADD COLUMN details TEXT;
                ALTER TABLE executions ADD COLUMN task_snapshot_json TEXT;
            "),
            (6, "Add task types (replace command with task_json)", "
                ALTER TABLE jobs ADD COLUMN task_json TEXT;
                UPDATE jobs SET task_json = json_object('type', 'shell', 'command', command) WHERE task_json IS NULL AND command IS NOT NULL;
            "),
            (7, "Add custom agents and job queue", "
                ALTER TABLE agents ADD COLUMN agent_type TEXT DEFAULT 'standard';
                CREATE TABLE IF NOT EXISTS job_queue (
                    id TEXT PRIMARY KEY,
                    execution_id TEXT NOT NULL,
                    agent_id TEXT NOT NULL,
                    task_json TEXT NOT NULL,
                    run_as TEXT,
                    timeout_secs INTEGER,
                    callback_url TEXT NOT NULL,
                    status TEXT NOT NULL DEFAULT 'pending',
                    created_at TEXT NOT NULL,
                    claimed_at TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_job_queue_agent ON job_queue(agent_id, status);
            "),
            (8, "Add job_id to job_queue", "
                ALTER TABLE job_queue ADD COLUMN job_id TEXT;
            "),
            (9, "Add task_types to agents", "
                ALTER TABLE agents ADD COLUMN task_types_json TEXT;
            "),
            (10, "Add output rules and extracted values", "
                ALTER TABLE jobs ADD COLUMN output_rules_json TEXT;
                ALTER TABLE executions ADD COLUMN extracted_json TEXT;
            "),
            (11, "Add job notifications", "
                ALTER TABLE jobs ADD COLUMN notifications_json TEXT;
            "),
            (12, "Add settings table", "
                CREATE TABLE IF NOT EXISTS settings (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                INSERT OR IGNORE INTO settings (key, value) VALUES ('retention_days', '7');
            "),
            (13, "Add variables table", "
                CREATE TABLE IF NOT EXISTS variables (
                    name TEXT PRIMARY KEY,
                    value TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
            "),
        ];

        for (version, description, sql) in &migrations {
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

        if current_version < migrations.len() as i64 {
            tracing::info!("database migrated to v{}", migrations.len());
        }

        Ok(())
    }
}
