use chrono::Utc;
use kronforce::dag::DagResolver;
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
        description: None,
        task: TaskType::Shell {
            command: "echo".to_string(),
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
    }
}

#[test]
fn test_no_cycle_simple() {
    let db = test_db();
    let job_a = make_job("a");
    let job_b = make_job("b");
    db.insert_job(&job_a).unwrap();
    db.insert_job(&job_b).unwrap();

    let dag = DagResolver::new(db);
    // job_c depends on job_a — no cycle
    let result = dag.validate_no_cycle(
        Uuid::new_v4(),
        &[Dependency {
            job_id: job_a.id,
            within_secs: None,
        }],
    );
    assert!(result.is_ok());
}

#[test]
fn test_cycle_detection() {
    let db = test_db();

    let mut job_a = make_job("cycle-a");
    let job_b = make_job("cycle-b");

    // a depends on b
    job_a.depends_on = vec![Dependency {
        job_id: job_b.id,
        within_secs: None,
    }];
    db.insert_job(&job_a).unwrap();
    db.insert_job(&job_b).unwrap();

    let dag = DagResolver::new(db.clone());

    // Now try to make b depend on a — should detect cycle
    let result = dag.validate_no_cycle(
        job_b.id,
        &[Dependency {
            job_id: job_a.id,
            within_secs: None,
        }],
    );
    assert!(result.is_err());
}

#[test]
fn test_empty_dependencies() {
    let db = test_db();
    let dag = DagResolver::new(db);
    let result = dag.validate_no_cycle(Uuid::new_v4(), &[]);
    assert!(result.is_ok());
}

// --- deps_satisfied tests ---

#[test]
fn test_deps_satisfied_empty_deps() {
    let db = test_db();
    let dag = DagResolver::new(db);
    // No dependencies means always satisfied
    assert!(dag.deps_satisfied(&[]).unwrap());
}

#[test]
fn test_deps_satisfied_no_execution_exists() {
    let db = test_db();
    let job = make_job("dep-target");
    db.insert_job(&job).unwrap();

    let dag = DagResolver::new(db);
    let deps = vec![Dependency {
        job_id: job.id,
        within_secs: None,
    }];
    // No execution exists, so deps are not satisfied
    assert!(!dag.deps_satisfied(&deps).unwrap());
}

#[test]
fn test_deps_satisfied_with_successful_execution() {
    let db = test_db();
    let job = make_job("dep-target-ok");
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
        triggered_by: TriggerSource::Scheduler,
        extracted: None,
    };
    db.insert_execution(&exec).unwrap();

    let dag = DagResolver::new(db);
    let deps = vec![Dependency {
        job_id: job.id,
        within_secs: None,
    }];
    assert!(dag.deps_satisfied(&deps).unwrap());
}

#[test]
fn test_deps_satisfied_with_failed_execution() {
    let db = test_db();
    let job = make_job("dep-target-fail");
    db.insert_job(&job).unwrap();

    let exec = ExecutionRecord {
        id: Uuid::new_v4(),
        job_id: job.id,
        agent_id: None,
        task_snapshot: None,
        status: ExecutionStatus::Failed,
        exit_code: Some(1),
        stdout: String::new(),
        stderr: "error".to_string(),
        stdout_truncated: false,
        stderr_truncated: false,
        started_at: Some(Utc::now()),
        finished_at: Some(Utc::now()),
        triggered_by: TriggerSource::Scheduler,
        extracted: None,
    };
    db.insert_execution(&exec).unwrap();

    let dag = DagResolver::new(db);
    let deps = vec![Dependency {
        job_id: job.id,
        within_secs: None,
    }];
    // Failed execution should not satisfy dependency
    assert!(!dag.deps_satisfied(&deps).unwrap());
}

#[test]
fn test_deps_satisfied_multiple_deps_all_ok() {
    let db = test_db();
    let job_a = make_job("multi-dep-a");
    let job_b = make_job("multi-dep-b");
    db.insert_job(&job_a).unwrap();
    db.insert_job(&job_b).unwrap();

    for job in [&job_a, &job_b] {
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
            triggered_by: TriggerSource::Scheduler,
            extracted: None,
        };
        db.insert_execution(&exec).unwrap();
    }

    let dag = DagResolver::new(db);
    let deps = vec![
        Dependency {
            job_id: job_a.id,
            within_secs: None,
        },
        Dependency {
            job_id: job_b.id,
            within_secs: None,
        },
    ];
    assert!(dag.deps_satisfied(&deps).unwrap());
}

