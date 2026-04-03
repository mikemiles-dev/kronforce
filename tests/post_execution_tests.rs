use chrono::Utc;
use kronforce::db::Db;
use kronforce::db::models::*;
use kronforce::executor::output_rules::process_post_execution;
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
        description: None,
        task: TaskType::Shell {
            command: "echo test".to_string(),
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
    }
}

fn insert_execution(db: &Db, job_id: Uuid) -> Uuid {
    let exec_id = Uuid::new_v4();
    let exec = ExecutionRecord {
        id: exec_id,
        job_id,
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
    exec_id
}

// --- No output rules ---

#[test]
fn test_no_output_rules_returns_empty() {
    let db = test_db();
    let job = make_job("no-rules");
    db.insert_job(&job).unwrap();
    let exec_id = insert_execution(&db, job.id);

    let events = process_post_execution(
        &db,
        &job,
        exec_id,
        "some output",
        "",
        ExecutionStatus::Succeeded,
    );
    assert!(events.is_empty());
}

// --- Extraction rules ---

#[test]
fn test_extraction_stores_extracted_values() {
    let db = test_db();
    let mut job = make_job("extract-job");
    job.output_rules = Some(OutputRules {
        extractions: vec![ExtractionRule {
            name: "duration".to_string(),
            pattern: r"took (\d+)ms".to_string(),
            rule_type: "regex".to_string(),
            write_to_variable: None,
            target: "variable".to_string(),
        }],
        triggers: vec![],
        assertions: vec![],
    });
    db.insert_job(&job).unwrap();
    let exec_id = insert_execution(&db, job.id);

    let events = process_post_execution(
        &db,
        &job,
        exec_id,
        "Processing took 150ms total",
        "",
        ExecutionStatus::Succeeded,
    );
    assert!(events.is_empty());

    // Verify extracted values were stored
    let fetched = db.get_execution(exec_id).unwrap().unwrap();
    let extracted = fetched.extracted.unwrap();
    assert_eq!(extracted["duration"], "150");
}

#[test]
fn test_extraction_jsonpath() {
    let db = test_db();
    let mut job = make_job("extract-json");
    job.output_rules = Some(OutputRules {
        extractions: vec![ExtractionRule {
            name: "count".to_string(),
            pattern: "$.results.count".to_string(),
            rule_type: "jsonpath".to_string(),
            write_to_variable: None,
            target: "variable".to_string(),
        }],
        triggers: vec![],
        assertions: vec![],
    });
    db.insert_job(&job).unwrap();
    let exec_id = insert_execution(&db, job.id);

    process_post_execution(
        &db,
        &job,
        exec_id,
        r#"{"results": {"count": 42}}"#,
        "",
        ExecutionStatus::Succeeded,
    );

    let fetched = db.get_execution(exec_id).unwrap().unwrap();
    let extracted = fetched.extracted.unwrap();
    assert_eq!(extracted["count"], "42");
}

// --- write_to_variable ---

#[test]
fn test_extraction_write_to_variable() {
    let db = test_db();
    let mut job = make_job("extract-var");
    job.output_rules = Some(OutputRules {
        extractions: vec![ExtractionRule {
            name: "version".to_string(),
            pattern: r"version: (\S+)".to_string(),
            rule_type: "regex".to_string(),
            write_to_variable: Some("CURRENT_VERSION".to_string()),
            target: "variable".to_string(),
        }],
        triggers: vec![],
        assertions: vec![],
    });
    db.insert_job(&job).unwrap();
    let exec_id = insert_execution(&db, job.id);

    process_post_execution(
        &db,
        &job,
        exec_id,
        "version: 2.5.1",
        "",
        ExecutionStatus::Succeeded,
    );

    // Verify the variable was upserted
    let var = db.get_variable("CURRENT_VERSION").unwrap().unwrap();
    assert_eq!(var.value, "2.5.1");
}

#[test]
fn test_extraction_write_to_variable_updates_existing() {
    let db = test_db();

    // Pre-set the variable
    db.upsert_variable("MY_VAR", "old_value").unwrap();

    let mut job = make_job("extract-var-update");
    job.output_rules = Some(OutputRules {
        extractions: vec![ExtractionRule {
            name: "status".to_string(),
            pattern: r"status=(\w+)".to_string(),
            rule_type: "regex".to_string(),
            write_to_variable: Some("MY_VAR".to_string()),
            target: "variable".to_string(),
        }],
        triggers: vec![],
        assertions: vec![],
    });
    db.insert_job(&job).unwrap();
    let exec_id = insert_execution(&db, job.id);

    process_post_execution(
        &db,
        &job,
        exec_id,
        "status=healthy",
        "",
        ExecutionStatus::Succeeded,
    );

    let var = db.get_variable("MY_VAR").unwrap().unwrap();
    assert_eq!(var.value, "healthy");
}

// --- Assertions ---

#[test]
fn test_assertion_pass_does_not_fail_execution() {
    let db = test_db();
    let mut job = make_job("assert-pass");
    job.output_rules = Some(OutputRules {
        extractions: vec![],
        triggers: vec![],
        assertions: vec![OutputAssertion {
            pattern: "OK".to_string(),
            message: None,
        }],
    });
    db.insert_job(&job).unwrap();
    let exec_id = insert_execution(&db, job.id);

    let events = process_post_execution(
        &db,
        &job,
        exec_id,
        "Status: OK",
        "",
        ExecutionStatus::Succeeded,
    );
    assert!(events.is_empty());

    // Execution should still be succeeded
    let fetched = db.get_execution(exec_id).unwrap().unwrap();
    assert_eq!(fetched.status, ExecutionStatus::Succeeded);
}

#[test]
fn test_assertion_failure_marks_execution_failed() {
    let db = test_db();
    let mut job = make_job("assert-fail");
    job.output_rules = Some(OutputRules {
        extractions: vec![],
        triggers: vec![],
        assertions: vec![OutputAssertion {
            pattern: "SUCCESS".to_string(),
            message: Some("expected SUCCESS in output".to_string()),
        }],
    });
    db.insert_job(&job).unwrap();
    let exec_id = insert_execution(&db, job.id);

    process_post_execution(
        &db,
        &job,
        exec_id,
        "FAILURE detected",
        "",
        ExecutionStatus::Succeeded,
    );

    // Execution should be marked as failed
    let fetched = db.get_execution(exec_id).unwrap().unwrap();
    assert_eq!(fetched.status, ExecutionStatus::Failed);
    assert!(fetched.stderr.contains("assertion failed"));
}

#[test]
fn test_assertion_not_run_on_failed_execution() {
    let db = test_db();
    let mut job = make_job("assert-skip");
    job.output_rules = Some(OutputRules {
        extractions: vec![],
        triggers: vec![],
        assertions: vec![OutputAssertion {
            pattern: "NEVER_FOUND".to_string(),
            message: Some("should not trigger".to_string()),
        }],
    });
    db.insert_job(&job).unwrap();
    let exec_id = insert_execution(&db, job.id);

    // Pass ExecutionStatus::Failed -- assertions should be skipped
    process_post_execution(
        &db,
        &job,
        exec_id,
        "some output",
        "",
        ExecutionStatus::Failed,
    );

    // Execution status should remain succeeded (as inserted), not changed by assertion
    let fetched = db.get_execution(exec_id).unwrap().unwrap();
    // It was inserted as Succeeded and assertion should NOT have run
    assert_eq!(fetched.status, ExecutionStatus::Succeeded);
}

// --- Triggers ---

#[test]
fn test_trigger_generates_events() {
    let db = test_db();
    let mut job = make_job("trigger-job");
    job.output_rules = Some(OutputRules {
        extractions: vec![],
        triggers: vec![OutputTrigger {
            pattern: "ERROR".to_string(),
            severity: "error".to_string(),
        }],
        assertions: vec![],
    });
    db.insert_job(&job).unwrap();
    let exec_id = insert_execution(&db, job.id);

    let events = process_post_execution(
        &db,
        &job,
        exec_id,
        "ERROR: disk full",
        "",
        ExecutionStatus::Succeeded,
    );

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "output.matched");
    assert_eq!(events[0].severity, EventSeverity::Error);
    assert!(events[0].message.contains("ERROR"));
    assert_eq!(events[0].job_id, Some(job.id));
}

