use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::*;

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
            );"
        ).map_err(AppError::Db)?;

        let current_version: i64 = conn
            .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |row| row.get(0))
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
            (11, "Add settings table", "
                CREATE TABLE IF NOT EXISTS settings (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                INSERT OR IGNORE INTO settings (key, value) VALUES ('retention_days', '7');
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
                if stmt.is_empty() { continue; }
                if let Err(e) = conn.execute_batch(stmt) {
                    // Ignore "duplicate column" errors for idempotency
                    let err_str = e.to_string();
                    if err_str.contains("duplicate column") || err_str.contains("already exists") {
                        tracing::debug!("migration v{}: skipping (already applied): {}", version, err_str);
                    } else {
                        tracing::error!("migration v{} failed: {}", version, e);
                        return Err(AppError::Db(e));
                    }
                }
            }
            conn.execute(
                "INSERT INTO schema_version (version, applied_at, description) VALUES (?1, ?2, ?3)",
                params![version, Utc::now().to_rfc3339(), description],
            ).map_err(AppError::Db)?;
            tracing::info!("migration v{} applied", version);
        }

        if current_version < migrations.len() as i64 {
            tracing::info!("database migrated to v{}", migrations.len());
        }

        Ok(())
    }

    // --- Jobs ---

    pub fn insert_job(&self, job: &Job) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        let schedule_json = serde_json::to_string(&job.schedule).unwrap();
        let depends_on_json = serde_json::to_string(&job.depends_on).unwrap();
        let target_json = job.target.as_ref().map(|t| serde_json::to_string(t).unwrap());
        let output_rules_json = job.output_rules.as_ref().map(|r| serde_json::to_string(r).unwrap());
        conn.execute(
            "INSERT INTO jobs (id, name, description, task_json, run_as, schedule_json, status, timeout_secs, depends_on_json, target_json, created_by, created_at, updated_at, output_rules_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                job.id.to_string(),
                job.name,
                job.description,
                serde_json::to_string(&job.task).unwrap(),
                job.run_as,
                schedule_json,
                job.status.as_str(),
                job.timeout_secs.map(|t| t as i64),
                depends_on_json,
                target_json,
                job.created_by.map(|id| id.to_string()),
                job.created_at.to_rfc3339(),
                job.updated_at.to_rfc3339(),
                output_rules_json,
            ],
        ).map_err(|e| {
            if let rusqlite::Error::SqliteFailure(ref err, _) = e {
                if err.code == rusqlite::ErrorCode::ConstraintViolation {
                    let msg = e.to_string();
                    if msg.contains("name") {
                        return AppError::Conflict(format!("job name '{}' already exists", job.name));
                    }
                    return AppError::BadRequest(format!("constraint violation: {msg}"));
                }
            }
            AppError::Db(e)
        })?;
        Ok(())
    }

    pub fn get_job(&self, id: Uuid) -> Result<Option<Job>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name, description, task_json, run_as, schedule_json, status, timeout_secs, depends_on_json, target_json, created_by, created_at, updated_at, output_rules_json FROM jobs WHERE id = ?1")
            .map_err(AppError::Db)?;
        let mut rows = stmt
            .query_map(params![id.to_string()], |row| Ok(row_to_job(row)))
            .map_err(AppError::Db)?;
        match rows.next() {
            Some(Ok(job)) => Ok(Some(job)),
            Some(Err(e)) => Err(AppError::Db(e)),
            None => Ok(None),
        }
    }

    pub fn count_jobs(
        &self,
        status_filter: Option<&str>,
        search: Option<&str>,
    ) -> Result<u32, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut where_clauses = Vec::new();
        let mut param_values: Vec<String> = Vec::new();

        if let Some(s) = status_filter {
            param_values.push(s.to_string());
            where_clauses.push(format!("status = ?{}", param_values.len()));
        }
        if let Some(q) = search {
            let like = format!("%{}%", q);
            param_values.push(like.clone());
            let idx1 = param_values.len();
            param_values.push(like);
            let idx2 = param_values.len();
            where_clauses.push(format!("(name LIKE ?{} OR task_json LIKE ?{})", idx1, idx2));
        }

        let sql = if where_clauses.is_empty() {
            "SELECT COUNT(*) FROM jobs".to_string()
        } else {
            format!("SELECT COUNT(*) FROM jobs WHERE {}", where_clauses.join(" AND "))
        };

        let mut stmt = conn.prepare(&sql).map_err(AppError::Db)?;
        let params: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
        stmt.query_row(params.as_slice(), |row| row.get(0))
            .map_err(AppError::Db)
    }

    pub fn list_jobs(
        &self,
        status_filter: Option<&str>,
        search: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Job>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut where_clauses = Vec::new();
        let mut param_values: Vec<String> = Vec::new();

        if let Some(s) = status_filter {
            param_values.push(s.to_string());
            where_clauses.push(format!("status = ?{}", param_values.len()));
        }
        if let Some(q) = search {
            let like = format!("%{}%", q);
            param_values.push(like.clone());
            let idx1 = param_values.len();
            param_values.push(like);
            let idx2 = param_values.len();
            where_clauses.push(format!("(name LIKE ?{} OR task_json LIKE ?{})", idx1, idx2));
        }

        // limit and offset as trailing params
        param_values.push(limit.to_string());
        let limit_idx = param_values.len();
        param_values.push(offset.to_string());
        let offset_idx = param_values.len();

        let sql = if where_clauses.is_empty() {
            format!(
                "SELECT id, name, description, task_json, run_as, schedule_json, status, timeout_secs, depends_on_json, target_json, created_by, created_at, updated_at, output_rules_json FROM jobs ORDER BY name LIMIT ?{} OFFSET ?{}",
                limit_idx, offset_idx
            )
        } else {
            format!(
                "SELECT id, name, description, task_json, run_as, schedule_json, status, timeout_secs, depends_on_json, target_json, created_by, created_at, updated_at, output_rules_json FROM jobs WHERE {} ORDER BY name LIMIT ?{} OFFSET ?{}",
                where_clauses.join(" AND "), limit_idx, offset_idx
            )
        };

        let mut stmt = conn.prepare(&sql).map_err(AppError::Db)?;
        let params: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
        let rows = stmt
            .query_map(params.as_slice(), |row| Ok(row_to_job(row)))
            .map_err(AppError::Db)?;
        let mut jobs = Vec::new();
        for row in rows {
            jobs.push(row.map_err(AppError::Db)?);
        }
        Ok(jobs)
    }

    pub fn update_job(&self, job: &Job) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        let schedule_json = serde_json::to_string(&job.schedule).unwrap();
        let depends_on_json = serde_json::to_string(&job.depends_on).unwrap();
        let target_json = job.target.as_ref().map(|t| serde_json::to_string(t).unwrap());
        let changed = conn
            .execute(
                "UPDATE jobs SET name=?1, description=?2, task_json=?3, run_as=?4, schedule_json=?5, status=?6, timeout_secs=?7, depends_on_json=?8, target_json=?9, updated_at=?10, output_rules_json=?11 WHERE id=?12",
                params![
                    job.name,
                    job.description,
                    serde_json::to_string(&job.task).unwrap(),
                    job.run_as,
                    schedule_json,
                    job.status.as_str(),
                    job.timeout_secs.map(|t| t as i64),
                    depends_on_json,
                    target_json,
                    job.updated_at.to_rfc3339(),
                    job.output_rules.as_ref().map(|r| serde_json::to_string(r).unwrap()),
                    job.id.to_string(),
                ],
            )
            .map_err(AppError::Db)?;
        if changed == 0 {
            return Err(AppError::NotFound(format!("job {} not found", job.id)));
        }
        Ok(())
    }

    pub fn delete_job(&self, id: Uuid) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        // Check if other jobs depend on this one
        let dependents: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT name, depends_on_json FROM jobs WHERE id != ?1")
                .map_err(AppError::Db)?;
            let rows = stmt
                .query_map(params![id.to_string()], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                    ))
                })
                .map_err(AppError::Db)?;
            let mut deps = Vec::new();
            for row in rows {
                let (name, json) = row.map_err(AppError::Db)?;
                let ids: Vec<Uuid> = serde_json::from_str(&json).unwrap_or_default();
                if ids.contains(&id) {
                    deps.push(name);
                }
            }
            deps
        };

        if !dependents.is_empty() {
            return Err(AppError::Conflict(format!(
                "cannot delete: jobs [{}] depend on this job",
                dependents.join(", ")
            )));
        }

        // Delete executions first
        conn.execute("DELETE FROM executions WHERE job_id = ?1", params![id.to_string()])
            .map_err(AppError::Db)?;
        let changed = conn
            .execute("DELETE FROM jobs WHERE id = ?1", params![id.to_string()])
            .map_err(AppError::Db)?;
        if changed == 0 {
            return Err(AppError::NotFound(format!("job {} not found", id)));
        }
        Ok(())
    }

    pub fn get_active_cron_jobs(&self) -> Result<Vec<Job>, AppError> {
        self.list_jobs(Some("scheduled"), None, u32::MAX, 0)
    }

    pub fn get_execution_counts(&self, job_id: Uuid) -> Result<(u32, u32, u32), AppError> {
        let conn = self.conn.lock().unwrap();
        let total: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM executions WHERE job_id = ?1",
                params![job_id.to_string()],
                |row| row.get(0),
            )
            .map_err(AppError::Db)?;
        let succeeded: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM executions WHERE job_id = ?1 AND status = 'succeeded'",
                params![job_id.to_string()],
                |row| row.get(0),
            )
            .map_err(AppError::Db)?;
        let failed: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM executions WHERE job_id = ?1 AND status IN ('failed', 'timed_out')",
                params![job_id.to_string()],
                |row| row.get(0),
            )
            .map_err(AppError::Db)?;
        Ok((total, succeeded, failed))
    }

    pub fn get_all_jobs_for_dag(&self) -> Result<Vec<(Uuid, Vec<Uuid>)>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, depends_on_json FROM jobs")
            .map_err(AppError::Db)?;
        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let deps_json: String = row.get(1)?;
                Ok((id, deps_json))
            })
            .map_err(AppError::Db)?;
        let mut result = Vec::new();
        for row in rows {
            let (id_str, deps_json) = row.map_err(AppError::Db)?;
            let id = Uuid::parse_str(&id_str).unwrap();
            // Support both old Vec<Uuid> and new Vec<Dependency> formats
            let deps: Vec<Uuid> = if let Ok(dep_objs) = serde_json::from_str::<Vec<crate::models::Dependency>>(&deps_json) {
                dep_objs.iter().map(|d| d.job_id).collect()
            } else {
                serde_json::from_str(&deps_json).unwrap_or_default()
            };
            result.push((id, deps));
        }
        Ok(result)
    }

    // --- Executions ---

    pub fn insert_execution(&self, rec: &ExecutionRecord) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        let triggered_by_json = serde_json::to_string(&rec.triggered_by).unwrap();
        conn.execute(
            "INSERT INTO executions (id, job_id, agent_id, task_snapshot_json, status, exit_code, stdout, stderr, stdout_truncated, stderr_truncated, started_at, finished_at, triggered_by_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                rec.id.to_string(),
                rec.job_id.to_string(),
                rec.agent_id.map(|a| a.to_string()),
                rec.task_snapshot.as_ref().map(|t| serde_json::to_string(t).unwrap()),
                rec.status.as_str(),
                rec.exit_code,
                rec.stdout,
                rec.stderr,
                rec.stdout_truncated as i32,
                rec.stderr_truncated as i32,
                rec.started_at.map(|t| t.to_rfc3339()),
                rec.finished_at.map(|t| t.to_rfc3339()),
                triggered_by_json,
            ],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    pub fn update_execution(&self, rec: &ExecutionRecord) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE executions SET agent_id=?1, status=?2, exit_code=?3, stdout=?4, stderr=?5, stdout_truncated=?6, stderr_truncated=?7, started_at=?8, finished_at=?9 WHERE id=?10",
            params![
                rec.agent_id.map(|a| a.to_string()),
                rec.status.as_str(),
                rec.exit_code,
                rec.stdout,
                rec.stderr,
                rec.stdout_truncated as i32,
                rec.stderr_truncated as i32,
                rec.started_at.map(|t| t.to_rfc3339()),
                rec.finished_at.map(|t| t.to_rfc3339()),
                rec.id.to_string(),
            ],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    pub fn update_execution_extracted(&self, id: Uuid, extracted: &serde_json::Value) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE executions SET extracted_json = ?1 WHERE id = ?2",
            params![serde_json::to_string(extracted).unwrap(), id.to_string()],
        ).map_err(AppError::Db)?;
        Ok(())
    }

    pub fn get_execution(&self, id: Uuid) -> Result<Option<ExecutionRecord>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, job_id, agent_id, task_snapshot_json, status, exit_code, stdout, stderr, stdout_truncated, stderr_truncated, started_at, finished_at, triggered_by_json, extracted_json FROM executions WHERE id = ?1")
            .map_err(AppError::Db)?;
        let mut rows = stmt
            .query_map(params![id.to_string()], |row| Ok(row_to_execution(row)))
            .map_err(AppError::Db)?;
        match rows.next() {
            Some(Ok(rec)) => Ok(Some(rec)),
            Some(Err(e)) => Err(AppError::Db(e)),
            None => Ok(None),
        }
    }

    pub fn list_all_executions(
        &self,
        status_filter: Option<&str>,
        search: Option<&str>,
        since: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ExecutionRecord>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut where_clauses = Vec::new();
        let mut param_values: Vec<String> = Vec::new();

        if let Some(s) = status_filter {
            param_values.push(s.to_string());
            where_clauses.push(format!("e.status = ?{}", param_values.len()));
        }
        if let Some(q) = search {
            let like = format!("%{}%", q);
            param_values.push(like);
            let idx = param_values.len();
            where_clauses.push(format!("(j.name LIKE ?{} OR e.stdout LIKE ?{})", idx, idx));
        }
        if let Some(s) = since {
            param_values.push(s.to_string());
            where_clauses.push(format!("e.started_at >= ?{}", param_values.len()));
        }

        param_values.push(limit.to_string());
        let limit_idx = param_values.len();
        param_values.push(offset.to_string());
        let offset_idx = param_values.len();

        let where_sql = if where_clauses.is_empty() { String::new() } else { format!("WHERE {}", where_clauses.join(" AND ")) };
        let sql = format!(
            "SELECT e.id, e.job_id, e.agent_id, e.task_snapshot_json, e.status, e.exit_code, e.stdout, e.stderr, e.stdout_truncated, e.stderr_truncated, e.started_at, e.finished_at, e.triggered_by_json, e.extracted_json FROM executions e LEFT JOIN jobs j ON e.job_id = j.id {} ORDER BY e.created_at DESC LIMIT ?{} OFFSET ?{}",
            where_sql, limit_idx, offset_idx
        );

        let mut stmt = conn.prepare(&sql).map_err(AppError::Db)?;
        let params: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
        let rows = stmt
            .query_map(params.as_slice(), |row| Ok(row_to_execution(row)))
            .map_err(AppError::Db)?;
        let mut recs = Vec::new();
        for row in rows {
            recs.push(row.map_err(AppError::Db)?);
        }
        Ok(recs)
    }

    pub fn count_all_executions(
        &self,
        status_filter: Option<&str>,
        search: Option<&str>,
        since: Option<&str>,
    ) -> Result<u32, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut where_clauses = Vec::new();
        let mut param_values: Vec<String> = Vec::new();

        if let Some(s) = status_filter {
            param_values.push(s.to_string());
            where_clauses.push(format!("e.status = ?{}", param_values.len()));
        }
        if let Some(q) = search {
            let like = format!("%{}%", q);
            param_values.push(like);
            let idx = param_values.len();
            where_clauses.push(format!("(j.name LIKE ?{} OR e.stdout LIKE ?{})", idx, idx));
        }
        if let Some(s) = since {
            param_values.push(s.to_string());
            where_clauses.push(format!("e.started_at >= ?{}", param_values.len()));
        }

        let where_sql = if where_clauses.is_empty() { String::new() } else { format!("WHERE {}", where_clauses.join(" AND ")) };
        let sql = format!("SELECT COUNT(*) FROM executions e LEFT JOIN jobs j ON e.job_id = j.id {}", where_sql);

        let mut stmt = conn.prepare(&sql).map_err(AppError::Db)?;
        let params: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
        stmt.query_row(params.as_slice(), |row| row.get(0)).map_err(AppError::Db)
    }

    pub fn count_executions_for_job(&self, job_id: Uuid) -> Result<u32, AppError> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM executions WHERE job_id = ?1",
            params![job_id.to_string()],
            |row| row.get(0),
        )
        .map_err(AppError::Db)
    }

    pub fn list_executions_for_job(
        &self,
        job_id: Uuid,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ExecutionRecord>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, job_id, agent_id, task_snapshot_json, status, exit_code, stdout, stderr, stdout_truncated, stderr_truncated, started_at, finished_at, triggered_by_json, extracted_json FROM executions WHERE job_id = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3")
            .map_err(AppError::Db)?;
        let rows = stmt
            .query_map(params![job_id.to_string(), limit, offset], |row| {
                Ok(row_to_execution(row))
            })
            .map_err(AppError::Db)?;
        let mut recs = Vec::new();
        for row in rows {
            recs.push(row.map_err(AppError::Db)?);
        }
        Ok(recs)
    }

    /// Get execution counts bucketed by minute for the last N minutes.
    /// Returns Vec<(minute_timestamp, succeeded, failed, other)>
    pub fn get_execution_timeline(
        &self,
        job_id: Option<Uuid>,
        minutes: u32,
    ) -> Result<Vec<(String, u32, u32, u32)>, AppError> {
        let conn = self.conn.lock().unwrap();
        let cutoff = (Utc::now() - chrono::Duration::minutes(minutes as i64)).to_rfc3339();

        let (sql, params_vec): (String, Vec<String>) = match job_id {
            Some(id) => (
                "SELECT strftime('%Y-%m-%dT%H:%M', started_at) as bucket, status, COUNT(*) as cnt FROM executions WHERE started_at > ?1 AND job_id = ?2 GROUP BY bucket, status ORDER BY bucket".to_string(),
                vec![cutoff, id.to_string()],
            ),
            None => (
                "SELECT strftime('%Y-%m-%dT%H:%M', started_at) as bucket, status, COUNT(*) as cnt FROM executions WHERE started_at > ?1 GROUP BY bucket, status ORDER BY bucket".to_string(),
                vec![cutoff],
            ),
        };

        let mut stmt = conn.prepare(&sql).map_err(AppError::Db)?;
        let params: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
        let rows = stmt.query_map(params.as_slice(), |row| {
            let bucket: Option<String> = row.get(0)?;
            let status: String = row.get(1)?;
            let count: u32 = row.get(2)?;
            Ok((bucket.unwrap_or_default(), status, count))
        }).map_err(AppError::Db)?;

        // Aggregate into buckets
        let mut bucket_map: std::collections::BTreeMap<String, (u32, u32, u32)> = std::collections::BTreeMap::new();

        // Pre-fill all minute buckets
        let now = Utc::now();
        for i in 0..minutes {
            let t = now - chrono::Duration::minutes(i as i64);
            let key = t.format("%Y-%m-%dT%H:%M").to_string();
            bucket_map.entry(key).or_insert((0, 0, 0));
        }

        for row in rows {
            let (bucket, status, count) = row.map_err(AppError::Db)?;
            let entry = bucket_map.entry(bucket).or_insert((0, 0, 0));
            match status.as_str() {
                "succeeded" => entry.0 += count,
                "failed" | "timed_out" => entry.1 += count,
                _ => entry.2 += count,
            }
        }

        Ok(bucket_map.into_iter().map(|(k, (s, f, o))| (k, s, f, o)).collect())
    }

    /// Get per-job execution counts for a specific minute bucket
    pub fn get_timeline_detail(
        &self,
        bucket: &str,
    ) -> Result<Vec<(String, String, u32)>, AppError> {
        let conn = self.conn.lock().unwrap();
        let bucket_start = format!("{}:00", bucket);
        let bucket_end = format!("{}:59", bucket);
        let mut stmt = conn.prepare(
            "SELECT j.name, e.status, COUNT(*) as cnt FROM executions e JOIN jobs j ON e.job_id = j.id WHERE e.started_at >= ?1 AND e.started_at <= ?2 GROUP BY j.name, e.status ORDER BY cnt DESC"
        ).map_err(AppError::Db)?;
        let rows = stmt.query_map(params![bucket_start, bucket_end], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, u32>(2)?))
        }).map_err(AppError::Db)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(AppError::Db)?);
        }
        Ok(result)
    }

    pub fn get_latest_execution_for_job(
        &self,
        job_id: Uuid,
    ) -> Result<Option<ExecutionRecord>, AppError> {
        let recs = self.list_executions_for_job(job_id, 1, 0)?;
        Ok(recs.into_iter().next())
    }

    // --- Agents ---

    pub fn upsert_agent(&self, agent: &Agent) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        let tags_json = serde_json::to_string(&agent.tags).unwrap();
        let task_types_json = if agent.task_types.is_empty() { None } else { Some(serde_json::to_string(&agent.task_types).unwrap()) };
        conn.execute(
            "INSERT INTO agents (id, name, tags_json, hostname, address, port, agent_type, status, last_heartbeat, registered_at, task_types_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(name) DO UPDATE SET
               tags_json=excluded.tags_json, hostname=excluded.hostname, address=excluded.address,
               port=excluded.port, agent_type=excluded.agent_type, status=excluded.status,
               last_heartbeat=excluded.last_heartbeat, registered_at=excluded.registered_at,
               task_types_json=excluded.task_types_json",
            params![
                agent.id.to_string(),
                agent.name,
                tags_json,
                agent.hostname,
                agent.address,
                agent.port as i64,
                agent.agent_type.as_str(),
                agent.status.as_str(),
                agent.last_heartbeat.map(|t| t.to_rfc3339()),
                agent.registered_at.to_rfc3339(),
                task_types_json,
            ],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    pub fn get_agent(&self, id: Uuid) -> Result<Option<Agent>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name, tags_json, hostname, address, port, agent_type, status, last_heartbeat, registered_at, task_types_json FROM agents WHERE id = ?1")
            .map_err(AppError::Db)?;
        let mut rows = stmt
            .query_map(params![id.to_string()], |row| Ok(row_to_agent(row)))
            .map_err(AppError::Db)?;
        match rows.next() {
            Some(Ok(a)) => Ok(Some(a)),
            Some(Err(e)) => Err(AppError::Db(e)),
            None => Ok(None),
        }
    }

    pub fn get_agent_by_name(&self, name: &str) -> Result<Option<Agent>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name, tags_json, hostname, address, port, agent_type, status, last_heartbeat, registered_at, task_types_json FROM agents WHERE name = ?1")
            .map_err(AppError::Db)?;
        let mut rows = stmt
            .query_map(params![name], |row| Ok(row_to_agent(row)))
            .map_err(AppError::Db)?;
        match rows.next() {
            Some(Ok(a)) => Ok(Some(a)),
            Some(Err(e)) => Err(AppError::Db(e)),
            None => Ok(None),
        }
    }

    pub fn list_agents(&self) -> Result<Vec<Agent>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name, tags_json, hostname, address, port, agent_type, status, last_heartbeat, registered_at, task_types_json FROM agents ORDER BY name")
            .map_err(AppError::Db)?;
        let rows = stmt
            .query_map([], |row| Ok(row_to_agent(row)))
            .map_err(AppError::Db)?;
        let mut agents = Vec::new();
        for row in rows {
            agents.push(row.map_err(AppError::Db)?);
        }
        Ok(agents)
    }

    pub fn get_online_agents_by_tag(&self, tag: &str) -> Result<Vec<Agent>, AppError> {
        let agents = self.list_agents()?;
        Ok(agents
            .into_iter()
            .filter(|a| a.status == AgentStatus::Online && a.tags.contains(&tag.to_string()))
            .collect())
    }

    pub fn get_online_agents(&self) -> Result<Vec<Agent>, AppError> {
        let agents = self.list_agents()?;
        Ok(agents
            .into_iter()
            .filter(|a| a.status == AgentStatus::Online)
            .collect())
    }

    pub fn get_online_agents_by_type(&self, agent_type: AgentType) -> Result<Vec<Agent>, AppError> {
        let agents = self.list_agents()?;
        Ok(agents
            .into_iter()
            .filter(|a| a.status == AgentStatus::Online && a.agent_type == agent_type)
            .collect())
    }

    pub fn update_agent_heartbeat(&self, id: Uuid, at: DateTime<Utc>) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE agents SET last_heartbeat = ?1, status = 'online' WHERE id = ?2",
            params![at.to_rfc3339(), id.to_string()],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    pub fn delete_agent(&self, id: Uuid) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM agents WHERE id = ?1", params![id.to_string()])
            .map_err(AppError::Db)?;
        Ok(())
    }

    pub fn update_agent_task_types(&self, id: Uuid, task_types: &[crate::models::TaskTypeDefinition]) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        let json = if task_types.is_empty() { None } else { Some(serde_json::to_string(task_types).unwrap()) };
        conn.execute(
            "UPDATE agents SET task_types_json = ?1 WHERE id = ?2",
            params![json, id.to_string()],
        ).map_err(AppError::Db)?;
        Ok(())
    }

    pub fn expire_agents(&self, timeout: std::time::Duration) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        let cutoff = (Utc::now() - chrono::Duration::seconds(timeout.as_secs() as i64)).to_rfc3339();
        conn.execute(
            "UPDATE agents SET status = 'offline' WHERE status = 'online' AND last_heartbeat < ?1",
            params![cutoff],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    // --- Job Queue (Custom Agents) ---

    pub fn enqueue_job(
        &self,
        id: Uuid,
        execution_id: Uuid,
        agent_id: Uuid,
        job_id: Uuid,
        task: &TaskType,
        run_as: Option<&str>,
        timeout_secs: Option<u64>,
        callback_url: &str,
    ) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO job_queue (id, execution_id, agent_id, job_id, task_json, run_as, timeout_secs, callback_url, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending', ?9)",
            params![
                id.to_string(),
                execution_id.to_string(),
                agent_id.to_string(),
                job_id.to_string(),
                serde_json::to_string(task).unwrap(),
                run_as,
                timeout_secs.map(|t| t as i64),
                callback_url,
                Utc::now().to_rfc3339(),
            ],
        ).map_err(AppError::Db)?;
        Ok(())
    }

    pub fn dequeue_job(&self, agent_id: Uuid) -> Result<Option<serde_json::Value>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, execution_id, agent_id, task_json, run_as, timeout_secs, callback_url, job_id FROM job_queue WHERE agent_id = ?1 AND status = 'pending' ORDER BY created_at ASC LIMIT 1")
            .map_err(AppError::Db)?;
        let result = stmt.query_row(params![agent_id.to_string()], |row| {
            let id: String = row.get(0)?;
            let exec_id: String = row.get(1)?;
            let agent: String = row.get(2)?;
            let task_json: String = row.get(3)?;
            let run_as: Option<String> = row.get(4)?;
            let timeout: Option<i64> = row.get(5)?;
            let callback: String = row.get(6)?;
            let job_id: Option<String> = row.get(7)?;
            Ok((id, exec_id, agent, task_json, run_as, timeout, callback, job_id))
        });

        match result {
            Ok((id, exec_id, _agent, task_json, run_as, timeout, callback, job_id)) => {
                // Mark as claimed
                conn.execute(
                    "UPDATE job_queue SET status = 'claimed', claimed_at = ?1 WHERE id = ?2",
                    params![Utc::now().to_rfc3339(), id],
                ).map_err(AppError::Db)?;

                let task: serde_json::Value = serde_json::from_str(&task_json).unwrap_or_default();
                Ok(Some(serde_json::json!({
                    "queue_id": id,
                    "execution_id": exec_id,
                    "job_id": job_id,
                    "agent_id": agent_id.to_string(),
                    "task": task,
                    "run_as": run_as,
                    "timeout_secs": timeout,
                    "callback_url": callback,
                })))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Db(e)),
        }
    }

    pub fn complete_queue_item(&self, execution_id: Uuid) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE job_queue SET status = 'completed' WHERE execution_id = ?1",
            params![execution_id.to_string()],
        ).map_err(AppError::Db)?;
        Ok(())
    }

    pub fn queue_depth(&self, agent_id: Uuid) -> Result<u32, AppError> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM job_queue WHERE agent_id = ?1 AND status = 'pending'",
            params![agent_id.to_string()],
            |row| row.get(0),
        ).map_err(AppError::Db)
    }

    pub fn fail_stale_pending_queue_items(&self, max_age_secs: i64) -> Result<u32, AppError> {
        let conn = self.conn.lock().unwrap();
        let cutoff = (Utc::now() - chrono::Duration::seconds(max_age_secs)).to_rfc3339();
        let mut stmt = conn.prepare(
            "SELECT id, execution_id FROM job_queue WHERE status = 'pending' AND created_at < ?1"
        ).map_err(AppError::Db)?;
        let rows: Vec<(String, String)> = stmt.query_map(params![cutoff], |row| {
            Ok((row.get(0)?, row.get(1)?))
        }).map_err(AppError::Db)?.filter_map(|r| r.ok()).collect();

        let count = rows.len() as u32;
        for (queue_id, exec_id) in &rows {
            let _ = conn.execute(
                "UPDATE job_queue SET status = 'completed' WHERE id = ?1",
                params![queue_id],
            );
            let now = Utc::now().to_rfc3339();
            let _ = conn.execute(
                "UPDATE executions SET status = 'failed', stderr = 'queued for custom agent but never claimed (timeout)', finished_at = ?1 WHERE id = ?2 AND status = 'pending'",
                params![now, exec_id],
            );
        }
        Ok(count)
    }

    pub fn fail_stale_claimed_queue_items(&self, max_age_secs: i64) -> Result<u32, AppError> {
        let conn = self.conn.lock().unwrap();
        let cutoff = (Utc::now() - chrono::Duration::seconds(max_age_secs)).to_rfc3339();
        let mut stmt = conn.prepare(
            "SELECT id, execution_id FROM job_queue WHERE status = 'claimed' AND claimed_at < ?1"
        ).map_err(AppError::Db)?;
        let rows: Vec<(String, String)> = stmt.query_map(params![cutoff], |row| {
            Ok((row.get(0)?, row.get(1)?))
        }).map_err(AppError::Db)?.filter_map(|r| r.ok()).collect();

        let count = rows.len() as u32;
        for (queue_id, exec_id) in &rows {
            let _ = conn.execute(
                "UPDATE job_queue SET status = 'completed' WHERE id = ?1",
                params![queue_id],
            );
            let now = Utc::now().to_rfc3339();
            let _ = conn.execute(
                "UPDATE executions SET status = 'failed', stderr = 'custom agent claimed job but never reported result (timeout)', finished_at = ?1 WHERE id = ?2 AND (status = 'pending' OR status = 'running')",
                params![now, exec_id],
            );
        }
        Ok(count)
    }

    // --- API Keys ---

    pub fn insert_api_key(&self, key: &ApiKey) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO api_keys (id, key_prefix, key_hash, name, role, created_at, last_used_at, active) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                key.id.to_string(),
                key.key_prefix,
                key.key_hash,
                key.name,
                key.role.as_str(),
                key.created_at.to_rfc3339(),
                key.last_used_at.map(|t| t.to_rfc3339()),
                key.active as i32,
            ],
        ).map_err(AppError::Db)?;
        Ok(())
    }

    pub fn get_api_key_by_hash(&self, hash: &str) -> Result<Option<ApiKey>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, key_prefix, key_hash, name, role, created_at, last_used_at, active FROM api_keys WHERE key_hash = ?1 AND active = 1")
            .map_err(AppError::Db)?;
        let mut rows = stmt
            .query_map(params![hash], |row| Ok(row_to_api_key(row)))
            .map_err(AppError::Db)?;
        match rows.next() {
            Some(Ok(key)) => Ok(Some(key)),
            Some(Err(e)) => Err(AppError::Db(e)),
            None => Ok(None),
        }
    }

    pub fn list_api_keys(&self) -> Result<Vec<ApiKey>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, key_prefix, key_hash, name, role, created_at, last_used_at, active FROM api_keys ORDER BY created_at DESC")
            .map_err(AppError::Db)?;
        let rows = stmt
            .query_map([], |row| Ok(row_to_api_key(row)))
            .map_err(AppError::Db)?;
        let mut keys = Vec::new();
        for row in rows {
            keys.push(row.map_err(AppError::Db)?);
        }
        Ok(keys)
    }

    pub fn delete_api_key(&self, id: Uuid) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE api_keys SET active = 0 WHERE id = ?1",
            params![id.to_string()],
        ).map_err(AppError::Db)?;
        Ok(())
    }

    pub fn update_api_key_last_used(&self, id: Uuid, at: DateTime<Utc>) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE api_keys SET last_used_at = ?1 WHERE id = ?2",
            params![at.to_rfc3339(), id.to_string()],
        ).map_err(AppError::Db)?;
        Ok(())
    }

    pub fn count_api_keys(&self) -> Result<u32, AppError> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM api_keys WHERE active = 1", [], |row| row.get(0))
            .map_err(AppError::Db)
    }

    // --- Events ---

    pub fn insert_event(&self, event: &Event) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO events (id, kind, severity, message, job_id, agent_id, api_key_id, api_key_name, details, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                event.id.to_string(),
                event.kind,
                event.severity.as_str(),
                event.message,
                event.job_id.map(|id| id.to_string()),
                event.agent_id.map(|id| id.to_string()),
                event.api_key_id.map(|id| id.to_string()),
                event.api_key_name,
                event.details,
                event.timestamp.to_rfc3339(),
            ],
        ).map_err(AppError::Db)?;
        Ok(())
    }

    pub fn list_events(&self, since: Option<&str>, limit: u32, offset: u32) -> Result<Vec<Event>, AppError> {
        let conn = self.conn.lock().unwrap();
        let sql = match since {
            Some(_) => "SELECT id, kind, severity, message, job_id, agent_id, api_key_id, api_key_name, details, timestamp FROM events WHERE timestamp >= ?3 ORDER BY timestamp DESC LIMIT ?1 OFFSET ?2",
            None => "SELECT id, kind, severity, message, job_id, agent_id, api_key_id, api_key_name, details, timestamp FROM events ORDER BY timestamp DESC LIMIT ?1 OFFSET ?2",
        };
        let mut stmt = conn.prepare(sql).map_err(AppError::Db)?;
        let rows = stmt
            .query_map(
                match since {
                    Some(s) => rusqlite::params_from_iter(vec![limit.to_string(), offset.to_string(), s.to_string()]),
                    None => rusqlite::params_from_iter(vec![limit.to_string(), offset.to_string()]),
                },
                |row| {
                let id_str: String = row.get(0)?;
                let severity_str: String = row.get(2)?;
                let job_id_str: Option<String> = row.get(4)?;
                let agent_id_str: Option<String> = row.get(5)?;
                let api_key_id_str: Option<String> = row.get(6)?;
                let api_key_name: Option<String> = row.get(7)?;
                let details: Option<String> = row.get(8)?;
                let ts_str: String = row.get(9)?;
                Ok(Event {
                    id: Uuid::parse_str(&id_str).unwrap(),
                    kind: row.get(1)?,
                    severity: EventSeverity::from_str(&severity_str).unwrap_or(EventSeverity::Info),
                    message: row.get(3)?,
                    job_id: job_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                    agent_id: agent_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                    api_key_id: api_key_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                    api_key_name,
                    details,
                    timestamp: DateTime::parse_from_rfc3339(&ts_str).unwrap().with_timezone(&Utc),
                })
            })
            .map_err(AppError::Db)?;
        let mut events = Vec::new();
        for row in rows {
            events.push(row.map_err(AppError::Db)?);
        }
        Ok(events)
    }

    pub fn count_events(&self, since: Option<&str>) -> Result<u32, AppError> {
        let conn = self.conn.lock().unwrap();
        match since {
            Some(s) => conn.query_row("SELECT COUNT(*) FROM events WHERE timestamp >= ?1", params![s], |row| row.get(0)),
            None => conn.query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0)),
        }.map_err(AppError::Db)
    }

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
            details,
            timestamp: Utc::now(),
        };
        self.insert_event(&event)
    }

    // --- Settings ---

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, AppError> {
        let conn = self.conn.lock().unwrap();
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

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        ).map_err(AppError::Db)?;
        Ok(())
    }

    pub fn get_all_settings(&self) -> Result<std::collections::HashMap<String, String>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT key, value FROM settings").map_err(AppError::Db)?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }).map_err(AppError::Db)?;
        let mut map = std::collections::HashMap::new();
        for row in rows {
            let (k, v) = row.map_err(AppError::Db)?;
            map.insert(k, v);
        }
        Ok(map)
    }

    pub fn purge_old_executions(&self, retention_days: i64) -> Result<u32, AppError> {
        let conn = self.conn.lock().unwrap();
        let cutoff = (Utc::now() - chrono::Duration::days(retention_days)).to_rfc3339();
        let deleted = conn.execute(
            "DELETE FROM executions WHERE finished_at IS NOT NULL AND finished_at < ?1",
            params![cutoff],
        ).map_err(AppError::Db)?;
        Ok(deleted as u32)
    }

    pub fn purge_old_events(&self, retention_days: i64) -> Result<u32, AppError> {
        let conn = self.conn.lock().unwrap();
        let cutoff = (Utc::now() - chrono::Duration::days(retention_days)).to_rfc3339();
        let deleted = conn.execute(
            "DELETE FROM events WHERE timestamp < ?1",
            params![cutoff],
        ).map_err(AppError::Db)?;
        Ok(deleted as u32)
    }

    pub fn purge_old_queue_items(&self, retention_days: i64) -> Result<u32, AppError> {
        let conn = self.conn.lock().unwrap();
        let cutoff = (Utc::now() - chrono::Duration::days(retention_days)).to_rfc3339();
        let deleted = conn.execute(
            "DELETE FROM job_queue WHERE status = 'completed' AND created_at < ?1",
            params![cutoff],
        ).map_err(AppError::Db)?;
        Ok(deleted as u32)
    }
}

