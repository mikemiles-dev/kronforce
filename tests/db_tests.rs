mod common;
use common::*;

use chrono::Utc;
use kronforce::db::models::*;
use uuid::Uuid;

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
        forward_url: None,
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
        email_output: Some("failure".to_string()),
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

    let mut exec = make_execution(job.id, ExecutionStatus::Succeeded);
    exec.task_snapshot = Some(job.task.clone());
    exec.stdout = "hello".to_string();
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

    let exec = make_execution(job.id, ExecutionStatus::Succeeded);
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
        let exec = make_execution(job.id, status);
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
        expires_at: None,
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
        expires_at: None,
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
            working_dir: None,
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
                working_dir: None,
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
    let exec = make_execution(job.id, ExecutionStatus::Succeeded);
    db.insert_execution(&exec).unwrap();

    // Should succeed now (was failing with FK constraint before fix)
    db.delete_job(job.id).unwrap();

    assert!(db.get_job(job.id).unwrap().is_none());
    assert!(db.get_execution(exec.id).unwrap().is_none());
}

// --- Concurrency Controls ---

#[test]
fn test_max_concurrent_round_trip() {
    let db = test_db();
    let mut job = make_job("concurrent-job");
    job.max_concurrent = 3;
    db.insert_job(&job).unwrap();

    let fetched = db.get_job(job.id).unwrap().unwrap();
    assert_eq!(fetched.max_concurrent, 3);
}

#[test]
fn test_count_running_executions_for_job() {
    let db = test_db();
    let job = make_job("count-test");
    db.insert_job(&job).unwrap();

    // No executions
    assert_eq!(db.count_running_executions_for_job(job.id).unwrap(), 0);

    // Insert a running execution
    let exec1 = make_execution(job.id, ExecutionStatus::Running);
    db.insert_execution(&exec1).unwrap();
    assert_eq!(db.count_running_executions_for_job(job.id).unwrap(), 1);

    // Insert a pending execution
    let mut exec2 = make_execution(job.id, ExecutionStatus::Pending);
    exec2.started_at = None;
    exec2.finished_at = None;
    db.insert_execution(&exec2).unwrap();
    assert_eq!(db.count_running_executions_for_job(job.id).unwrap(), 2);

    // Insert a succeeded execution — should NOT count
    let exec3 = make_execution(job.id, ExecutionStatus::Succeeded);
    db.insert_execution(&exec3).unwrap();
    assert_eq!(db.count_running_executions_for_job(job.id).unwrap(), 2);
}

// --- Parameterized Runs ---

#[test]
fn test_job_parameters_round_trip() {
    let db = test_db();
    let mut job = make_job("param-job");
    job.parameters = Some(vec![
        JobParameter {
            name: "version".to_string(),
            param_type: "text".to_string(),
            required: true,
            default: Some("1.0".to_string()),
            options: None,
            description: Some("Release version".to_string()),
        },
        JobParameter {
            name: "env".to_string(),
            param_type: "select".to_string(),
            required: false,
            default: Some("staging".to_string()),
            options: Some(vec!["staging".to_string(), "production".to_string()]),
            description: None,
        },
    ]);
    db.insert_job(&job).unwrap();

    let fetched = db.get_job(job.id).unwrap().unwrap();
    let params = fetched.parameters.unwrap();
    assert_eq!(params.len(), 2);
    assert_eq!(params[0].name, "version");
    assert!(params[0].required);
    assert_eq!(params[1].param_type, "select");
    assert_eq!(params[1].options.as_ref().unwrap().len(), 2);
}

#[test]
fn test_execution_params_round_trip() {
    let db = test_db();
    let job = make_job("exec-params");
    db.insert_job(&job).unwrap();

    let mut exec = ExecutionRecord::new(Uuid::new_v4(), job.id, TriggerSource::Api);
    exec.params = Some(serde_json::json!({"version": "2.0", "env": "production"}));
    db.insert_execution(&exec).unwrap();

    let fetched = db.get_execution(exec.id).unwrap().unwrap();
    let params = fetched.params.unwrap();
    assert_eq!(params["version"], "2.0");
    assert_eq!(params["env"], "production");
}