#[test]
fn test_trigger_matches_stderr() {
    let db = test_db();
    let mut job = make_job("trigger-stderr");
    job.output_rules = Some(OutputRules {
        extractions: vec![],
        triggers: vec![OutputTrigger {
            pattern: "WARN".to_string(),
            severity: "warning".to_string(),
        }],
        assertions: vec![],
    });
    db.insert_job(&job).unwrap();
    let exec_id = insert_execution(&db, job.id);

    let events = process_post_execution(
        &db,
        &job,
        exec_id,
        "",
        "WARN: memory low",
        ExecutionStatus::Succeeded,
    );

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].severity, EventSeverity::Warning);
}

#[test]
fn test_trigger_no_match_no_events() {
    let db = test_db();
    let mut job = make_job("trigger-nomatch");
    job.output_rules = Some(OutputRules {
        extractions: vec![],
        triggers: vec![OutputTrigger {
            pattern: "CRITICAL".to_string(),
            severity: "error".to_string(),
        }],
        assertions: vec![],
    });
    db.insert_job(&job).unwrap();
    let exec_id = insert_execution(&db, job.id);

    let events = process_post_execution(
        &db,
        &job,
        exec_id,
        "everything is fine",
        "",
        ExecutionStatus::Succeeded,
    );

    assert!(events.is_empty());
}

