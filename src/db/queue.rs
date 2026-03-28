use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

use super::Db;
use crate::error::AppError;
use crate::models::*;

impl Db {
    #[allow(clippy::too_many_arguments)]
    /// Adds a job to the agent queue for poll-based dispatch.
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

    /// Dequeues the oldest pending job for the given agent, marking it as claimed.
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
            Ok((
                id, exec_id, agent, task_json, run_as, timeout, callback, job_id,
            ))
        });

        match result {
            Ok((id, exec_id, _agent, task_json, run_as, timeout, callback, job_id)) => {
                // Mark as claimed
                conn.execute(
                    "UPDATE job_queue SET status = 'claimed', claimed_at = ?1 WHERE id = ?2",
                    params![Utc::now().to_rfc3339(), id],
                )
                .map_err(AppError::Db)?;

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

    /// Marks a queue item as completed by its execution ID.
    pub fn complete_queue_item(&self, execution_id: Uuid) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE job_queue SET status = 'completed' WHERE execution_id = ?1",
            params![execution_id.to_string()],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    /// Returns the number of pending items in the queue for a given agent.
    pub fn queue_depth(&self, agent_id: Uuid) -> Result<u32, AppError> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM job_queue WHERE agent_id = ?1 AND status = 'pending'",
            params![agent_id.to_string()],
            |row| row.get(0),
        )
        .map_err(AppError::Db)
    }

    /// Fails pending queue items older than the max age and marks their executions as failed.
    pub fn fail_stale_pending_queue_items(&self, max_age_secs: i64) -> Result<u32, AppError> {
        let conn = self.conn.lock().unwrap();
        let cutoff = (Utc::now() - chrono::Duration::seconds(max_age_secs)).to_rfc3339();
        let mut stmt = conn.prepare(
            "SELECT id, execution_id FROM job_queue WHERE status = 'pending' AND created_at < ?1"
        ).map_err(AppError::Db)?;
        let rows: Vec<(String, String)> = stmt
            .query_map(params![cutoff], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(AppError::Db)?
            .filter_map(|r| r.ok())
            .collect();

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

    /// Fails claimed queue items older than the max age that never reported a result.
    pub fn fail_stale_claimed_queue_items(&self, max_age_secs: i64) -> Result<u32, AppError> {
        let conn = self.conn.lock().unwrap();
        let cutoff = (Utc::now() - chrono::Duration::seconds(max_age_secs)).to_rfc3339();
        let mut stmt = conn.prepare(
            "SELECT id, execution_id FROM job_queue WHERE status = 'claimed' AND claimed_at < ?1"
        ).map_err(AppError::Db)?;
        let rows: Vec<(String, String)> = stmt
            .query_map(params![cutoff], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(AppError::Db)?
            .filter_map(|r| r.ok())
            .collect();

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
}
