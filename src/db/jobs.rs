use rusqlite::params;
use uuid::Uuid;

use super::Db;
use super::helpers::*;
use crate::error::AppError;
use crate::models::*;

impl Db {
    pub fn insert_job(&self, job: &Job) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        let schedule_json = serde_json::to_string(&job.schedule).unwrap();
        let depends_on_json = serde_json::to_string(&job.depends_on).unwrap();
        let target_json = job
            .target
            .as_ref()
            .map(|t| serde_json::to_string(t).unwrap());
        let output_rules_json = job
            .output_rules
            .as_ref()
            .map(|r| serde_json::to_string(r).unwrap());
        let notifications_json = job
            .notifications
            .as_ref()
            .map(|n| serde_json::to_string(n).unwrap());
        conn.execute(
            "INSERT INTO jobs (id, name, description, task_json, run_as, schedule_json, status, timeout_secs, depends_on_json, target_json, created_by, created_at, updated_at, output_rules_json, notifications_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
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
                notifications_json,
            ],
        ).map_err(|e| {
            if let rusqlite::Error::SqliteFailure(ref err, _) = e
                && err.code == rusqlite::ErrorCode::ConstraintViolation {
                    let msg = e.to_string();
                    if msg.contains("name") {
                        return AppError::Conflict(format!("job name '{}' already exists", job.name));
                    }
                    return AppError::BadRequest(format!("constraint violation: {msg}"));
                }
            AppError::Db(e)
        })?;
        Ok(())
    }

    pub fn get_job(&self, id: Uuid) -> Result<Option<Job>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name, description, task_json, run_as, schedule_json, status, timeout_secs, depends_on_json, target_json, created_by, created_at, updated_at, output_rules_json, notifications_json FROM jobs WHERE id = ?1")
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
            format!(
                "SELECT COUNT(*) FROM jobs WHERE {}",
                where_clauses.join(" AND ")
            )
        };

        let mut stmt = conn.prepare(&sql).map_err(AppError::Db)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = param_values
            .iter()
            .map(|s| s as &dyn rusqlite::types::ToSql)
            .collect();
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
                "SELECT id, name, description, task_json, run_as, schedule_json, status, timeout_secs, depends_on_json, target_json, created_by, created_at, updated_at, output_rules_json, notifications_json FROM jobs ORDER BY name LIMIT ?{} OFFSET ?{}",
                limit_idx, offset_idx
            )
        } else {
            format!(
                "SELECT id, name, description, task_json, run_as, schedule_json, status, timeout_secs, depends_on_json, target_json, created_by, created_at, updated_at, output_rules_json, notifications_json FROM jobs WHERE {} ORDER BY name LIMIT ?{} OFFSET ?{}",
                where_clauses.join(" AND "),
                limit_idx,
                offset_idx
            )
        };

        let mut stmt = conn.prepare(&sql).map_err(AppError::Db)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = param_values
            .iter()
            .map(|s| s as &dyn rusqlite::types::ToSql)
            .collect();
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
        let target_json = job
            .target
            .as_ref()
            .map(|t| serde_json::to_string(t).unwrap());
        let changed = conn
            .execute(
                "UPDATE jobs SET name=?1, description=?2, task_json=?3, run_as=?4, schedule_json=?5, status=?6, timeout_secs=?7, depends_on_json=?8, target_json=?9, updated_at=?10, output_rules_json=?11, notifications_json=?12 WHERE id=?13",
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
                    job.notifications.as_ref().map(|n| serde_json::to_string(n).unwrap()),
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
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
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
            let deps: Vec<Uuid> = if let Ok(dep_objs) =
                serde_json::from_str::<Vec<crate::models::Dependency>>(&deps_json)
            {
                dep_objs.iter().map(|d| d.job_id).collect()
            } else {
                serde_json::from_str(&deps_json).unwrap_or_default()
            };
            result.push((id, deps));
        }
        Ok(result)
    }
}