#[test]
fn test_trigger_multiple_patterns() {
    let db = test_db();
    let mut job = make_job("trigger-multi");
    job.output_rules = Some(OutputRules {
        extractions: vec![],
        triggers: vec![
            OutputTrigger {
                pattern: "ERROR".to_string(),
                severity: "error".to_string(),
            },
            OutputTrigger {
                pattern: "WARNING".to_string(),
                severity: "warning".to_string(),
            },
        ],
        assertions: vec![],
    });
    db.insert_job(&job).unwrap();
    let exec_id = insert_execution(&db, job.id);

    let events = process_post_execution(
        &db,
        &job,
        exec_id,
        "ERROR and WARNING in output",
        "",
        ExecutionStatus::Succeeded,
    );

    assert_eq!(events.len(), 2);
}

#[test]
fn test_trigger_events_stored_in_db() {
    let db = test_db();
    let mut job = make_job("trigger-stored");
    job.output_rules = Some(OutputRules {
        extractions: vec![],
        triggers: vec![OutputTrigger {
            pattern: "ALERT".to_string(),
            severity: "info".to_string(),
        }],
        assertions: vec![],
    });
    db.insert_job(&job).unwrap();
    let exec_id = insert_execution(&db, job.id);

    process_post_execution(
        &db,
        &job,
        exec_id,
        "ALERT: new deployment",
        "",
        ExecutionStatus::Succeeded,
    );

    // Verify events are in the DB
    let db_events = db.list_events(None, 10, 0).unwrap();
    assert_eq!(db_events.len(), 1);
    assert_eq!(db_events[0].kind, "output.matched");
}

// --- Combined rules ---

#[test]
fn test_combined_extraction_and_trigger() {
    let db = test_db();
    let mut job = make_job("combined-rules");
    job.output_rules = Some(OutputRules {
        extractions: vec![ExtractionRule {
            name: "count".to_string(),
            pattern: r"processed (\d+)".to_string(),
            rule_type: "regex".to_string(),
            write_to_variable: None,
            target: "variable".to_string(),
        }],
        triggers: vec![OutputTrigger {
            pattern: "ERROR".to_string(),
            severity: "error".to_string(),
        }],
        assertions: vec![],
    });
    db.insert_job(&job).unwrap();
    let exec_id = insert_execution(&db, job.id);

    let events = process_post_execution(
        &db,
        &job,
        exec_id,
        "processed 100 records, then ERROR occurred",
        "",
        ExecutionStatus::Succeeded,
    );

    // Should have 1 trigger event
    assert_eq!(events.len(), 1);

    // Should have extracted value
    let fetched = db.get_execution(exec_id).unwrap().unwrap();
    let extracted = fetched.extracted.unwrap();
    assert_eq!(extracted["count"], "100");
}
