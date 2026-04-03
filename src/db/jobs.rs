use rusqlite::params;
use uuid::Uuid;

use super::Db;
use super::helpers::*;
use crate::db::models::*;
use crate::error::AppError;

impl Db {
    /// Inserts a new job. Returns a conflict error if the job name already exists.
    pub fn insert_job(&self, job: &Job) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let schedule_json = serde_json::to_string(&job.schedule)
            .map_err(|e| AppError::Internal(format!("serialize schedule: {e}")))?;
        let depends_on_json = serde_json::to_string(&job.depends_on)
            .map_err(|e| AppError::Internal(format!("serialize depends_on: {e}")))?;
        let target_json = job
            .target
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| AppError::Internal(format!("serialize target: {e}")))?;
        let output_rules_json = job
            .output_rules
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| AppError::Internal(format!("serialize output_rules: {e}")))?;
        let notifications_json = job
            .notifications
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| AppError::Internal(format!("serialize notifications: {e}")))?;
        let task_json = serde_json::to_string(&job.task)
            .map_err(|e| AppError::Internal(format!("serialize task: {e}")))?;
        conn.execute(
            "INSERT INTO jobs (id, name, description, task_json, run_as, schedule_json, status, timeout_secs, depends_on_json, target_json, created_by, created_at, updated_at, output_rules_json, notifications_json, group_name, retry_max, retry_delay_secs, retry_backoff, approval_required, priority)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                job.id.to_string(),
                job.name,
                job.description,
                task_json,
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
                job.group,
                job.retry_max as i64,
                job.retry_delay_secs as i64,
                job.retry_backoff,
                job.approval_required as i32,
                job.priority,
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

    /// Looks up a job by its UUID.
    pub fn get_job(&self, id: Uuid) -> Result<Option<Job>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let mut stmt = conn
            .prepare("SELECT id, name, description, task_json, run_as, schedule_json, status, timeout_secs, depends_on_json, target_json, created_by, created_at, updated_at, output_rules_json, notifications_json, group_name, retry_max, retry_delay_secs, retry_backoff, approval_required, priority FROM jobs WHERE id = ?1")
            .map_err(AppError::Db)?;
        let mut rows = stmt
            .query_map(params![id.to_string()], Job::from_row)
            .map_err(AppError::Db)?;
        match rows.next() {
            Some(Ok(job)) => Ok(Some(job)),
            Some(Err(e)) => Err(AppError::Db(e)),
            None => Ok(None),
        }
    }

    fn build_job_filters(
        status_filter: Option<&str>,
        search: Option<&str>,
        group_filter: Option<&str>,
    ) -> QueryFilters {
        let mut f = QueryFilters::new();
        if let Some(s) = status_filter {
            f.add_status(s);
        }
        if let Some(q) = search {
            f.add_search(q, &["name", "task_json"]);
        }
        if let Some(g) = group_filter {
            if g == "Default" {
                // Match both 'Default' and NULL (for pre-migration jobs)
                f.where_clauses
                    .push("(group_name = 'Default' OR group_name IS NULL)".to_string());
            } else {
                f.add_eq("group_name", g);
            }
        }
        f
    }

    /// Returns the total number of jobs matching the given filters.
    pub fn count_jobs(
        &self,
        status_filter: Option<&str>,
        search: Option<&str>,
        group_filter: Option<&str>,
    ) -> Result<u32, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let f = Self::build_job_filters(status_filter, search, group_filter);
        let sql = format!("SELECT COUNT(*) FROM jobs{}", f.where_sql());
        let mut stmt = conn.prepare(&sql).map_err(AppError::Db)?;
        stmt.query_row(f.to_params().as_slice(), |row| row.get(0))
            .map_err(AppError::Db)
    }

    /// Returns a paginated list of jobs with optional status and search filters.
    pub fn list_jobs(
        &self,
        status_filter: Option<&str>,
        search: Option<&str>,
        group_filter: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Job>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let mut f = Self::build_job_filters(status_filter, search, group_filter);
        let (li, oi) = f.add_limit_offset(limit, offset);
        let sql = format!(
            "SELECT id, name, description, task_json, run_as, schedule_json, status, timeout_secs, depends_on_json, target_json, created_by, created_at, updated_at, output_rules_json, notifications_json, group_name, retry_max, retry_delay_secs, retry_backoff, approval_required, priority FROM jobs{} ORDER BY name LIMIT ?{} OFFSET ?{}",
            f.where_sql(),
            li,
            oi
        );
        let mut stmt = conn.prepare(&sql).map_err(AppError::Db)?;
        let rows = stmt
            .query_map(f.to_params().as_slice(), Job::from_row)
            .map_err(AppError::Db)?;
        let mut jobs = Vec::new();
        for row in rows {
            jobs.push(row.map_err(AppError::Db)?);
        }
        Ok(jobs)
    }

    /// Updates all fields of an existing job. Returns not-found if the job does not exist.
    pub fn update_job(&self, job: &Job) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let schedule_json = serde_json::to_string(&job.schedule)
            .map_err(|e| AppError::Internal(format!("serialize schedule: {e}")))?;
        let depends_on_json = serde_json::to_string(&job.depends_on)
            .map_err(|e| AppError::Internal(format!("serialize depends_on: {e}")))?;
        let target_json = job
            .target
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| AppError::Internal(format!("serialize target: {e}")))?;
        let task_json = serde_json::to_string(&job.task)
            .map_err(|e| AppError::Internal(format!("serialize task: {e}")))?;
        let output_rules_json = job
            .output_rules
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| AppError::Internal(format!("serialize output_rules: {e}")))?;
        let notifications_json = job
            .notifications
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| AppError::Internal(format!("serialize notifications: {e}")))?;
        let changed = conn
            .execute(
                "UPDATE jobs SET name=?1, description=?2, task_json=?3, run_as=?4, schedule_json=?5, status=?6, timeout_secs=?7, depends_on_json=?8, target_json=?9, updated_at=?10, output_rules_json=?11, notifications_json=?12, group_name=?13, retry_max=?14, retry_delay_secs=?15, retry_backoff=?16, approval_required=?17, priority=?18 WHERE id=?19",
                params![
                    job.name,
                    job.description,
                    task_json,
                    job.run_as,
                    schedule_json,
                    job.status.as_str(),
                    job.timeout_secs.map(|t| t as i64),
                    depends_on_json,
                    target_json,
                    job.updated_at.to_rfc3339(),
                    output_rules_json,
                    notifications_json,
                    job.group,
                    job.retry_max as i64,
                    job.retry_delay_secs as i64,
                    job.retry_backoff,
                    job.approval_required as i32,
                    job.priority,
                    job.id.to_string(),
                ],
            )
            .map_err(AppError::Db)?;
        if changed == 0 {
            return Err(AppError::NotFound(format!("job {} not found", job.id)));
        }
        Ok(())
    }

    /// Deletes a job. Returns a conflict error if other jobs depend on it.
    pub fn delete_job(&self, id: Uuid) -> Result<(), AppError> {
        self.with_transaction(|tx| {
            // Check if other jobs depend on this one
            let dependents: Vec<String> = {
                let mut stmt = tx
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

            // Delete related records first (foreign key constraints)
            tx.execute(
                "DELETE FROM executions WHERE job_id = ?1",
                params![id.to_string()],
            )
            .map_err(AppError::Db)?;
            tx.execute(
                "DELETE FROM job_queue WHERE job_id = ?1",
                params![id.to_string()],
            )
            .map_err(AppError::Db)?;

            let changed = tx
                .execute("DELETE FROM jobs WHERE id = ?1", params![id.to_string()])
                .map_err(AppError::Db)?;
            if changed == 0 {
                return Err(AppError::NotFound(format!("job {} not found", id)));
            }
            Ok(())
        })
    }

    /// Returns all jobs with a "scheduled" status for cron evaluation.
    pub fn get_active_cron_jobs(&self) -> Result<Vec<Job>, AppError> {
        self.list_jobs(Some("scheduled"), None, None, u32::MAX, 0)
    }

    /// Returns all job IDs with their dependency lists for DAG cycle validation.
    pub fn get_all_jobs_for_dag(&self) -> Result<Vec<(Uuid, Vec<Uuid>)>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
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
            let id = Uuid::parse_str(&id_str)
                .map_err(|e| AppError::Internal(format!("invalid UUID in db: {e}")))?;
            // Support both old Vec<Uuid> and new Vec<Dependency> formats
            let deps: Vec<Uuid> =
                if let Ok(dep_objs) = serde_json::from_str::<Vec<Dependency>>(&deps_json) {
                    dep_objs.iter().map(|d| d.job_id).collect()
                } else {
                    serde_json::from_str(&deps_json).unwrap_or_default()
                };
            result.push((id, deps));
        }
        Ok(result)
    }

    /// Returns job counts grouped by task type for chart display.
    pub fn get_task_type_counts(&self) -> Result<std::collections::HashMap<String, u32>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let mut stmt = conn
            .prepare("SELECT task_json FROM jobs")
            .map_err(AppError::Db)?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(AppError::Db)?;
        let mut counts = std::collections::HashMap::new();
        for row in rows {
            let task_json = row.map_err(AppError::Db)?;
            let type_name = serde_json::from_str::<serde_json::Value>(&task_json)
                .ok()
                .and_then(|v| {
                    // Format: {"type": "shell", ...} or {"Shell": {...}}
                    let key = v
                        .get("type")
                        .and_then(|t| t.as_str().map(String::from))
                        .or_else(|| v.as_object()?.keys().next().cloned())?;
                    Some(key)
                })
                .map(|k| {
                    match k.to_lowercase().as_str() {
                        "shell" => "Shell Command",
                        "http" => "HTTP Request",
                        "script" => "Rhai Script",
                        "sql" => "SQL Query",
                        "ftp" => "FTP Transfer",
                        "file_push" | "filepush" => "File Push",
                        "kafka" => "Kafka",
                        "rabbitmq" => "RabbitMQ",
                        "mqtt" => "MQTT",
                        "redis" => "Redis",
                        "custom" => "Custom Agent",
                        _ => return k,
                    }
                    .to_string()
                })
                .unwrap_or_else(|| "Unknown".to_string());
            *counts.entry(type_name).or_insert(0) += 1;
        }
        Ok(counts)
    }

    /// Returns job counts grouped by schedule kind for chart display.
    pub fn get_schedule_type_counts(
        &self,
    ) -> Result<std::collections::HashMap<String, u32>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let mut stmt = conn
            .prepare("SELECT schedule_json FROM jobs")
            .map_err(AppError::Db)?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(AppError::Db)?;
        let mut counts = std::collections::HashMap::new();
        for row in rows {
            let sched_json = row.map_err(AppError::Db)?;
            let type_name = serde_json::from_str::<serde_json::Value>(&sched_json)
                .ok()
                .and_then(|v| {
                    // Format: {"type": "cron", "value": "..."} or {"Cron": "..."} or "OnDemand"
                    let key = v
                        .get("type")
                        .and_then(|t| t.as_str().map(String::from))
                        .or_else(|| {
                            if let Some(s) = v.as_str() {
                                Some(s.to_string())
                            } else {
                                v.as_object()?.keys().find(|k| *k != "value").cloned()
                            }
                        })?;
                    Some(match key.to_lowercase().as_str() {
                        "cron" => "Cron Schedule".to_string(),
                        "on_demand" | "ondemand" => "On Demand".to_string(),
                        "one_shot" | "oneshot" => "One-Time".to_string(),
                        "event" => "Event Trigger".to_string(),
                        _ => key,
                    })
                })
                .unwrap_or_else(|| "Unknown".to_string());
            *counts.entry(type_name).or_insert(0) += 1;
        }
        Ok(counts)
    }

    /// Returns the list of distinct group names across all jobs, merged with any
    /// custom (empty) groups stored in the `custom_groups` setting. Always includes "Default".
    pub fn get_distinct_groups(&self) -> Result<Vec<String>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let mut stmt = conn
            .prepare("SELECT DISTINCT COALESCE(group_name, 'Default') FROM jobs ORDER BY 1")
            .map_err(AppError::Db)?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(AppError::Db)?;
        let mut groups = std::collections::BTreeSet::new();
        groups.insert("Default".to_string());
        for row in rows {
            groups.insert(row.map_err(AppError::Db)?);
        }
        // Merge custom (empty) groups from settings
        drop(stmt);
        let custom = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'custom_groups'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap_or_default();
        if !custom.is_empty()
            && let Ok(names) = serde_json::from_str::<Vec<String>>(&custom)
        {
            for name in names {
                groups.insert(name);
            }
        }
        Ok(groups.into_iter().collect())
    }

    /// Returns the total number of distinct groups.
    pub fn count_groups(&self) -> Result<u32, AppError> {
        self.get_distinct_groups().map(|g| g.len() as u32)
    }

    /// Adds a custom group name that persists even with no jobs assigned.
    pub fn add_custom_group(&self, name: &str) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let existing = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'custom_groups'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap_or_default();
        let mut names: Vec<String> = if existing.is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&existing).unwrap_or_default()
        };
        if !names.contains(&name.to_string()) {
            names.push(name.to_string());
            names.sort();
            let json = serde_json::to_string(&names)
                .map_err(|e| AppError::Internal(format!("serialize: {e}")))?;
            conn.execute(
                "INSERT INTO settings (key, value) VALUES ('custom_groups', ?1) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params![json],
            )
            .map_err(AppError::Db)?;
        }
        Ok(())
    }

    /// Sets the group_name for a list of job UUIDs.
    pub fn bulk_set_group(&self, job_ids: &[Uuid], group: Option<&str>) -> Result<u32, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let mut count = 0u32;
        for id in job_ids {
            let changed = conn
                .execute(
                    "UPDATE jobs SET group_name = ?1 WHERE id = ?2",
                    rusqlite::params![group, id.to_string()],
                )
                .map_err(AppError::Db)?;
            count += changed as u32;
        }
        Ok(count)
    }

    /// Renames all jobs from one group to another.
    pub fn rename_group(&self, old_name: &str, new_name: &str) -> Result<u32, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        // Handle Default group which may also be NULL in the DB
        let count = if old_name == "Default" {
            conn.execute(
                "UPDATE jobs SET group_name = ?1 WHERE group_name = 'Default' OR group_name IS NULL",
                rusqlite::params![new_name],
            )
            .map_err(AppError::Db)?
        } else {
            conn.execute(
                "UPDATE jobs SET group_name = ?1 WHERE group_name = ?2",
                rusqlite::params![new_name, old_name],
            )
            .map_err(AppError::Db)?
        };
        Ok(count as u32)
    }

    /// Saves a snapshot of a job definition as a new version.
    pub fn save_job_version(
        &self,
        job: &Job,
        actor_key_id: Option<uuid::Uuid>,
        actor_name: Option<&str>,
    ) -> Result<i64, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;

        let job_id_str = job.id.to_string();
        let next_version: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) + 1 FROM job_versions WHERE job_id = ?1",
                params![job_id_str],
                |row| row.get(0),
            )
            .map_err(AppError::Db)?;

        let snapshot = serde_json::to_string(job)
            .map_err(|e| AppError::Internal(format!("serialize job snapshot: {e}")))?;
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO job_versions (job_id, version, snapshot_json, changed_by_key_id, changed_by_name, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                job_id_str,
                next_version,
                snapshot,
                actor_key_id.map(|id| id.to_string()),
                actor_name,
                now,
            ],
        )
        .map_err(AppError::Db)?;

        Ok(next_version)
    }

    /// Returns version history for a job, newest first.
    pub fn list_job_versions(
        &self,
        job_id: uuid::Uuid,
    ) -> Result<Vec<serde_json::Value>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let mut stmt = conn
            .prepare("SELECT version, snapshot_json, changed_by_name, created_at FROM job_versions WHERE job_id = ?1 ORDER BY version DESC")
            .map_err(AppError::Db)?;
        let rows = stmt
            .query_map(params![job_id.to_string()], |row| {
                let version: i64 = row.get(0)?;
                let snapshot: String = row.get(1)?;
                let changed_by: Option<String> = row.get(2)?;
                let created_at: String = row.get(3)?;
                Ok(serde_json::json!({
                    "version": version,
                    "snapshot": serde_json::from_str::<serde_json::Value>(&snapshot).unwrap_or_default(),
                    "changed_by": changed_by,
                    "created_at": created_at,
                }))
            })
            .map_err(AppError::Db)?;
        let mut versions = Vec::new();
        for row in rows {
            versions.push(row.map_err(AppError::Db)?);
        }
        Ok(versions)
    }
}
