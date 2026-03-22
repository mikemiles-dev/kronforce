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
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                command TEXT NOT NULL,
                schedule_json TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'active',
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
            ",
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    // --- Jobs ---

    pub fn insert_job(&self, job: &Job) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        let schedule_json = serde_json::to_string(&job.schedule).unwrap();
        let depends_on_json = serde_json::to_string(&job.depends_on).unwrap();
        conn.execute(
            "INSERT INTO jobs (id, name, description, command, schedule_json, status, timeout_secs, depends_on_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                job.id.to_string(),
                job.name,
                job.description,
                job.command,
                schedule_json,
                job.status.as_str(),
                job.timeout_secs.map(|t| t as i64),
                depends_on_json,
                job.created_at.to_rfc3339(),
                job.updated_at.to_rfc3339(),
            ],
        ).map_err(|e| {
            if let rusqlite::Error::SqliteFailure(ref err, _) = e {
                if err.code == rusqlite::ErrorCode::ConstraintViolation {
                    return AppError::Conflict(format!("job name '{}' already exists", job.name));
                }
            }
            AppError::Db(e)
        })?;
        Ok(())
    }

    pub fn get_job(&self, id: Uuid) -> Result<Option<Job>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name, description, command, schedule_json, status, timeout_secs, depends_on_json, created_at, updated_at FROM jobs WHERE id = ?1")
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
            where_clauses.push(format!("(name LIKE ?{} OR command LIKE ?{})", idx1, idx2));
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
            where_clauses.push(format!("(name LIKE ?{} OR command LIKE ?{})", idx1, idx2));
        }

        // limit and offset as trailing params
        param_values.push(limit.to_string());
        let limit_idx = param_values.len();
        param_values.push(offset.to_string());
        let offset_idx = param_values.len();

        let sql = if where_clauses.is_empty() {
            format!(
                "SELECT id, name, description, command, schedule_json, status, timeout_secs, depends_on_json, created_at, updated_at FROM jobs ORDER BY name LIMIT ?{} OFFSET ?{}",
                limit_idx, offset_idx
            )
        } else {
            format!(
                "SELECT id, name, description, command, schedule_json, status, timeout_secs, depends_on_json, created_at, updated_at FROM jobs WHERE {} ORDER BY name LIMIT ?{} OFFSET ?{}",
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
        let changed = conn
            .execute(
                "UPDATE jobs SET name=?1, description=?2, command=?3, schedule_json=?4, status=?5, timeout_secs=?6, depends_on_json=?7, updated_at=?8 WHERE id=?9",
                params![
                    job.name,
                    job.description,
                    job.command,
                    schedule_json,
                    job.status.as_str(),
                    job.timeout_secs.map(|t| t as i64),
                    depends_on_json,
                    job.updated_at.to_rfc3339(),
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
        self.list_jobs(Some("active"), None, u32::MAX, 0)
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
            let deps: Vec<Uuid> = serde_json::from_str(&deps_json).unwrap_or_default();
            result.push((id, deps));
        }
        Ok(result)
    }

    // --- Executions ---

    pub fn insert_execution(&self, rec: &ExecutionRecord) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        let triggered_by_json = serde_json::to_string(&rec.triggered_by).unwrap();
        conn.execute(
            "INSERT INTO executions (id, job_id, status, exit_code, stdout, stderr, stdout_truncated, stderr_truncated, started_at, finished_at, triggered_by_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                rec.id.to_string(),
                rec.job_id.to_string(),
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
            "UPDATE executions SET status=?1, exit_code=?2, stdout=?3, stderr=?4, stdout_truncated=?5, stderr_truncated=?6, started_at=?7, finished_at=?8 WHERE id=?9",
            params![
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

    pub fn get_execution(&self, id: Uuid) -> Result<Option<ExecutionRecord>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, job_id, status, exit_code, stdout, stderr, stdout_truncated, stderr_truncated, started_at, finished_at, triggered_by_json FROM executions WHERE id = ?1")
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
            .prepare("SELECT id, job_id, status, exit_code, stdout, stderr, stdout_truncated, stderr_truncated, started_at, finished_at, triggered_by_json FROM executions WHERE job_id = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3")
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

    pub fn get_latest_execution_for_job(
        &self,
        job_id: Uuid,
    ) -> Result<Option<ExecutionRecord>, AppError> {
        let recs = self.list_executions_for_job(job_id, 1, 0)?;
        Ok(recs.into_iter().next())
    }
}

fn row_to_job(row: &rusqlite::Row) -> Job {
    let id_str: String = row.get(0).unwrap();
    let schedule_json: String = row.get(4).unwrap();
    let status_str: String = row.get(5).unwrap();
    let timeout: Option<i64> = row.get(6).unwrap();
    let depends_json: String = row.get(7).unwrap();
    let created_str: String = row.get(8).unwrap();
    let updated_str: String = row.get(9).unwrap();

    Job {
        id: Uuid::parse_str(&id_str).unwrap(),
        name: row.get(1).unwrap(),
        description: row.get(2).unwrap(),
        command: row.get(3).unwrap(),
        schedule: serde_json::from_str(&schedule_json).unwrap(),
        status: JobStatus::from_str(&status_str).unwrap(),
        timeout_secs: timeout.map(|t| t as u64),
        depends_on: serde_json::from_str(&depends_json).unwrap_or_default(),
        created_at: DateTime::parse_from_rfc3339(&created_str)
            .unwrap()
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_str)
            .unwrap()
            .with_timezone(&Utc),
    }
}

fn row_to_execution(row: &rusqlite::Row) -> ExecutionRecord {
    let id_str: String = row.get(0).unwrap();
    let job_id_str: String = row.get(1).unwrap();
    let status_str: String = row.get(2).unwrap();
    let stdout_trunc: i32 = row.get(6).unwrap();
    let stderr_trunc: i32 = row.get(7).unwrap();
    let started_str: Option<String> = row.get(8).unwrap();
    let finished_str: Option<String> = row.get(9).unwrap();
    let triggered_json: String = row.get(10).unwrap();

    ExecutionRecord {
        id: Uuid::parse_str(&id_str).unwrap(),
        job_id: Uuid::parse_str(&job_id_str).unwrap(),
        status: ExecutionStatus::from_str(&status_str).unwrap(),
        exit_code: row.get(3).unwrap(),
        stdout: row.get(4).unwrap(),
        stderr: row.get(5).unwrap(),
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
    }
}