// --- Webhook Triggers ---

#[test]
fn test_webhook_token_round_trip() {
    let db = test_db();
    let mut job = make_job("webhook-job");
    job.webhook_token = Some("abc123def456".to_string());
    db.insert_job(&job).unwrap();

    let fetched = db.get_job(job.id).unwrap().unwrap();
    assert_eq!(fetched.webhook_token.as_deref(), Some("abc123def456"));
}

#[test]
fn test_get_job_by_webhook_token() {
    let db = test_db();
    let mut job = make_job("webhook-lookup");
    job.webhook_token = Some("token123".to_string());
    db.insert_job(&job).unwrap();

    let found = db.get_job_by_webhook_token("token123").unwrap().unwrap();
    assert_eq!(found.id, job.id);
    assert_eq!(found.name, "webhook-lookup");

    // Non-existent token
    assert!(db.get_job_by_webhook_token("bogus").unwrap().is_none());
}

#[test]
fn test_set_webhook_token() {
    let db = test_db();
    let job = make_job("webhook-set");
    db.insert_job(&job).unwrap();

    // Initially no token
    let fetched = db.get_job(job.id).unwrap().unwrap();
    assert!(fetched.webhook_token.is_none());

    // Set token
    db.set_webhook_token(job.id, Some("newtoken")).unwrap();
    let fetched = db.get_job(job.id).unwrap().unwrap();
    assert_eq!(fetched.webhook_token.as_deref(), Some("newtoken"));

    // Lookup by token works
    let found = db.get_job_by_webhook_token("newtoken").unwrap().unwrap();
    assert_eq!(found.id, job.id);

    // Clear token
    db.set_webhook_token(job.id, None).unwrap();
    let fetched = db.get_job(job.id).unwrap().unwrap();
    assert!(fetched.webhook_token.is_none());
    assert!(db.get_job_by_webhook_token("newtoken").unwrap().is_none());
}

// --- Approval + Params ---

#[test]
fn test_pending_approval_execution_stores_params() {
    let db = test_db();
    let job = make_job("approval-params");
    db.insert_job(&job).unwrap();

    let mut exec = ExecutionRecord::new(Uuid::new_v4(), job.id, TriggerSource::Api);
    exec.status = ExecutionStatus::PendingApproval;
    exec.params = Some(serde_json::json!({"version": "3.0", "env": "staging"}));
    db.insert_execution(&exec).unwrap();

    let fetched = db.get_execution(exec.id).unwrap().unwrap();
    assert_eq!(fetched.status, ExecutionStatus::PendingApproval);
    let params = fetched.params.unwrap();
    assert_eq!(params["version"], "3.0");
    assert_eq!(params["env"], "staging");
}

// --- Count Running Edge Cases ---

#[test]
fn test_count_running_excludes_terminal_statuses() {
    let db = test_db();
    let job = make_job("count-edge");
    db.insert_job(&job).unwrap();

    // Insert executions with various terminal statuses — none should count
    for status in [
        ExecutionStatus::Succeeded,
        ExecutionStatus::Failed,
        ExecutionStatus::TimedOut,
        ExecutionStatus::Cancelled,
        ExecutionStatus::Skipped,
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
            triggered_by: TriggerSource::Api,
            extracted: None,
            retry_of: None,
            attempt_number: 1,
            params: None,
        };
        db.insert_execution(&exec).unwrap();
    }

    assert_eq!(db.count_running_executions_for_job(job.id).unwrap(), 0);
}

#[test]
fn test_count_running_different_jobs_isolated() {
    let db = test_db();
    let job_a = make_job("count-iso-a");
    let job_b = make_job("count-iso-b");
    db.insert_job(&job_a).unwrap();
    db.insert_job(&job_b).unwrap();

    // Running execution on job_a
    let exec = ExecutionRecord {
        id: Uuid::new_v4(),
        job_id: job_a.id,
        agent_id: None,
        task_snapshot: None,
        status: ExecutionStatus::Running,
        exit_code: None,
        stdout: String::new(),
        stderr: String::new(),
        stdout_truncated: false,
        stderr_truncated: false,
        started_at: Some(Utc::now()),
        finished_at: None,
        triggered_by: TriggerSource::Api,
        extracted: None,
        retry_of: None,
        attempt_number: 1,
        params: None,
    };
    db.insert_execution(&exec).unwrap();

    assert_eq!(db.count_running_executions_for_job(job_a.id).unwrap(), 1);
    assert_eq!(db.count_running_executions_for_job(job_b.id).unwrap(), 0);
}

