use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

use super::Db;
use super::helpers::*;
use crate::error::AppError;
use crate::models::*;

impl Db {
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

    pub fn update_execution_extracted(
        &self,
        id: Uuid,
        extracted: &serde_json::Value,
    ) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE executions SET extracted_json = ?1 WHERE id = ?2",
            params![serde_json::to_string(extracted).unwrap(), id.to_string()],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    pub fn fail_execution_assertion(&self, id: Uuid, message: &str) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE executions SET status = 'failed', stderr = COALESCE(stderr, '') || ?1 WHERE id = ?2",
            params![format!("\n[assertion failed] {}", message), id.to_string()],
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

    pub fn count_executions_for_job(&self, job_id: Uuid) -> Result<u32, AppError> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM executions WHERE job_id = ?1",
            params![job_id.to_string()],
            |row| row.get(0),
        )
        .map_err(AppError::Db)
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

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };
        let sql = format!(
            "SELECT e.id, e.job_id, e.agent_id, e.task_snapshot_json, e.status, e.exit_code, e.stdout, e.stderr, e.stdout_truncated, e.stderr_truncated, e.started_at, e.finished_at, e.triggered_by_json, e.extracted_json FROM executions e LEFT JOIN jobs j ON e.job_id = j.id {} ORDER BY e.created_at DESC LIMIT ?{} OFFSET ?{}",
            where_sql, limit_idx, offset_idx
        );

        let mut stmt = conn.prepare(&sql).map_err(AppError::Db)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = param_values
            .iter()
            .map(|s| s as &dyn rusqlite::types::ToSql)
            .collect();
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

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };
        let sql = format!(
            "SELECT COUNT(*) FROM executions e LEFT JOIN jobs j ON e.job_id = j.id {}",
            where_sql
        );

        let mut stmt = conn.prepare(&sql).map_err(AppError::Db)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = param_values
            .iter()
            .map(|s| s as &dyn rusqlite::types::ToSql)
            .collect();
        stmt.query_row(params.as_slice(), |row| row.get(0))
            .map_err(AppError::Db)
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
        let params: Vec<&dyn rusqlite::types::ToSql> = params_vec
            .iter()
            .map(|s| s as &dyn rusqlite::types::ToSql)
            .collect();
        let rows = stmt
            .query_map(params.as_slice(), |row| {
                let bucket: Option<String> = row.get(0)?;
                let status: String = row.get(1)?;
                let count: u32 = row.get(2)?;
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
        let conn = self.conn.lock().unwrap();
        let bucket_start = format!("{}:00", bucket);
        let bucket_end = format!("{}:59", bucket);
        let mut stmt = conn.prepare(
            "SELECT j.name, e.status, COUNT(*) as cnt FROM executions e JOIN jobs j ON e.job_id = j.id WHERE e.started_at >= ?1 AND e.started_at <= ?2 GROUP BY j.name, e.status ORDER BY cnt DESC"
        ).map_err(AppError::Db)?;
        let rows = stmt
            .query_map(params![bucket_start, bucket_end], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, u32>(2)?,
                ))
            })
            .map_err(AppError::Db)?;
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
}
