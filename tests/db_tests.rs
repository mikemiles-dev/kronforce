use chrono::Utc;
use kronforce::db::Db;
use kronforce::db::models::*;
use uuid::Uuid;

fn test_db() -> Db {
    let db = Db::open(":memory:").unwrap();
    db.migrate().unwrap();
    db
}

fn make_job(name: &str) -> Job {
    Job {
        id: Uuid::new_v4(),
        name: name.to_string(),
        description: Some("test job".to_string()),
        task: TaskType::Shell {
            command: "echo hello".to_string(),
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
    }
}

// --- Job CRUD ---

#[test]
fn test_insert_and_get_job() {
    let db = test_db();
    let job = make_job("test-job");
    db.insert_job(&job).unwrap();

    let fetched = db.get_job(job.id).unwrap().unwrap();
    assert_eq!(fetched.name, "test-job");
    assert_eq!(fetched.description.as_deref(), Some("test job"));
}

#[test]
fn test_duplicate_job_name_rejected() {
    let db = test_db();
    let job1 = make_job("duplicate");
    db.insert_job(&job1).unwrap();

    let job2 = make_job("duplicate");
    let result = db.insert_job(&job2);
    assert!(result.is_err());
}

#[test]
fn test_update_job() {
    let db = test_db();
    let mut job = make_job("updatable");
    db.insert_job(&job).unwrap();

    job.description = Some("updated".to_string());
    job.updated_at = Utc::now();
    db.update_job(&job).unwrap();

    let fetched = db.get_job(job.id).unwrap().unwrap();
    assert_eq!(fetched.description.as_deref(), Some("updated"));
}

#[test]
fn test_delete_job() {
    let db = test_db();
    let job = make_job("deletable");
    db.insert_job(&job).unwrap();
    db.delete_job(job.id).unwrap();

    let fetched = db.get_job(job.id).unwrap();
    assert!(fetched.is_none());
}

#[test]
fn test_list_jobs() {
    let db = test_db();
    db.insert_job(&make_job("job-a")).unwrap();
    db.insert_job(&make_job("job-b")).unwrap();
    db.insert_job(&make_job("job-c")).unwrap();

    let jobs = db.list_jobs(None, None, None, 100, 0).unwrap();
    assert_eq!(jobs.len(), 3);
}

#[test]
fn test_list_jobs_pagination() {
    let db = test_db();
    for i in 0..10 {
        db.insert_job(&make_job(&format!("job-{}", i))).unwrap();
    }

    let page1 = db.list_jobs(None, None, None, 3, 0).unwrap();
    assert_eq!(page1.len(), 3);

    let page2 = db.list_jobs(None, None, None, 3, 3).unwrap();
    assert_eq!(page2.len(), 3);
}

#[test]
fn test_count_jobs() {
    let db = test_db();
    db.insert_job(&make_job("count-a")).unwrap();
    db.insert_job(&make_job("count-b")).unwrap();

    let count = db.count_jobs(None, None, None).unwrap();
    assert_eq!(count, 2);
}

#[test]
fn test_count_jobs_with_status_filter() {
    let db = test_db();
    let mut job = make_job("paused-job");
    job.status = JobStatus::Paused;
    db.insert_job(&job).unwrap();
    db.insert_job(&make_job("active-job")).unwrap();

    let count = db.count_jobs(Some("scheduled"), None, None).unwrap();
    assert_eq!(count, 1);
}

// --- Job with output rules ---

#[test]
fn test_job_with_output_rules() {
    let db = test_db();
    let mut job = make_job("rules-job");
    job.output_rules = Some(OutputRules {
        extractions: vec![ExtractionRule {
            name: "duration".to_string(),
            pattern: r"took (\d+)ms".to_string(),
            rule_type: "regex".to_string(),
            write_to_variable: None,
            target: "variable".to_string(),
        }],
        triggers: vec![OutputTrigger {
            pattern: "ERROR".to_string(),
            severity: "error".to_string(),
        }],
        assertions: vec![OutputAssertion {
            pattern: "OK".to_string(),
            message: Some("expected OK".to_string()),
        }],
    });
    db.insert_job(&job).unwrap();

    let fetched = db.get_job(job.id).unwrap().unwrap();
    let rules = fetched.output_rules.unwrap();
    assert_eq!(rules.extractions.len(), 1);
    assert_eq!(rules.triggers.len(), 1);
    assert_eq!(rules.assertions.len(), 1);
    assert_eq!(rules.extractions[0].name, "duration");
}

// --- Job with notifications ---

#[test]
fn test_job_with_notifications() {
    let db = test_db();
    let mut job = make_job("notif-job");
    job.notifications = Some(JobNotificationConfig {
        on_failure: true,
        on_success: false,
        on_assertion_failure: true,
        recipients: None,
    });
    db.insert_job(&job).unwrap();

    let fetched = db.get_job(job.id).unwrap().unwrap();
    let notif = fetched.notifications.unwrap();
    assert!(notif.on_failure);
    assert!(!notif.on_success);
    assert!(notif.on_assertion_failure);
}

// --- Executions ---

#[test]
fn test_insert_and_get_execution() {
    let db = test_db();
    let job = make_job("exec-job");
    db.insert_job(&job).unwrap();

    let exec = ExecutionRecord {
        id: Uuid::new_v4(),
        job_id: job.id,
        agent_id: None,
        task_snapshot: Some(job.task.clone()),
        status: ExecutionStatus::Succeeded,
        exit_code: Some(0),
        stdout: "hello".to_string(),
        stderr: String::new(),
        stdout_truncated: false,
        stderr_truncated: false,
        started_at: Some(Utc::now()),
        finished_at: Some(Utc::now()),
        triggered_by: TriggerSource::Api,
        extracted: None,
        retry_of: None,
        attempt_number: 1,
    };
    db.insert_execution(&exec).unwrap();

    let fetched = db.get_execution(exec.id).unwrap().unwrap();
    assert_eq!(fetched.status, ExecutionStatus::Succeeded);
    assert_eq!(fetched.stdout, "hello");
}

#[test]
fn test_update_execution_extracted() {
    let db = test_db();
    let job = make_job("extract-job");
    db.insert_job(&job).unwrap();

    let exec = ExecutionRecord {
        id: Uuid::new_v4(),
        job_id: job.id,
        agent_id: None,
        task_snapshot: None,
        status: ExecutionStatus::Succeeded,
        exit_code: Some(0),
        stdout: String::new(),
        stderr: String::new(),
        stdout_truncated: false,
        stderr_truncated: false,
        started_at: Some(Utc::now()),
        finished_at: Some(Utc::now()),
        triggered_by: TriggerSource::Api,
        extracted: None,
        retry_of: None,
        attempt_number: 1,
    };
    db.insert_execution(&exec).unwrap();
    db.update_execution_extracted(exec.id, &serde_json::json!({"key": "value"}))
        .unwrap();

    let fetched = db.get_execution(exec.id).unwrap().unwrap();
    assert!(fetched.extracted.is_some());
}

#[test]
fn test_execution_counts() {
    let db = test_db();
    let job = make_job("counts-job");
    db.insert_job(&job).unwrap();

    for status in [
        ExecutionStatus::Succeeded,
        ExecutionStatus::Succeeded,
        ExecutionStatus::Failed,
    ] {
        let exec = ExecutionRecord {
            id: Uuid::new_v4(),
            job_id: job.id,
            agent_id: None,
            task_snapshot: None,
            status,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            started_at: Some(Utc::now()),
            finished_at: Some(Utc::now()),
            triggered_by: TriggerSource::Scheduler,
            extracted: None,
            retry_of: None,
            attempt_number: 1,
        };
        db.insert_execution(&exec).unwrap();
    }

    let (total, succeeded, failed) = db.get_execution_counts(job.id).unwrap();
    assert_eq!(total, 3);
    assert_eq!(succeeded, 2);
    assert_eq!(failed, 1);
}

// --- Agents ---

#[test]
fn test_upsert_and_get_agent() {
    let db = test_db();
    let agent = Agent {
        id: Uuid::new_v4(),
        name: "test-agent".to_string(),
        tags: vec!["linux".to_string()],
        hostname: "host1".to_string(),
        address: "127.0.0.1".to_string(),
        port: 8081,
        agent_type: AgentType::Standard,
        status: AgentStatus::Online,
        last_heartbeat: Some(Utc::now()),
        registered_at: Utc::now(),
        task_types: vec![],
    };
    db.upsert_agent(&agent).unwrap();

    let fetched = db.get_agent(agent.id).unwrap().unwrap();
    assert_eq!(fetched.name, "test-agent");
    assert_eq!(fetched.status, AgentStatus::Online);
}

#[test]
fn test_agent_re_registration_updates() {
    let db = test_db();
    let mut agent = Agent {
        id: Uuid::new_v4(),
        name: "reregister".to_string(),
        tags: vec![],
        hostname: "host1".to_string(),
        address: "127.0.0.1".to_string(),
        port: 8081,
        agent_type: AgentType::Standard,
        status: AgentStatus::Online,
        last_heartbeat: Some(Utc::now()),
        registered_at: Utc::now(),
        task_types: vec![],
    };
    db.upsert_agent(&agent).unwrap();

    agent.hostname = "host2".to_string();
    db.upsert_agent(&agent).unwrap();

    let fetched = db.get_agent_by_name("reregister").unwrap().unwrap();
    assert_eq!(fetched.hostname, "host2");
}

#[test]
fn test_list_agents() {
    let db = test_db();
    for i in 0..3 {
        let agent = Agent {
            id: Uuid::new_v4(),
            name: format!("agent-{}", i),
            tags: vec![],
            hostname: "host".to_string(),
            address: "127.0.0.1".to_string(),
            port: 8081 + i as u16,
            agent_type: AgentType::Standard,
            status: AgentStatus::Online,
            last_heartbeat: Some(Utc::now()),
            registered_at: Utc::now(),
            task_types: vec![],
        };
        db.upsert_agent(&agent).unwrap();
    }

    let agents = db.list_agents().unwrap();
    assert_eq!(agents.len(), 3);
}

#[test]
fn test_get_online_agents_by_type() {
    let db = test_db();
    let standard = Agent {
        id: Uuid::new_v4(),
        name: "std-agent".to_string(),
        tags: vec![],
        hostname: "h".to_string(),
        address: "127.0.0.1".to_string(),
        port: 8081,
        agent_type: AgentType::Standard,
        status: AgentStatus::Online,
        last_heartbeat: Some(Utc::now()),
        registered_at: Utc::now(),
        task_types: vec![],
    };
    let custom = Agent {
        id: Uuid::new_v4(),
        name: "custom-agent".to_string(),
        tags: vec![],
        hostname: "h".to_string(),
        address: "127.0.0.1".to_string(),
        port: 8082,
        agent_type: AgentType::Custom,
        status: AgentStatus::Online,
        last_heartbeat: Some(Utc::now()),
        registered_at: Utc::now(),
        task_types: vec![],
    };
    db.upsert_agent(&standard).unwrap();
    db.upsert_agent(&custom).unwrap();

    let std_agents = db.get_online_agents_by_type(AgentType::Standard).unwrap();
    assert_eq!(std_agents.len(), 1);
    assert_eq!(std_agents[0].name, "std-agent");

    let cust_agents = db.get_online_agents_by_type(AgentType::Custom).unwrap();
    assert_eq!(cust_agents.len(), 1);
    assert_eq!(cust_agents[0].name, "custom-agent");
}

// --- API Keys ---

#[test]
fn test_api_key_crud() {
    let db = test_db();
    let (raw, prefix) = kronforce::api::generate_api_key();
    let hash = kronforce::api::hash_api_key(&raw);

    let key = ApiKey {
        id: Uuid::new_v4(),
        key_prefix: prefix.clone(),
        key_hash: hash.clone(),
        name: "test-key".to_string(),
        role: ApiKeyRole::Admin,
        created_at: Utc::now(),
        last_used_at: None,
        active: true,
        allowed_groups: None,
        ip_allowlist: None,
    };
    db.insert_api_key(&key).unwrap();

    let fetched = db.get_api_key_by_hash(&hash).unwrap().unwrap();
    assert_eq!(fetched.name, "test-key");
    assert_eq!(fetched.role, ApiKeyRole::Admin);

    let keys = db.list_api_keys().unwrap();
    assert_eq!(keys.len(), 1);

    let count = db.count_api_keys().unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_api_key_agent_role() {
    let db = test_db();
    let (raw, prefix) = kronforce::api::generate_api_key();
    let hash = kronforce::api::hash_api_key(&raw);

    let key = ApiKey {
        id: Uuid::new_v4(),
        key_prefix: prefix,
        key_hash: hash.clone(),
        name: "agent-key".to_string(),
        role: ApiKeyRole::Agent,
        created_at: Utc::now(),
        last_used_at: None,
        active: true,
        allowed_groups: None,
        ip_allowlist: None,
    };
    db.insert_api_key(&key).unwrap();

    let fetched = db.get_api_key_by_hash(&hash).unwrap().unwrap();
    assert!(fetched.role.is_agent());
    assert!(!fetched.role.can_write());
    assert!(!fetched.role.can_manage_keys());
}

// --- Settings ---

#[test]
fn test_settings_crud() {
    let db = test_db();

    db.set_setting("test_key", "test_value").unwrap();
    let val = db.get_setting("test_key").unwrap().unwrap();
    assert_eq!(val, "test_value");

    db.set_setting("test_key", "updated").unwrap();
    let val = db.get_setting("test_key").unwrap().unwrap();
    assert_eq!(val, "updated");
}

#[test]
fn test_get_all_settings() {
    let db = test_db();
    db.set_setting("a", "1").unwrap();
    db.set_setting("b", "2").unwrap();

    let all = db.get_all_settings().unwrap();
    assert!(all.len() >= 2); // includes retention_days default
    assert_eq!(all.get("a").unwrap(), "1");
}

#[test]
fn test_get_nonexistent_setting() {
    let db = test_db();
    let val = db.get_setting("nonexistent").unwrap();
    assert!(val.is_none());
}

// --- Queue ---

#[test]
fn test_enqueue_and_dequeue() {
    let db = test_db();
    let agent_id = Uuid::new_v4();
    let exec_id = Uuid::new_v4();
    let job_id = Uuid::new_v4();

    db.enqueue_job(
        Uuid::new_v4(),
        exec_id,
        agent_id,
        job_id,
        &TaskType::Shell {
            command: "echo test".to_string(),
        },
        None,
        None,
        "http://callback",
    )
    .unwrap();

    let job = db.dequeue_job(agent_id).unwrap();
    assert!(job.is_some());

    let data = job.unwrap();
    assert_eq!(data["execution_id"].as_str().unwrap(), exec_id.to_string());
    assert_eq!(data["job_id"].as_str().unwrap(), job_id.to_string());

    // Second dequeue returns nothing (already claimed)
    let empty = db.dequeue_job(agent_id).unwrap();
    assert!(empty.is_none());
}

#[test]
fn test_queue_depth() {
    let db = test_db();
    let agent_id = Uuid::new_v4();

    for _ in 0..3 {
        db.enqueue_job(
            Uuid::new_v4(),
            Uuid::new_v4(),
            agent_id,
            Uuid::new_v4(),
            &TaskType::Shell {
                command: "echo".to_string(),
            },
            None,
            None,
            "http://cb",
        )
        .unwrap();
    }

    let depth = db.queue_depth(agent_id).unwrap();
    assert_eq!(depth, 3);
}

// --- Events ---

#[test]
fn test_log_event() {
    let db = test_db();
    db.log_event(
        "test.event",
        EventSeverity::Info,
        "test message",
        None,
        None,
    )
    .unwrap();

    let events = db.list_events(None, 10, 0).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "test.event");
    assert_eq!(events[0].message, "test message");
}

// --- Migrations ---

#[test]
fn test_fresh_migration() {
    let db = test_db();
    // If we got here, migration succeeded
    // Verify we can do basic operations
    let count = db.count_jobs(None, None, None).unwrap();
    assert_eq!(count, 0);
}

// --- Job Groups ---

#[test]
fn test_job_group_default_is_none() {
    let db = test_db();
    let job = make_job("no-group");
    db.insert_job(&job).unwrap();
    let fetched = db.get_job(job.id).unwrap().unwrap();
    // group is None in struct but from_row maps NULL to "Default"
    assert_eq!(fetched.group, Some("Default".to_string()));
}

#[test]
fn test_job_group_set_on_create() {
    let db = test_db();
    let mut job = make_job("grouped-job");
    job.group = Some("ETL".to_string());
    db.insert_job(&job).unwrap();
    let fetched = db.get_job(job.id).unwrap().unwrap();
    assert_eq!(fetched.group, Some("ETL".to_string()));
}

#[test]
fn test_job_group_update() {
    let db = test_db();
    let mut job = make_job("update-group");
    job.group = Some("ETL".to_string());
    db.insert_job(&job).unwrap();

    job.group = Some("Monitoring".to_string());
    job.updated_at = Utc::now();
    db.update_job(&job).unwrap();

    let fetched = db.get_job(job.id).unwrap().unwrap();
    assert_eq!(fetched.group, Some("Monitoring".to_string()));
}

#[test]
fn test_distinct_groups_includes_default() {
    let db = test_db();
    let groups = db.get_distinct_groups().unwrap();
    assert!(groups.contains(&"Default".to_string()));
}

#[test]
fn test_distinct_groups_from_jobs() {
    let db = test_db();
    let mut job1 = make_job("etl-1");
    job1.group = Some("ETL".to_string());
    db.insert_job(&job1).unwrap();

    let mut job2 = make_job("mon-1");
    job2.group = Some("Monitoring".to_string());
    db.insert_job(&job2).unwrap();

    let groups = db.get_distinct_groups().unwrap();
    assert!(groups.contains(&"Default".to_string()));
    assert!(groups.contains(&"ETL".to_string()));
    assert!(groups.contains(&"Monitoring".to_string()));
}

#[test]
fn test_distinct_groups_no_duplicates() {
    let db = test_db();
    let mut job1 = make_job("etl-1");
    job1.group = Some("ETL".to_string());
    db.insert_job(&job1).unwrap();

    let mut job2 = make_job("etl-2");
    job2.group = Some("ETL".to_string());
    db.insert_job(&job2).unwrap();

    let groups = db.get_distinct_groups().unwrap();
    let etl_count = groups.iter().filter(|g| *g == "ETL").count();
    assert_eq!(etl_count, 1);
}

#[test]
fn test_custom_group_persists() {
    let db = test_db();
    db.add_custom_group("EmptyGroup").unwrap();
    let groups = db.get_distinct_groups().unwrap();
    assert!(groups.contains(&"EmptyGroup".to_string()));
}

#[test]
fn test_custom_group_no_duplicate() {
    let db = test_db();
    db.add_custom_group("ETL").unwrap();
    db.add_custom_group("ETL").unwrap();
    let groups = db.get_distinct_groups().unwrap();
    let etl_count = groups.iter().filter(|g| *g == "ETL").count();
    assert_eq!(etl_count, 1);
}

#[test]
fn test_bulk_set_group() {
    let db = test_db();
    let job1 = make_job("bulk-1");
    let job2 = make_job("bulk-2");
    db.insert_job(&job1).unwrap();
    db.insert_job(&job2).unwrap();

    let count = db
        .bulk_set_group(&[job1.id, job2.id], Some("BatchGroup"))
        .unwrap();
    assert_eq!(count, 2);

    let fetched1 = db.get_job(job1.id).unwrap().unwrap();
    let fetched2 = db.get_job(job2.id).unwrap().unwrap();
    assert_eq!(fetched1.group, Some("BatchGroup".to_string()));
    assert_eq!(fetched2.group, Some("BatchGroup".to_string()));
}

#[test]
fn test_rename_group() {
    let db = test_db();
    let mut job1 = make_job("rename-1");
    job1.group = Some("OldName".to_string());
    let mut job2 = make_job("rename-2");
    job2.group = Some("OldName".to_string());
    let mut job3 = make_job("rename-other");
    job3.group = Some("Other".to_string());
    db.insert_job(&job1).unwrap();
    db.insert_job(&job2).unwrap();
    db.insert_job(&job3).unwrap();

    let count = db.rename_group("OldName", "NewName").unwrap();
    assert_eq!(count, 2);

    let fetched1 = db.get_job(job1.id).unwrap().unwrap();
    let fetched2 = db.get_job(job2.id).unwrap().unwrap();
    let fetched3 = db.get_job(job3.id).unwrap().unwrap();
    assert_eq!(fetched1.group, Some("NewName".to_string()));
    assert_eq!(fetched2.group, Some("NewName".to_string()));
    assert_eq!(fetched3.group, Some("Other".to_string()));
}

#[test]
fn test_rename_default_group_includes_null() {
    let db = test_db();
    // Job with NULL group (pre-migration style)
    let job = make_job("null-group");
    db.insert_job(&job).unwrap();

    let count = db.rename_group("Default", "NewDefault").unwrap();
    assert!(count >= 1);

    let fetched = db.get_job(job.id).unwrap().unwrap();
    assert_eq!(fetched.group, Some("NewDefault".to_string()));
}

#[test]
fn test_filter_jobs_by_group() {
    let db = test_db();
    let mut job1 = make_job("etl-job");
    job1.group = Some("ETL".to_string());
    let mut job2 = make_job("mon-job");
    job2.group = Some("Monitoring".to_string());
    db.insert_job(&job1).unwrap();
    db.insert_job(&job2).unwrap();

    let count = db.count_jobs(None, None, Some("ETL")).unwrap();
    assert_eq!(count, 1);

    let jobs = db.list_jobs(None, None, Some("ETL"), 100, 0).unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].name, "etl-job");
}