// Columns: id(0), name(1), description(2), command(3), run_as(4), schedule_json(5), status(6),
//          timeout_secs(7), depends_on_json(8), target_json(9), created_by(10), created_at(11), updated_at(12)
fn row_to_job(row: &rusqlite::Row) -> Job {
    let id_str: String = row.get(0).unwrap();
    let run_as: Option<String> = row.get(4).unwrap();
    let schedule_json: String = row.get(5).unwrap();
    let status_str: String = row.get(6).unwrap();
    let timeout: Option<i64> = row.get(7).unwrap();
    let depends_json: String = row.get(8).unwrap();
    let target_json: Option<String> = row.get(9).unwrap();
    let created_by_str: Option<String> = row.get(10).unwrap();
    let created_str: String = row.get(11).unwrap();
    let updated_str: String = row.get(12).unwrap();

    let task_json: String = row.get(3).unwrap();

    Job {
        id: Uuid::parse_str(&id_str).unwrap(),
        name: row.get(1).unwrap(),
        description: row.get(2).unwrap(),
        task: serde_json::from_str(&task_json).unwrap(),
        run_as,
        schedule: serde_json::from_str(&schedule_json).unwrap(),
        status: JobStatus::from_str(&status_str).unwrap(),
        timeout_secs: timeout.map(|t| t as u64),
        depends_on: serde_json::from_str(&depends_json).unwrap_or_default(),
        target: target_json.and_then(|s| serde_json::from_str(&s).ok()),
        created_by: created_by_str.and_then(|s| Uuid::parse_str(&s).ok()),
        created_at: DateTime::parse_from_rfc3339(&created_str)
            .unwrap()
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_str)
            .unwrap()
            .with_timezone(&Utc),
        output_rules: {
            let or_json: Option<String> = row.get(13).unwrap_or(None);
            or_json.and_then(|s| serde_json::from_str(&s).ok())
        },
    }
}