// --- Schedule Window ---

#[test]
fn test_schedule_window_round_trip() {
    let db = test_db();
    let mut job = make_job("window-job");
    let starts = chrono::Utc::now() + chrono::Duration::hours(1);
    let expires = chrono::Utc::now() + chrono::Duration::days(7);
    job.starts_at = Some(starts);
    job.expires_at = Some(expires);
    db.insert_job(&job).unwrap();

    let fetched = db.get_job(job.id).unwrap().unwrap();
    // Compare timestamps within 1 second (rfc3339 precision)
    assert!((fetched.starts_at.unwrap() - starts).num_seconds().abs() < 2);
    assert!((fetched.expires_at.unwrap() - expires).num_seconds().abs() < 2);
}

// --- Email Output Config ---

#[test]
fn test_email_output_config_round_trip() {
    let db = test_db();
    let mut job = make_job("email-output-job");
    job.notifications = Some(JobNotificationConfig {
        on_failure: true,
        on_success: false,
        on_assertion_failure: false,
        recipients: None,
        email_output: Some("always".to_string()),
    });
    db.insert_job(&job).unwrap();

    let fetched = db.get_job(job.id).unwrap().unwrap();
    let notif = fetched.notifications.unwrap();
    assert_eq!(notif.email_output.as_deref(), Some("always"));
    assert!(notif.on_failure);
}

// --- Output Forward URL ---

#[test]
fn test_forward_url_round_trip() {
    let db = test_db();
    let mut job = make_job("forward-job");
    job.output_rules = Some(OutputRules {
        extractions: vec![],
        triggers: vec![],
        assertions: vec![],
        forward_url: Some("https://hooks.example.com/output".to_string()),
    });
    db.insert_job(&job).unwrap();

    let fetched = db.get_job(job.id).unwrap().unwrap();
    let rules = fetched.output_rules.unwrap();
    assert_eq!(
        rules.forward_url.as_deref(),
        Some("https://hooks.example.com/output")
    );
}

// --- TriggerSource::Webhook in executions ---

#[test]
fn test_webhook_trigger_source_round_trip() {
    let db = test_db();
    let job = make_job("webhook-trigger-src");
    db.insert_job(&job).unwrap();

    let mut exec = ExecutionRecord::new(
        Uuid::new_v4(),
        job.id,
        TriggerSource::Webhook {
            token_prefix: "abcd1234".to_string(),
        },
    );
    exec.status = ExecutionStatus::Succeeded;
    db.insert_execution(&exec).unwrap();

    let fetched = db.get_execution(exec.id).unwrap().unwrap();
    if let TriggerSource::Webhook { token_prefix } = fetched.triggered_by {
        assert_eq!(token_prefix, "abcd1234");
    } else {
        panic!("expected Webhook trigger source");
    }
}

// --- Calendar Schedule ---

#[test]
fn test_calendar_schedule_round_trip() {
    let db = test_db();
    let mut job = make_job("cal-job");
    job.schedule = ScheduleKind::Calendar(CalendarSchedule {
        anchor: "last_day".to_string(),
        offset_days: -2,
        nth: None,
        weekday: None,
        hour: 9,
        minute: 30,
        months: vec![1, 4, 7, 10],
        skip_weekends: true,
        holidays: vec!["2026-12-25".to_string()],
    });
    db.insert_job(&job).unwrap();

    let fetched = db.get_job(job.id).unwrap().unwrap();
    if let ScheduleKind::Calendar(cal) = &fetched.schedule {
        assert_eq!(cal.anchor, "last_day");
        assert_eq!(cal.offset_days, -2);
        assert_eq!(cal.hour, 9);
        assert_eq!(cal.minute, 30);
        assert_eq!(cal.months, vec![1, 4, 7, 10]);
        assert!(cal.skip_weekends);
        assert_eq!(cal.holidays, vec!["2026-12-25"]);
    } else {
        panic!("expected Calendar schedule");
    }
}