#[test]
fn test_deps_satisfied_multiple_deps_one_failed() {
    let db = test_db();
    let job_a = make_job("partial-dep-a");
    let job_b = make_job("partial-dep-b");
    db.insert_job(&job_a).unwrap();
    db.insert_job(&job_b).unwrap();

    // job_a succeeded
    db.insert_execution(&ExecutionRecord {
        id: Uuid::new_v4(),
        job_id: job_a.id,
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
        triggered_by: TriggerSource::Scheduler,
        extracted: None,
    })
    .unwrap();

    // job_b failed
    db.insert_execution(&ExecutionRecord {
        id: Uuid::new_v4(),
        job_id: job_b.id,
        agent_id: None,
        task_snapshot: None,
        status: ExecutionStatus::Failed,
        exit_code: Some(1),
        stdout: String::new(),
        stderr: String::new(),
        stdout_truncated: false,
        stderr_truncated: false,
        started_at: Some(Utc::now()),
        finished_at: Some(Utc::now()),
        triggered_by: TriggerSource::Scheduler,
        extracted: None,
    })
    .unwrap();

    let dag = DagResolver::new(db);
    let deps = vec![
        Dependency {
            job_id: job_a.id,
            within_secs: None,
        },
        Dependency {
            job_id: job_b.id,
            within_secs: None,
        },
    ];
    // One failed, so not satisfied
    assert!(!dag.deps_satisfied(&deps).unwrap());
}

// --- validate_no_cycle with complex graph structures ---

#[test]
fn test_no_cycle_diamond_graph() {
    // A -> B, A -> C, B -> D, C -> D (diamond shape, no cycle)
    let db = test_db();
    let job_a = make_job("diamond-a");
    let mut job_b = make_job("diamond-b");
    let mut job_c = make_job("diamond-c");
    let job_d = make_job("diamond-d");

    job_b.depends_on = vec![Dependency {
        job_id: job_a.id,
        within_secs: None,
    }];
    job_c.depends_on = vec![Dependency {
        job_id: job_a.id,
        within_secs: None,
    }];

    db.insert_job(&job_a).unwrap();
    db.insert_job(&job_b).unwrap();
    db.insert_job(&job_c).unwrap();
    db.insert_job(&job_d).unwrap();

    let dag = DagResolver::new(db);
    // D depends on both B and C -- no cycle
    let result = dag.validate_no_cycle(
        job_d.id,
        &[
            Dependency {
                job_id: job_b.id,
                within_secs: None,
            },
            Dependency {
                job_id: job_c.id,
                within_secs: None,
            },
        ],
    );
    assert!(result.is_ok());
}

#[test]
fn test_cycle_detection_three_nodes() {
    // A -> B -> C, then trying to make C -> A creates a cycle
    let db = test_db();
    let job_a = make_job("tri-a");
    let mut job_b = make_job("tri-b");
    let mut job_c = make_job("tri-c");

    job_b.depends_on = vec![Dependency {
        job_id: job_a.id,
        within_secs: None,
    }];
    job_c.depends_on = vec![Dependency {
        job_id: job_b.id,
        within_secs: None,
    }];

    db.insert_job(&job_a).unwrap();
    db.insert_job(&job_b).unwrap();
    db.insert_job(&job_c).unwrap();

    let dag = DagResolver::new(db);
    // Try to make A depend on C -- creates cycle A -> B -> C -> A
    let result = dag.validate_no_cycle(
        job_a.id,
        &[Dependency {
            job_id: job_c.id,
            within_secs: None,
        }],
    );
    assert!(result.is_err());
}

#[test]
fn test_self_dependency_cycle() {
    // Job depending on itself
    let db = test_db();
    let job = make_job("self-dep");
    db.insert_job(&job).unwrap();

    let dag = DagResolver::new(db);
    let result = dag.validate_no_cycle(
        job.id,
        &[Dependency {
            job_id: job.id,
            within_secs: None,
        }],
    );
    assert!(result.is_err());
}

#[test]
fn test_dependency_on_nonexistent_job() {
    let db = test_db();
    let job = make_job("existing");
    db.insert_job(&job).unwrap();

    let dag = DagResolver::new(db);
    let result = dag.validate_no_cycle(
        job.id,
        &[Dependency {
            job_id: Uuid::new_v4(), // does not exist
            within_secs: None,
        }],
    );
    assert!(result.is_err());
}

#[test]
fn test_no_cycle_chain_of_four() {
    // A -> B -> C -> D (linear chain, no cycle)
    let db = test_db();
    let job_a = make_job("chain-a");
    let mut job_b = make_job("chain-b");
    let mut job_c = make_job("chain-c");
    let job_d = make_job("chain-d");

    job_b.depends_on = vec![Dependency {
        job_id: job_a.id,
        within_secs: None,
    }];
    job_c.depends_on = vec![Dependency {
        job_id: job_b.id,
        within_secs: None,
    }];

    db.insert_job(&job_a).unwrap();
    db.insert_job(&job_b).unwrap();
    db.insert_job(&job_c).unwrap();
    db.insert_job(&job_d).unwrap();

    let dag = DagResolver::new(db);
    let result = dag.validate_no_cycle(
        job_d.id,
        &[Dependency {
            job_id: job_c.id,
            within_secs: None,
        }],
    );
    assert!(result.is_ok());
}