// Columns: id(0), job_id(1), agent_id(2), task_snapshot_json(3), status(4), exit_code(5),
//          stdout(6), stderr(7), stdout_truncated(8), stderr_truncated(9), started_at(10), finished_at(11), triggered_by_json(12)
fn row_to_execution(row: &rusqlite::Row) -> ExecutionRecord {
    let id_str: String = row.get(0).unwrap();
    let job_id_str: String = row.get(1).unwrap();
    let agent_id_str: Option<String> = row.get(2).unwrap();
    let task_snap_json: Option<String> = row.get(3).unwrap();
    let status_str: String = row.get(4).unwrap();
    let stdout_trunc: i32 = row.get(8).unwrap();
    let stderr_trunc: i32 = row.get(9).unwrap();
    let started_str: Option<String> = row.get(10).unwrap();
    let finished_str: Option<String> = row.get(11).unwrap();
    let triggered_json: String = row.get(12).unwrap();

    ExecutionRecord {
        id: Uuid::parse_str(&id_str).unwrap(),
        job_id: Uuid::parse_str(&job_id_str).unwrap(),
        agent_id: agent_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
        task_snapshot: task_snap_json.and_then(|s| serde_json::from_str(&s).ok()),
        status: ExecutionStatus::from_str(&status_str).unwrap(),
        exit_code: row.get(5).unwrap(),
        stdout: row.get(6).unwrap(),
        stderr: row.get(7).unwrap(),
        stdout_truncated: stdout_trunc != 0,
        stderr_truncated: stderr_trunc != 0,
        started_at: started_str.map(|s| {
            DateTime::parse_from_rfc3339(&s)
                .unwrap()
                .with_timezone(&Utc)
        }),
        finished_at: finished_str.map(|s| {
            DateTime::parse_from_rfc3339(&s)
                .unwrap()
                .with_timezone(&Utc)
        }),
        triggered_by: serde_json::from_str(&triggered_json).unwrap(),
        extracted: {
            let ex_json: Option<String> = row.get(13).unwrap_or(None);
            ex_json.and_then(|s| serde_json::from_str(&s).ok())
        },
    }
}