#[test]
fn test_interval_schedule_round_trip() {
    let db = test_db();
    let mut job = make_job("interval-job");
    job.schedule = ScheduleKind::Interval {
        interval_secs: 1800,
    };
    db.insert_job(&job).unwrap();

    let fetched = db.get_job(job.id).unwrap().unwrap();
    if let ScheduleKind::Interval { interval_secs } = &fetched.schedule {
        assert_eq!(*interval_secs, 1800);
    } else {
        panic!("expected Interval schedule");
    }
}

#[test]
fn test_timezone_round_trip() {
    let db = test_db();
    let mut job = make_job("tz-job");
    job.timezone = Some("America/New_York".to_string());
    db.insert_job(&job).unwrap();

    let fetched = db.get_job(job.id).unwrap().unwrap();
    assert_eq!(fetched.timezone.as_deref(), Some("America/New_York"));
}

#[test]
fn test_nth_weekday_schedule() {
    let db = test_db();
    let mut job = make_job("nth-wd-job");
    job.schedule = ScheduleKind::Calendar(CalendarSchedule {
        anchor: "nth_weekday".to_string(),
        offset_days: 0,
        nth: Some(2),
        weekday: Some("tuesday".to_string()),
        hour: 10,
        minute: 0,
        months: vec![],
        skip_weekends: false,
        holidays: vec![],
    });
    db.insert_job(&job).unwrap();

    let fetched = db.get_job(job.id).unwrap().unwrap();
    if let ScheduleKind::Calendar(cal) = &fetched.schedule {
        assert_eq!(cal.anchor, "nth_weekday");
        assert_eq!(cal.nth, Some(2));
        assert_eq!(cal.weekday.as_deref(), Some("tuesday"));
    } else {
        panic!("expected Calendar schedule");
    }
}

// --- Pipeline Schedules (settings-based) ---

#[test]
fn test_pipeline_schedule_crud() {
    let db = test_db();
    let group = "ETL";
    let key = format!("pipeline_schedule_{}", group);

    // Initially no schedule
    assert!(db.get_setting(&key).unwrap().is_none());

    // Set a cron schedule
    let cron_sched = serde_json::json!({"type": "cron", "value": "0 0 * * * *"});
    db.set_setting(&key, &cron_sched.to_string()).unwrap();

    let val = db.get_setting(&key).unwrap().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&val).unwrap();
    assert_eq!(parsed["type"], "cron");
    assert_eq!(parsed["value"], "0 0 * * * *");

    // Update to interval
    let interval_sched = serde_json::json!({"type": "interval", "value": {"interval_secs": 3600}});
    db.set_setting(&key, &interval_sched.to_string()).unwrap();

    let val = db.get_setting(&key).unwrap().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&val).unwrap();
    assert_eq!(parsed["type"], "interval");
    assert_eq!(parsed["value"]["interval_secs"], 3600);

    // Delete schedule
    db.delete_setting(&key).unwrap();
    assert!(db.get_setting(&key).unwrap().is_none());
}

#[test]
fn test_pipeline_schedule_deserialization() {
    let db = test_db();
    let key = "pipeline_schedule_TestGroup";

    // Store a cron schedule and verify it deserializes to ScheduleKind
    let sched = serde_json::json!({"type": "cron", "value": "0 */5 * * * *"});
    db.set_setting(key, &sched.to_string()).unwrap();

    let val = db.get_setting(key).unwrap().unwrap();
    let parsed: ScheduleKind = serde_json::from_str(&val).unwrap();
    if let ScheduleKind::Cron(expr) = parsed {
        assert_eq!(expr.0, "0 */5 * * * *");
    } else {
        panic!("expected Cron schedule kind");
    }
}

