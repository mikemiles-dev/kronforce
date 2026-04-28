//! Job management API handlers, split into focused sub-modules.

mod crud;
mod groups;
mod triggers;
mod versions;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::auth::AuthUser;
use super::{AppState, PaginatedResponse, log_and_notify, paginate, paginated_response};
use crate::db::models::*;
use crate::db::{Db, db_call};
use crate::error::AppError;
use crate::scheduler::cron_parser::CronSchedule;

// Re-export all handler functions so `src/api/mod.rs` can reference them as `jobs::*`.
pub(crate) use crud::{create_job, delete_job, get_job_handler, list_jobs, update_job};
pub(crate) use groups::{
    bulk_set_group, create_group, delete_pipeline_schedule, get_pipeline_schedule, list_groups,
    rename_group, set_pipeline_schedule,
};
pub(crate) use triggers::{
    approve_execution, delete_webhook, generate_webhook, trigger_job, webhook_trigger,
};
pub(crate) use versions::list_job_versions;

/// Request body for creating a new job.
#[derive(Deserialize)]
pub(crate) struct CreateJobRequest {
    name: String,
    description: Option<String>,
    task: TaskType,
    run_as: Option<String>,
    schedule: ScheduleKind,
    timeout_secs: Option<u64>,
    depends_on: Option<Vec<Dependency>>,
    target: Option<AgentTarget>,
    output_rules: Option<OutputRules>,
    notifications: Option<JobNotificationConfig>,
    group: Option<String>,
    retry_max: Option<u32>,
    retry_delay_secs: Option<u64>,
    retry_backoff: Option<f64>,
    #[serde(default)]
    approval_required: bool,
    #[serde(default)]
    priority: i32,
    sla_deadline: Option<String>,
    #[serde(default)]
    sla_warning_mins: u32,
    starts_at: Option<chrono::DateTime<chrono::Utc>>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    max_concurrent: Option<u32>,
    parameters: Option<Vec<JobParameter>>,
    timezone: Option<String>,
}

/// Request body for updating an existing job. All fields are optional (partial update).
#[derive(Deserialize)]
pub(crate) struct UpdateJobRequest {
    name: Option<String>,
    description: Option<String>,
    task: Option<TaskType>,
    run_as: Option<String>,
    schedule: Option<ScheduleKind>,
    status: Option<JobStatus>,
    timeout_secs: Option<u64>,
    depends_on: Option<Vec<Dependency>>,
    target: Option<AgentTarget>,
    output_rules: Option<OutputRules>,
    notifications: Option<JobNotificationConfig>,
    group: Option<String>,
    retry_max: Option<u32>,
    retry_delay_secs: Option<u64>,
    retry_backoff: Option<f64>,
    approval_required: Option<bool>,
    priority: Option<i32>,
    sla_deadline: Option<String>,
    sla_warning_mins: Option<u32>,
    starts_at: Option<chrono::DateTime<chrono::Utc>>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    max_concurrent: Option<u32>,
    parameters: Option<Vec<JobParameter>>,
    timezone: Option<String>,
    /// Optimistic concurrency: if set, the update is rejected with 409 Conflict
    /// when the job's `updated_at` doesn't match this value (another user modified it).
    pub(crate) if_unmodified_since: Option<chrono::DateTime<chrono::Utc>>,
}

/// Query parameters for the trigger endpoint.
#[derive(Deserialize)]
pub(crate) struct TriggerQuery {
    /// When true, skip dependency checks for this single execution.
    pub(crate) skip_deps: Option<bool>,
}

/// Optional JSON body for trigger requests (params, etc.).
#[derive(Deserialize, Default)]
pub(crate) struct TriggerBody {
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

/// Response returned when a job is manually triggered.
#[derive(Serialize)]
pub(crate) struct TriggerResponse {
    pub(crate) message: String,
    pub(crate) job_id: Uuid,
}

/// Query parameters for paginated job listing.
#[derive(Deserialize)]
pub(crate) struct ListJobsQuery {
    pub(crate) status: Option<String>,
    pub(crate) search: Option<String>,
    pub(crate) group: Option<String>,
    pub(crate) page: Option<u32>,
    pub(crate) per_page: Option<u32>,
}

/// Summary of a job's most recent execution.
#[derive(Serialize)]
pub(crate) struct LastExecution {
    pub(crate) id: uuid::Uuid,
    pub(crate) status: ExecutionStatus,
    pub(crate) exit_code: Option<i32>,
    pub(crate) finished_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize)]
pub(crate) struct ExecutionCounts {
    pub(crate) total: u32,
    pub(crate) succeeded: u32,
    pub(crate) failed: u32,
}

#[derive(Serialize)]
pub(crate) struct DepStatus {
    pub(crate) job_id: Uuid,
    pub(crate) job_name: Option<String>,
    pub(crate) within_secs: Option<u64>,
    pub(crate) satisfied: bool,
}

/// Enriched job response with next fire time, execution stats, and dependency status.
#[derive(Serialize)]
pub(crate) struct JobResponse {
    #[serde(flatten)]
    pub(crate) job: Job,
    pub(crate) next_fire_time: Option<chrono::DateTime<chrono::Utc>>,
    pub(crate) last_execution: Option<LastExecution>,
    pub(crate) execution_counts: ExecutionCounts,
    pub(crate) deps_satisfied: bool,
    pub(crate) deps_status: Vec<DepStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) webhook_url: Option<String>,
}

/// Maximum allowed length for group names.
pub(super) const MAX_GROUP_NAME_LEN: usize = 50;

/// Default group name for jobs that don't specify one.
pub(super) const DEFAULT_GROUP_NAME: &str = "Default";

/// Maximum allowed length for job names.
const MAX_JOB_NAME_LEN: usize = 255;
/// Maximum allowed length for cron expressions.
const MAX_CRON_EXPR_LEN: usize = 200;

/// Persists a group name to custom_groups so it survives job deletion.
pub(super) async fn persist_group(db: &crate::db::Db, group: &Option<String>) {
    if let Some(g) = group
        && g != DEFAULT_GROUP_NAME
    {
        let db = db.clone();
        let g = g.clone();
        let _ = db_call(&db, move |db| db.add_custom_group(&g)).await;
    }
}

/// Normalizes and validates a group name. Empty/None becomes "Default".
pub(super) fn normalize_group(group: Option<String>) -> Result<Option<String>, AppError> {
    match group {
        None => Ok(Some(DEFAULT_GROUP_NAME.to_string())),
        Some(g) if g.trim().is_empty() => Ok(Some(DEFAULT_GROUP_NAME.to_string())),
        Some(g) => {
            let g = g.trim().to_string();
            if g.len() > MAX_GROUP_NAME_LEN {
                return Err(AppError::BadRequest(format!(
                    "group name exceeds {} character limit",
                    MAX_GROUP_NAME_LEN
                )));
            }
            if !g
                .chars()
                .all(|c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '_')
            {
                return Err(AppError::BadRequest(
                    "group name may only contain alphanumeric characters, spaces, hyphens, and underscores".into(),
                ));
            }
            Ok(Some(g))
        }
    }
}

fn validate_job_name(name: &str) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::BadRequest("job name cannot be empty".into()));
    }
    if name.len() > MAX_JOB_NAME_LEN {
        return Err(AppError::BadRequest(format!(
            "job name exceeds {} character limit",
            MAX_JOB_NAME_LEN
        )));
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' || c == '.')
    {
        return Err(AppError::BadRequest(
            "job name may only contain alphanumeric characters, spaces, hyphens, underscores, and dots".into(),
        ));
    }
    Ok(())
}
