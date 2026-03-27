use kronforce::db::Db;
use kronforce::dag::DagResolver;
use kronforce::models::*;
use chrono::Utc;
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
        task: TaskType::Shell { command: "echo".to_string() },
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
        &[Dependency { job_id: job_a.id, within_secs: None }],
    );
    assert!(result.is_ok());
}

#[test]
fn test_cycle_detection() {
    let db = test_db();

    let mut job_a = make_job("cycle-a");
    let mut job_b = make_job("cycle-b");

    // a depends on b
    job_a.depends_on = vec![Dependency { job_id: job_b.id, within_secs: None }];
    db.insert_job(&job_a).unwrap();
    db.insert_job(&job_b).unwrap();

    let dag = DagResolver::new(db.clone());

    // Now try to make b depend on a — should detect cycle
    let result = dag.validate_no_cycle(
        job_b.id,
        &[Dependency { job_id: job_a.id, within_secs: None }],
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
