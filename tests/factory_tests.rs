use chrono::{TimeZone, Utc};
use kronforce::models::*;
use uuid::Uuid;

// --- ExecutionRecord::new() tests ---

#[test]
fn test_execution_record_new_defaults() {
    let id = Uuid::new_v4();
    let job_id = Uuid::new_v4();
    let rec = ExecutionRecord::new(id, job_id, TriggerSource::Scheduler);

    assert_eq!(rec.id, id);
    assert_eq!(rec.job_id, job_id);
    assert!(rec.agent_id.is_none());
    assert!(rec.task_snapshot.is_none());
    assert_eq!(rec.status, ExecutionStatus::Pending);
    assert!(rec.exit_code.is_none());
    assert_eq!(rec.stdout, "");
    assert_eq!(rec.stderr, "");
    assert!(!rec.stdout_truncated);
    assert!(!rec.stderr_truncated);
    assert!(rec.started_at.is_none());
    assert!(rec.finished_at.is_none());
    assert!(rec.extracted.is_none());
}

#[test]
fn test_execution_record_new_preserves_trigger_scheduler() {
    let rec = ExecutionRecord::new(Uuid::new_v4(), Uuid::new_v4(), TriggerSource::Scheduler);
    if let TriggerSource::Scheduler = rec.triggered_by {
        // OK
    } else {
        panic!("expected Scheduler trigger");
    }
}

#[test]
fn test_execution_record_new_preserves_trigger_api() {
    let rec = ExecutionRecord::new(Uuid::new_v4(), Uuid::new_v4(), TriggerSource::Api);
    if let TriggerSource::Api = rec.triggered_by {
        // OK
    } else {
        panic!("expected Api trigger");
    }
}

#[test]
fn test_execution_record_new_preserves_trigger_dependency() {
    let parent_id = Uuid::new_v4();
    let rec = ExecutionRecord::new(
        Uuid::new_v4(),
        Uuid::new_v4(),
        TriggerSource::Dependency {
            parent_execution_id: parent_id,
        },
    );
    if let TriggerSource::Dependency {
        parent_execution_id,
    } = rec.triggered_by
    {
        assert_eq!(parent_execution_id, parent_id);
    } else {
        panic!("expected Dependency trigger");
    }
}

// --- Builder chain tests ---

#[test]
fn test_with_status() {
    let rec = ExecutionRecord::new(Uuid::new_v4(), Uuid::new_v4(), TriggerSource::Api)
        .with_status(ExecutionStatus::Running);
    assert_eq!(rec.status, ExecutionStatus::Running);
}

#[test]
fn test_with_agent_id() {
    let agent_id = Uuid::new_v4();
    let rec = ExecutionRecord::new(Uuid::new_v4(), Uuid::new_v4(), TriggerSource::Api)
        .with_agent_id(agent_id);
    assert_eq!(rec.agent_id, Some(agent_id));
}

#[test]
fn test_with_task_snapshot() {
    let task = TaskType::Shell {
        command: "echo hello".to_string(),
    };
    let rec = ExecutionRecord::new(Uuid::new_v4(), Uuid::new_v4(), TriggerSource::Api)
        .with_task_snapshot(task);
    assert!(rec.task_snapshot.is_some());
    if let Some(TaskType::Shell { command }) = &rec.task_snapshot {
        assert_eq!(command, "echo hello");
    } else {
        panic!("expected Shell task snapshot");
    }
}

#[test]
fn test_with_started_at() {
    let now = Utc::now();
    let rec = ExecutionRecord::new(Uuid::new_v4(), Uuid::new_v4(), TriggerSource::Api)
        .with_started_at(now);
    assert_eq!(rec.started_at, Some(now));
}

#[test]
fn test_builder_chain_full() {
    let agent_id = Uuid::new_v4();
    let start_time = Utc.with_ymd_and_hms(2025, 6, 1, 12, 0, 0).unwrap();
    let task = TaskType::Http {
        method: HttpMethod::Get,
        url: "https://example.com".to_string(),
        headers: None,
        body: None,
        expect_status: Some(200),
    };

    let rec = ExecutionRecord::new(Uuid::new_v4(), Uuid::new_v4(), TriggerSource::Scheduler)
        .with_status(ExecutionStatus::Running)
        .with_agent_id(agent_id)
        .with_task_snapshot(task)
        .with_started_at(start_time);

    assert_eq!(rec.status, ExecutionStatus::Running);
    assert_eq!(rec.agent_id, Some(agent_id));
    assert!(rec.task_snapshot.is_some());
    assert_eq!(rec.started_at, Some(start_time));
    // Defaults should still hold
    assert!(rec.exit_code.is_none());
    assert_eq!(rec.stdout, "");
    assert!(rec.finished_at.is_none());
}

// --- ApiKey::bootstrap() tests ---

#[test]
fn test_api_key_bootstrap_auto_generates() {
    let (key, raw) = ApiKey::bootstrap(ApiKeyRole::Admin, "test-key", None);
    assert!(raw.starts_with("kf_"));
    assert_eq!(key.name, "test-key");
    assert_eq!(key.role, ApiKeyRole::Admin);
    assert!(key.active);
    assert!(key.last_used_at.is_none());
    assert!(!key.key_hash.is_empty());
    assert!(key.key_prefix.starts_with("kf_"));
}

#[test]
fn test_api_key_bootstrap_with_preset() {
    let preset = "my_custom_key_1234567890abcdef".to_string();
    let (key, raw) = ApiKey::bootstrap(ApiKeyRole::Operator, "preset-key", Some(preset.clone()));
    assert_eq!(raw, preset);
    // prefix should be first KEY_PREFIX_LEN (11) chars
    assert_eq!(key.key_prefix, "my_custom_k");
    assert_eq!(key.role, ApiKeyRole::Operator);
}

#[test]
fn test_api_key_bootstrap_with_short_preset() {
    // Preset shorter than KEY_PREFIX_LEN (11) should not panic
    let preset = "short".to_string();
    let (key, raw) = ApiKey::bootstrap(ApiKeyRole::Viewer, "short-key", Some(preset.clone()));
    assert_eq!(raw, "short");
    assert_eq!(key.key_prefix, "short");
}

#[test]
fn test_api_key_bootstrap_with_empty_preset_generates_key() {
    // Empty string preset should be filtered out and auto-generate
    let (_, raw) = ApiKey::bootstrap(ApiKeyRole::Agent, "auto-key", Some(String::new()));
    assert!(raw.starts_with("kf_"));
}

#[test]
fn test_api_key_bootstrap_uniqueness() {
    let (_, raw1) = ApiKey::bootstrap(ApiKeyRole::Admin, "key1", None);
    let (_, raw2) = ApiKey::bootstrap(ApiKeyRole::Admin, "key2", None);
    assert_ne!(raw1, raw2);
}

#[test]
fn test_api_key_bootstrap_different_roles() {
    for role in [
        ApiKeyRole::Admin,
        ApiKeyRole::Operator,
        ApiKeyRole::Viewer,
        ApiKeyRole::Agent,
    ] {
        let (key, _) = ApiKey::bootstrap(role, "test", None);
        assert_eq!(key.role, role);
    }
}

#[test]
fn test_api_key_bootstrap_hash_is_deterministic_for_same_key() {
    let preset = "deterministic_key_test_123".to_string();
    let (key1, _) = ApiKey::bootstrap(ApiKeyRole::Admin, "k1", Some(preset.clone()));
    let (key2, _) = ApiKey::bootstrap(ApiKeyRole::Admin, "k2", Some(preset));
    assert_eq!(key1.key_hash, key2.key_hash);
}