fn row_to_agent(row: &rusqlite::Row) -> Agent {
    let id_str: String = row.get(0).unwrap();
    let tags_json: String = row.get(2).unwrap();
    let agent_type_str: String = row.get(6).unwrap_or_else(|_| "standard".to_string());
    let status_str: String = row.get(7).unwrap();
    let hb_str: Option<String> = row.get(8).unwrap();
    let reg_str: String = row.get(9).unwrap();
    let task_types_str: Option<String> = row.get(10).unwrap_or(None);

    Agent {
        id: Uuid::parse_str(&id_str).unwrap(),
        name: row.get(1).unwrap(),
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
        hostname: row.get(3).unwrap(),
        address: row.get(4).unwrap(),
        port: {
            let p: i64 = row.get(5).unwrap();
            p as u16
        },
        agent_type: AgentType::from_str(&agent_type_str),
        status: AgentStatus::from_str(&status_str).unwrap_or(AgentStatus::Offline),
        last_heartbeat: hb_str.map(|s| {
            DateTime::parse_from_rfc3339(&s)
                .unwrap()
                .with_timezone(&Utc)
        }),
        registered_at: DateTime::parse_from_rfc3339(&reg_str)
            .unwrap()
            .with_timezone(&Utc),
        task_types: task_types_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default(),
    }
}

fn row_to_api_key(row: &rusqlite::Row) -> ApiKey {
    let id_str: String = row.get(0).unwrap();
    let role_str: String = row.get(4).unwrap();
    let created_str: String = row.get(5).unwrap();
    let last_used_str: Option<String> = row.get(6).unwrap();
    let active_int: i32 = row.get(7).unwrap();

    ApiKey {
        id: Uuid::parse_str(&id_str).unwrap(),
        key_prefix: row.get(1).unwrap(),
        key_hash: row.get(2).unwrap(),
        name: row.get(3).unwrap(),
        role: ApiKeyRole::from_str(&role_str).unwrap_or(ApiKeyRole::Viewer),
        created_at: DateTime::parse_from_rfc3339(&created_str).unwrap().with_timezone(&Utc),
        last_used_at: last_used_str.map(|s| DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&Utc)),
        active: active_int != 0,
    }
}
