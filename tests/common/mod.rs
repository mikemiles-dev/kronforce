#![allow(dead_code)]
/// Shared test fixtures — centralizes Job, ExecutionRecord, and Db construction
/// so that adding a new field only requires updating this one file.
use chrono::Utc;
use kronforce::db::Db;
use kronforce::db::models::*;
use uuid::Uuid;

pub fn test_db() -> Db {
    let db = Db::open(":memory:").unwrap();
    db.migrate().unwrap();
    db
}

pub fn make_job(name: &str) -> Job {
    Job {
        id: Uuid::new_v4(),
        name: name.to_string(),
        description: Some("test job".to_string()),
        task: TaskType::Shell {
            command: format!("echo {}", name),
            working_dir: None,
        },
        run_as: None,
        schedule: ScheduleKind::OnDemand,
        status: JobStatus::Scheduled,
        timeout_secs: None,
        depends_on: vec![],
        target: None,
        created_by: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        output_rules: None,
        notifications: None,
        group: None,
        retry_max: 0,
        retry_delay_secs: 0,
        retry_backoff: 1.0,
        approval_required: false,
        priority: 0,
        sla_deadline: None,
        sla_warning_mins: 0,
        starts_at: None,
        expires_at: None,
        max_concurrent: 0,
        parameters: None,
        webhook_token: None,
        timezone: None,
    }
}

pub fn make_job_with_status(name: &str, status: JobStatus) -> Job {
    let mut job = make_job(name);
    job.status = status;
    job
}

pub fn make_execution(job_id: Uuid, status: ExecutionStatus) -> ExecutionRecord {
    ExecutionRecord {
        id: Uuid::new_v4(),
        job_id,
        agent_id: None,
        task_snapshot: None,
        status,
        exit_code: if status == ExecutionStatus::Succeeded {
            Some(0)
        } else {
            None
        },
        stdout: String::new(),
        stderr: String::new(),
        stdout_truncated: false,
        stderr_truncated: false,
        started_at: Some(Utc::now()),
        finished_at: if status != ExecutionStatus::Running {
            Some(Utc::now())
        } else {
            None
        },
        triggered_by: TriggerSource::Api,
        extracted: None,
        retry_of: None,
        attempt_number: 1,
        params: None,
    }
}