#[test]
fn test_filter_jobs_by_default_group() {
    let db = test_db();
    let job1 = make_job("default-job"); // group is None -> Default
    let mut job2 = make_job("etl-job");
    job2.group = Some("ETL".to_string());
    db.insert_job(&job1).unwrap();
    db.insert_job(&job2).unwrap();

    let count = db.count_jobs(None, None, Some("Default")).unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_delete_job_with_executions() {
    let db = test_db();
    let job = make_job("delete-with-execs");
    db.insert_job(&job).unwrap();

    // Create an execution for this job
    let exec = ExecutionRecord {
        id: Uuid::new_v4(),
        job_id: job.id,
        agent_id: None,
        task_snapshot: None,
        status: ExecutionStatus::Succeeded,
        exit_code: Some(0),
        stdout: String::new(),
        stderr: String::new(),
        stdout_truncated: false,
        stderr_truncated: false,
        started_at: Some(Utc::now()),
        finished_at: Some(Utc::now()),
        triggered_by: TriggerSource::Api,
        extracted: None,
        retry_of: None,
        attempt_number: 1,
    };
    db.insert_execution(&exec).unwrap();

    // Should succeed now (was failing with FK constraint before fix)
    db.delete_job(job.id).unwrap();

    assert!(db.get_job(job.id).unwrap().is_none());
    assert!(db.get_execution(exec.id).unwrap().is_none());
}
