mod common;
use common::*;

use kronforce::db::models::*;

// --- count_jobs with filters ---

#[test]
fn test_count_jobs_no_filter() {
    let db = test_db();
    db.insert_job(&make_job_with_status("job-1", JobStatus::Scheduled))
        .unwrap();
    db.insert_job(&make_job_with_status("job-2", JobStatus::Paused))
        .unwrap();
    db.insert_job(&make_job_with_status("job-3", JobStatus::Unscheduled))
        .unwrap();

    let count = db.count_jobs(None, None, None).unwrap();
    assert_eq!(count, 3);
}

#[test]
fn test_count_jobs_status_filter_scheduled() {
    let db = test_db();
    db.insert_job(&make_job_with_status("active-1", JobStatus::Scheduled))
        .unwrap();
    db.insert_job(&make_job_with_status("active-2", JobStatus::Scheduled))
        .unwrap();
    db.insert_job(&make_job_with_status("paused-1", JobStatus::Paused))
        .unwrap();

    let count = db.count_jobs(Some("scheduled"), None, None).unwrap();
    assert_eq!(count, 2);
}

#[test]
fn test_count_jobs_status_filter_paused() {
    let db = test_db();
    db.insert_job(&make_job_with_status("active-1", JobStatus::Scheduled))
        .unwrap();
    db.insert_job(&make_job_with_status("paused-1", JobStatus::Paused))
        .unwrap();
    db.insert_job(&make_job_with_status("paused-2", JobStatus::Paused))
        .unwrap();

    let count = db.count_jobs(Some("paused"), None, None).unwrap();
    assert_eq!(count, 2);
}

#[test]
fn test_count_jobs_search_filter() {
    let db = test_db();
    db.insert_job(&make_job_with_status("deploy-web", JobStatus::Scheduled))
        .unwrap();
    db.insert_job(&make_job_with_status("deploy-api", JobStatus::Scheduled))
        .unwrap();
    db.insert_job(&make_job_with_status("backup-db", JobStatus::Scheduled))
        .unwrap();

    let count = db.count_jobs(None, Some("deploy"), None).unwrap();
    assert_eq!(count, 2);
}

#[test]
fn test_count_jobs_combined_filters() {
    let db = test_db();
    db.insert_job(&make_job_with_status("deploy-web", JobStatus::Scheduled))
        .unwrap();
    db.insert_job(&make_job_with_status("deploy-api", JobStatus::Paused))
        .unwrap();
    db.insert_job(&make_job_with_status("backup-db", JobStatus::Scheduled))
        .unwrap();

    let count = db
        .count_jobs(Some("scheduled"), Some("deploy"), None)
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_count_jobs_no_match() {
    let db = test_db();
    db.insert_job(&make_job_with_status("job-a", JobStatus::Scheduled))
        .unwrap();

    let count = db.count_jobs(None, Some("nonexistent"), None).unwrap();
    assert_eq!(count, 0);
}

// --- list_jobs with filters ---

#[test]
fn test_list_jobs_no_filter() {
    let db = test_db();
    db.insert_job(&make_job_with_status("alpha", JobStatus::Scheduled))
        .unwrap();
    db.insert_job(&make_job_with_status("beta", JobStatus::Paused))
        .unwrap();

    let jobs = db.list_jobs(None, None, None, 100, 0).unwrap();
    assert_eq!(jobs.len(), 2);
}

#[test]
fn test_list_jobs_status_filter() {
    let db = test_db();
    db.insert_job(&make_job_with_status("active-job", JobStatus::Scheduled))
        .unwrap();
    db.insert_job(&make_job_with_status("paused-job", JobStatus::Paused))
        .unwrap();

    let jobs = db.list_jobs(Some("paused"), None, None, 100, 0).unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].name, "paused-job");
}

#[test]
fn test_list_jobs_search_filter() {
    let db = test_db();
    db.insert_job(&make_job_with_status("deploy-prod", JobStatus::Scheduled))
        .unwrap();
    db.insert_job(&make_job_with_status(
        "deploy-staging",
        JobStatus::Scheduled,
    ))
    .unwrap();
    db.insert_job(&make_job_with_status("backup-daily", JobStatus::Scheduled))
        .unwrap();

    let jobs = db.list_jobs(None, Some("deploy"), None, 100, 0).unwrap();
    assert_eq!(jobs.len(), 2);
}

#[test]
fn test_list_jobs_pagination_limit() {
    let db = test_db();
    for i in 0..5 {
        db.insert_job(&make_job_with_status(
            &format!("job-{:02}", i),
            JobStatus::Scheduled,
        ))
        .unwrap();
    }

    let page = db.list_jobs(None, None, None, 2, 0).unwrap();
    assert_eq!(page.len(), 2);
}

#[test]
fn test_list_jobs_pagination_offset() {
    let db = test_db();
    for i in 0..5 {
        db.insert_job(&make_job_with_status(
            &format!("job-{:02}", i),
            JobStatus::Scheduled,
        ))
        .unwrap();
    }

    let page1 = db.list_jobs(None, None, None, 3, 0).unwrap();
    let page2 = db.list_jobs(None, None, None, 3, 3).unwrap();
    assert_eq!(page1.len(), 3);
    assert_eq!(page2.len(), 2);
    // No overlap
    assert_ne!(page1[0].name, page2[0].name);
}

#[test]
fn test_list_jobs_ordered_by_name() {
    let db = test_db();
    db.insert_job(&make_job_with_status("charlie", JobStatus::Scheduled))
        .unwrap();
    db.insert_job(&make_job_with_status("alpha", JobStatus::Scheduled))
        .unwrap();
    db.insert_job(&make_job_with_status("bravo", JobStatus::Scheduled))
        .unwrap();

    let jobs = db.list_jobs(None, None, None, 100, 0).unwrap();
    assert_eq!(jobs[0].name, "alpha");
    assert_eq!(jobs[1].name, "bravo");
    assert_eq!(jobs[2].name, "charlie");
}

#[test]
fn test_list_jobs_search_matches_task_json() {
    let db = test_db();
    // The task_json contains the command, so searching for "echo" should match
    db.insert_job(&make_job_with_status("my-job", JobStatus::Scheduled))
        .unwrap();

    let jobs = db.list_jobs(None, Some("echo"), None, 100, 0).unwrap();
    assert_eq!(jobs.len(), 1);
}

#[test]
fn test_list_jobs_empty_result() {
    let db = test_db();
    let jobs = db.list_jobs(None, None, None, 100, 0).unwrap();
    assert!(jobs.is_empty());
}
