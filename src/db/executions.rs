use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

use super::Db;
use super::helpers::*;
use crate::db::models::*;
use crate::error::AppError;

impl Db {
    /// Inserts a new execution record.
    pub fn insert_execution(&self, rec: &ExecutionRecord) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let triggered_by_json = serde_json::to_string(&rec.triggered_by)
            .map_err(|e| AppError::Internal(format!("serialize: {e}")))?;
        conn.execute(
            "INSERT INTO executions (id, job_id, agent_id, task_snapshot_json, status, exit_code, stdout, stderr, stdout_truncated, stderr_truncated, started_at, finished_at, triggered_by_json, retry_of, attempt_number, params_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                rec.id.to_string(),
                rec.job_id.to_string(),
                rec.agent_id.map(|a| a.to_string()),
                rec.task_snapshot.as_ref().map(serde_json::to_string).transpose().map_err(|e| AppError::Internal(format!("serialize: {e}")))?,
                rec.status.as_str(),
                rec.exit_code,
                rec.stdout,
                rec.stderr,
                rec.stdout_truncated as i32,
                rec.stderr_truncated as i32,
                rec.started_at.map(|t| t.to_rfc3339()),
                rec.finished_at.map(|t| t.to_rfc3339()),
                triggered_by_json,
                rec.retry_of.map(|id| id.to_string()),
                rec.attempt_number as i32,
                rec.params.as_ref().map(serde_json::to_string).transpose().map_err(|e| AppError::Internal(format!("serialize: {e}")))?,
            ],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    /// Updates an existing execution record's status, output, and timestamps.
    pub fn update_execution(&self, rec: &ExecutionRecord) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
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

    /// Stores extracted key-value data from output rules on an execution.
    pub fn update_execution_extracted(
        &self,
        id: Uuid,
        extracted: &serde_json::Value,
    ) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        conn.execute(
            "UPDATE executions SET extracted_json = ?1 WHERE id = ?2",
            params![
                serde_json::to_string(extracted)
                    .map_err(|e| AppError::Internal(format!("serialize: {e}")))?,
                id.to_string()
            ],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    /// Replaces the execution stdout with extracted output (for "output" target extractions).
    pub fn update_execution_stdout(&self, id: Uuid, stdout: &str) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        conn.execute(
            "UPDATE executions SET stdout = ?1 WHERE id = ?2",
            params![stdout, id.to_string()],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    /// Updates the status of an execution.
    pub fn update_execution_status(
        &self,
        id: Uuid,
        status: ExecutionStatus,
    ) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        conn.execute(
            "UPDATE executions SET status = ?1 WHERE id = ?2",
            params![status.as_str(), id.to_string()],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    /// Marks an execution as failed and appends the assertion failure message to stderr.
    pub fn fail_execution_assertion(&self, id: Uuid, message: &str) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        conn.execute(
            "UPDATE executions SET status = 'failed', stderr = COALESCE(stderr, '') || ?1 WHERE id = ?2",
            params![format!("\n[assertion failed] {}", message), id.to_string()],
        ).map_err(AppError::Db)?;
        Ok(())
    }

    /// Looks up an execution record by its UUID.
    pub fn get_execution(&self, id: Uuid) -> Result<Option<ExecutionRecord>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let mut stmt = conn
            .prepare("SELECT id, job_id, agent_id, task_snapshot_json, status, exit_code, stdout, stderr, stdout_truncated, stderr_truncated, started_at, finished_at, triggered_by_json, extracted_json, retry_of, attempt_number, params_json FROM executions WHERE id = ?1")
            .map_err(AppError::Db)?;
        let mut rows = stmt
            .query_map(params![id.to_string()], ExecutionRecord::from_row)
            .map_err(AppError::Db)?;
        match rows.next() {
            Some(Ok(rec)) => Ok(Some(rec)),
            Some(Err(e)) => Err(AppError::Db(e)),
            None => Ok(None),
        }
    }

    /// Returns a paginated list of executions for a specific job, newest first.
    pub fn list_executions_for_job(
        &self,
        job_id: Uuid,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ExecutionRecord>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let mut stmt = conn
            .prepare("SELECT id, job_id, agent_id, task_snapshot_json, status, exit_code, stdout, stderr, stdout_truncated, stderr_truncated, started_at, finished_at, triggered_by_json, extracted_json, retry_of, attempt_number, params_json FROM executions WHERE job_id = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3")
            .map_err(AppError::Db)?;
        let rows = stmt
            .query_map(params![job_id.to_string(), limit, offset], |row| {
                ExecutionRecord::from_row(row)
            })
            .map_err(AppError::Db)?;
        let mut recs = Vec::new();
        for row in rows {
            recs.push(row.map_err(AppError::Db)?);
        }
        Ok(recs)
    }

    /// Returns the total number of executions for a specific job.
    pub fn count_executions_for_job(&self, job_id: Uuid) -> Result<u32, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        conn.query_row(
            "SELECT COUNT(*) FROM executions WHERE job_id = ?1",
            params![job_id.to_string()],
            |row| row.get(0),
        )
        .map_err(AppError::Db)
    }

    fn build_exec_filters(
        status_filter: Option<&str>,
        search: Option<&str>,
        since: Option<&str>,
    ) -> QueryFilters {
        let mut f = QueryFilters::new();
        if let Some(s) = status_filter {
            f.add_eq("e.status", s);
        }
        if let Some(q) = search {
            f.add_search(q, &["j.name", "e.stdout"]);
        }
        if let Some(s) = since {
            f.add_gte("e.started_at", s);
        }
        f
    }