#[test]
fn test_pipeline_schedule_interval_deserialization() {
    let db = test_db();
    let key = "pipeline_schedule_Batch";

    let sched = serde_json::json!({"type": "interval", "value": {"interval_secs": 1800}});
    db.set_setting(key, &sched.to_string()).unwrap();

    let val = db.get_setting(key).unwrap().unwrap();
    let parsed: ScheduleKind = serde_json::from_str(&val).unwrap();
    if let ScheduleKind::Interval { interval_secs } = parsed {
        assert_eq!(interval_secs, 1800);
    } else {
        panic!("expected Interval schedule kind");
    }
}

#[test]
fn test_pipeline_schedules_enumeration() {
    let db = test_db();

    // Set schedules for two groups
    db.set_setting(
        "pipeline_schedule_ETL",
        &serde_json::json!({"type": "cron", "value": "0 0 * * * *"}).to_string(),
    )
    .unwrap();
    db.set_setting(
        "pipeline_schedule_Batch",
        &serde_json::json!({"type": "interval", "value": {"interval_secs": 600}}).to_string(),
    )
    .unwrap();

    // Enumerate all pipeline schedules via get_all_settings
    let all = db.get_all_settings().unwrap();
    let pipeline_scheds: Vec<_> = all
        .iter()
        .filter(|(k, _)| k.starts_with("pipeline_schedule_"))
        .collect();

    assert!(pipeline_scheds.len() >= 2);

    // Verify we can extract group names
    let groups: Vec<String> = pipeline_scheds
        .iter()
        .filter_map(|(k, _)| k.strip_prefix("pipeline_schedule_").map(String::from))
        .collect();
    assert!(groups.contains(&"ETL".to_string()));
    assert!(groups.contains(&"Batch".to_string()));
}

// --- Connections ---

