use kronforce::db::models::ApiKeyRole;

#[test]
fn test_viewer_cannot_write() {
    let role = ApiKeyRole::Viewer;
    assert!(!role.can_write(), "viewer should not have write access");
    assert!(!role.can_manage_keys(), "viewer should not manage keys");
    assert!(!role.is_agent(), "viewer is not an agent");
}

#[test]
fn test_operator_can_write() {
    let role = ApiKeyRole::Operator;
    assert!(role.can_write(), "operator should have write access");
    assert!(!role.can_manage_keys(), "operator should not manage keys");
}

#[test]
fn test_admin_can_everything() {
    let role = ApiKeyRole::Admin;
    assert!(role.can_write(), "admin should have write access");
    assert!(role.can_manage_keys(), "admin should manage keys");
}

#[test]
fn test_agent_cannot_write() {
    let role = ApiKeyRole::Agent;
    assert!(!role.can_write(), "agent should not have write access");
    assert!(!role.can_manage_keys(), "agent should not manage keys");
    assert!(role.is_agent(), "agent should be agent");
}

#[test]
fn test_viewer_can_access_all_groups_by_default() {
    use chrono::Utc;
    use kronforce::db::models::ApiKey;
    use uuid::Uuid;

    let key = ApiKey {
        id: Uuid::new_v4(),
        key_prefix: "kf_test".to_string(),
        key_hash: "hash".to_string(),
        name: "test".to_string(),
        role: ApiKeyRole::Viewer,
        created_at: Utc::now(),
        last_used_at: None,
        active: true,
        allowed_groups: None,
        ip_allowlist: None,
        expires_at: None,
    };
    assert!(key.can_access_group(Some("ETL")));
    assert!(key.can_access_group(Some("Default")));
    assert!(key.can_access_group(None));
}

#[test]
fn test_scoped_key_restricts_groups() {
    use chrono::Utc;
    use kronforce::db::models::ApiKey;
    use uuid::Uuid;

    let key = ApiKey {
        id: Uuid::new_v4(),
        key_prefix: "kf_test".to_string(),
        key_hash: "hash".to_string(),
        name: "scoped".to_string(),
        role: ApiKeyRole::Operator,
        created_at: Utc::now(),
        last_used_at: None,
        active: true,
        allowed_groups: Some(vec!["ETL".to_string(), "Monitoring".to_string()]),
        ip_allowlist: None,
        expires_at: None,
    };
    assert!(key.can_access_group(Some("ETL")));
    assert!(key.can_access_group(Some("Monitoring")));
    assert!(!key.can_access_group(Some("Deploys")));
    assert!(!key.can_access_group(Some("Default")));
}

#[test]
fn test_admin_bypasses_group_scoping() {
    use chrono::Utc;
    use kronforce::db::models::ApiKey;
    use uuid::Uuid;

    let key = ApiKey {
        id: Uuid::new_v4(),
        key_prefix: "kf_test".to_string(),
        key_hash: "hash".to_string(),
        name: "admin".to_string(),
        role: ApiKeyRole::Admin,
        created_at: Utc::now(),
        last_used_at: None,
        active: true,
        allowed_groups: Some(vec!["ETL".to_string()]),
        ip_allowlist: None,
        expires_at: None,
    };
    // Admin always sees everything regardless of allowed_groups
    assert!(key.can_access_group(Some("Deploys")));
    assert!(key.can_access_group(Some("Random")));
}