    /// Returns a paginated list of all executions across jobs with optional filters.
    pub fn list_all_executions(
        &self,
        status_filter: Option<&str>,
        search: Option<&str>,
        since: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ExecutionRecord>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let mut f = Self::build_exec_filters(status_filter, search, since);
        let (li, oi) = f.add_limit_offset(limit, offset);
        let sql = format!(
            "SELECT e.id, e.job_id, e.agent_id, e.task_snapshot_json, e.status, e.exit_code, e.stdout, e.stderr, e.stdout_truncated, e.stderr_truncated, e.started_at, e.finished_at, e.triggered_by_json, e.extracted_json, e.retry_of, e.attempt_number, e.params_json FROM executions e LEFT JOIN jobs j ON e.job_id = j.id{} ORDER BY e.created_at DESC LIMIT ?{} OFFSET ?{}",
            f.where_sql(),
            li,
            oi
        );
        let mut stmt = conn.prepare(&sql).map_err(AppError::Db)?;
        let rows = stmt
            .query_map(f.to_params().as_slice(), ExecutionRecord::from_row)
            .map_err(AppError::Db)?;
        let mut recs = Vec::new();
        for row in rows {
            recs.push(row.map_err(AppError::Db)?);
        }
        Ok(recs)
    }

    /// Returns the total number of executions matching the given filters.
    pub fn count_all_executions(
        &self,
        status_filter: Option<&str>,
        search: Option<&str>,
        since: Option<&str>,
    ) -> Result<u32, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let f = Self::build_exec_filters(status_filter, search, since);
        let sql = format!(
            "SELECT COUNT(*) FROM executions e LEFT JOIN jobs j ON e.job_id = j.id{}",
            f.where_sql()
        );
        let mut stmt = conn.prepare(&sql).map_err(AppError::Db)?;
        stmt.query_row(f.to_params().as_slice(), |row| row.get(0))
            .map_err(AppError::Db)
    }

    /// Returns (total, succeeded, failed) execution counts for a job.
    pub fn get_execution_counts(&self, job_id: Uuid) -> Result<(u32, u32, u32), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
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

    /// Get execution counts bucketed by minute for the last N minutes.
    /// Returns Vec<(minute_timestamp, succeeded, failed, other)>
    pub fn get_execution_timeline(
        &self,
        job_id: Option<Uuid>,
        minutes: u32,
    ) -> Result<Vec<(String, u32, u32, u32)>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
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
        let params: Vec<&dyn rusqlite::types::ToSql> = params_vec
            .iter()
            .map(|s| s as &dyn rusqlite::types::ToSql)
            .collect();
        let rows = stmt
            .query_map(params.as_slice(), |row| {
                let bucket: Option<String> = col(row, "bucket")?;
                let status: String = col(row, "status")?;
                let count: u32 = col(row, "cnt")?;
                Ok((bucket.unwrap_or_default(), status, count))
            })
            .map_err(AppError::Db)?;

        // Aggregate into buckets
        let mut bucket_map: std::collections::BTreeMap<String, (u32, u32, u32)> =
            std::collections::BTreeMap::new();

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

        Ok(bucket_map
            .into_iter()
            .map(|(k, (s, f, o))| (k, s, f, o))
            .collect())
    }

    /// Get per-job execution counts for a specific minute bucket
    pub fn get_timeline_detail(
        &self,
        bucket: &str,
    ) -> Result<Vec<(String, String, u32)>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let bucket_start = format!("{}:00", bucket);
        let bucket_end = format!("{}:59", bucket);
        let mut stmt = conn.prepare(
            "SELECT j.name, e.status, COUNT(*) as cnt FROM executions e JOIN jobs j ON e.job_id = j.id WHERE e.started_at >= ?1 AND e.started_at <= ?2 GROUP BY j.name, e.status ORDER BY cnt DESC"
        ).map_err(AppError::Db)?;
        let rows = stmt
            .query_map(params![bucket_start, bucket_end], |row| {
                Ok((
                    col::<String>(row, "name")?,
                    col::<String>(row, "status")?,
                    col::<u32>(row, "cnt")?,
                ))
            })
            .map_err(AppError::Db)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(AppError::Db)?);
        }
        Ok(result)
    }

    /// Returns the most recent execution for a job, if any.
    pub fn get_latest_execution_for_job(
        &self,
        job_id: Uuid,
    ) -> Result<Option<ExecutionRecord>, AppError> {
        let recs = self.list_executions_for_job(job_id, 1, 0)?;
        Ok(recs.into_iter().next())
    }

    /// Counts running/pending executions for a job (for concurrency control).
    pub fn count_running_executions_for_job(&self, job_id: Uuid) -> Result<u32, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        conn.query_row(
            "SELECT COUNT(*) FROM executions WHERE job_id = ?1 AND status IN ('running', 'pending')",
            params![job_id.to_string()],
            |row| row.get(0),
        )
        .map_err(AppError::Db)
    }

    /// Returns execution counts grouped by status for chart display.
    pub fn get_execution_outcome_counts(
        &self,
    ) -> Result<std::collections::HashMap<String, u32>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let mut stmt = conn
            .prepare("SELECT status, COUNT(*) FROM executions GROUP BY status")
            .map_err(AppError::Db)?;
        let rows = stmt
            .query_map([], |row| {
                let status: String = col(row, "status")?;
                let count: u32 = row.get(1)?; // COUNT(*) has no stable column name
                Ok((status, count))
            })
            .map_err(AppError::Db)?;
        let mut result = std::collections::HashMap::new();
        for row in rows {
            let (status, count) = row.map_err(AppError::Db)?;
            let label = match status.as_str() {
                "succeeded" => "Succeeded",
                "failed" => "Failed",
                "timed_out" => "Timed Out",
                "cancelled" => "Cancelled",
                "running" => "Running",
                "pending" => "Pending",
                _ => &status,
            };
            result.insert(label.to_string(), count);
        }
        Ok(result)
    }
}