#[test]
fn test_connection_crud() {
    let db = test_db();
    let conn = kronforce::db::models::Connection {
        name: "test-postgres".to_string(),
        conn_type: kronforce::db::models::ConnectionType::Postgres,
        description: Some("Test DB".to_string()),
        config: serde_json::json!({"connection_string": "postgresql://user:pass@localhost/test"}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    db.insert_connection(&conn).unwrap();

    let fetched = db.get_connection("test-postgres").unwrap().unwrap();
    assert_eq!(fetched.name, "test-postgres");
    assert_eq!(
        fetched.conn_type,
        kronforce::db::models::ConnectionType::Postgres
    );
    assert_eq!(fetched.description.as_deref(), Some("Test DB"));
    assert_eq!(
        fetched.config["connection_string"],
        "postgresql://user:pass@localhost/test"
    );
}

#[test]
fn test_connection_list() {
    let db = test_db();
    for name in ["conn-a", "conn-b", "conn-c"] {
        let conn = kronforce::db::models::Connection {
            name: name.to_string(),
            conn_type: kronforce::db::models::ConnectionType::Redis,
            description: None,
            config: serde_json::json!({"url": "redis://localhost"}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        db.insert_connection(&conn).unwrap();
    }
    let all = db.list_connections().unwrap();
    assert_eq!(all.len(), 3);
}

#[test]
fn test_connection_update() {
    let db = test_db();
    let conn = kronforce::db::models::Connection {
        name: "update-me".to_string(),
        conn_type: kronforce::db::models::ConnectionType::Http,
        description: Some("old".to_string()),
        config: serde_json::json!({"base_url": "https://old.example.com"}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    db.insert_connection(&conn).unwrap();

    let mut updated = conn.clone();
    updated.description = Some("new".to_string());
    updated.config = serde_json::json!({"base_url": "https://new.example.com"});
    db.update_connection("update-me", &updated).unwrap();

    let fetched = db.get_connection("update-me").unwrap().unwrap();
    assert_eq!(fetched.description.as_deref(), Some("new"));
    assert_eq!(fetched.config["base_url"], "https://new.example.com");
}

#[test]
fn test_connection_delete() {
    let db = test_db();
    let conn = kronforce::db::models::Connection {
        name: "delete-me".to_string(),
        conn_type: kronforce::db::models::ConnectionType::Kafka,
        description: None,
        config: serde_json::json!({"broker": "localhost:9092"}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    db.insert_connection(&conn).unwrap();

    assert!(db.delete_connection("delete-me").unwrap());
    assert!(db.get_connection("delete-me").unwrap().is_none());
    assert!(!db.delete_connection("delete-me").unwrap());
}

#[test]
fn test_connection_not_found() {
    let db = test_db();
    assert!(db.get_connection("nonexistent").unwrap().is_none());
}

// --- Data Export ---

#[test]
fn test_export_includes_jobs_and_variables() {
    let db = test_db();
    let job = make_job("export-job");
    db.insert_job(&job).unwrap();

    let var = kronforce::db::models::Variable {
        name: "EXPORT_VAR".to_string(),
        value: "test-value".to_string(),
        updated_at: Utc::now(),
        secret: false,
        expires_at: None,
    };
    db.insert_variable(&var).unwrap();

    let jobs = db.list_jobs(None, None, None, 100, 0).unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].name, "export-job");

    let vars = db.list_variables().unwrap();
    assert!(
        vars.iter()
            .any(|v| v.name == "EXPORT_VAR" && v.value == "test-value")
    );
}

#[test]
fn test_export_secret_variable_decrypted() {
    let db = test_db();
    let var = kronforce::db::models::Variable {
        name: "SECRET_KEY".to_string(),
        value: "super-secret-123".to_string(),
        updated_at: Utc::now(),
        secret: true,
        expires_at: None,
    };
    db.insert_variable(&var).unwrap();

    // list_variables returns decrypted values (encryption not enabled in tests)
    let vars = db.list_variables().unwrap();
    let secret = vars.iter().find(|v| v.name == "SECRET_KEY").unwrap();
    assert_eq!(secret.value, "super-secret-123");
    assert!(secret.secret);
}

#[test]
fn test_export_connections_decrypted() {
    let db = test_db();
    let conn = kronforce::db::models::Connection {
        name: "export-db".to_string(),
        conn_type: kronforce::db::models::ConnectionType::Postgres,
        description: Some("Export test".to_string()),
        config: serde_json::json!({"connection_string": "postgresql://user:secret_pw@host/db"}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    db.insert_connection(&conn).unwrap();

    let conns = db.list_connections().unwrap();
    assert_eq!(conns.len(), 1);
    assert_eq!(
        conns[0].config["connection_string"],
        "postgresql://user:secret_pw@host/db"
    );
}

#[test]
fn test_export_api_keys_metadata() {
    let db = test_db();
    let (raw, prefix) = kronforce::api::generate_api_key();
    let hash = kronforce::api::hash_api_key(&raw);

    let key = kronforce::db::models::ApiKey {
        id: Uuid::new_v4(),
        key_prefix: prefix,
        key_hash: hash,
        name: "export-key".to_string(),
        role: kronforce::db::models::ApiKeyRole::Operator,
        created_at: Utc::now(),
        last_used_at: None,
        active: true,
        allowed_groups: Some(vec!["ETL".to_string()]),
        ip_allowlist: None,
        expires_at: None,
    };
    db.insert_api_key(&key).unwrap();

    let keys = db.list_api_keys().unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].name, "export-key");
    assert_eq!(keys[0].role, kronforce::db::models::ApiKeyRole::Operator);
    assert_eq!(keys[0].allowed_groups, Some(vec!["ETL".to_string()]));
}

#[test]
fn test_export_settings_included() {
    let db = test_db();
    db.set_setting("retention_days", "30").unwrap();
    db.set_setting(
        "pipeline_schedule_ETL",
        r#"{"type":"cron","value":"0 0 6 * * *"}"#,
    )
    .unwrap();

    let settings = db.get_all_settings().unwrap();
    assert_eq!(settings.get("retention_days").unwrap(), "30");
    assert!(settings.contains_key("pipeline_schedule_ETL"));
}

#[test]
fn test_export_groups_included() {
    let db = test_db();
    db.add_custom_group("ExportGroup").unwrap();
    let mut job = make_job("grouped-export");
    job.group = Some("ETL".to_string());
    db.insert_job(&job).unwrap();

    let groups = db.get_distinct_groups().unwrap();
    assert!(groups.contains(&"Default".to_string()));
    assert!(groups.contains(&"ETL".to_string()));
    assert!(groups.contains(&"ExportGroup".to_string()));
}
