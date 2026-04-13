use kronforce::db::models::*;

// --- TaskType serialization ---

#[test]
fn test_shell_task_serde() {
    let task = TaskType::Shell {
        command: "echo hello".to_string(),
        working_dir: None,
    };
    let json = serde_json::to_string(&task).unwrap();
    assert!(json.contains("\"type\":\"shell\""));
    let back: TaskType = serde_json::from_str(&json).unwrap();
    if let TaskType::Shell { command, .. } = back {
        assert_eq!(command, "echo hello");
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn test_http_task_serde() {
    let task = TaskType::Http {
        method: HttpMethod::Post,
        url: "https://example.com".to_string(),
        headers: None,
        body: Some("{\"key\":\"val\"}".to_string()),
        expect_status: Some(200),
    };
    let json = serde_json::to_string(&task).unwrap();
    let back: TaskType = serde_json::from_str(&json).unwrap();
    if let TaskType::Http {
        method,
        url,
        expect_status,
        ..
    } = back
    {
        assert_eq!(method, HttpMethod::Post);
        assert_eq!(url, "https://example.com");
        assert_eq!(expect_status, Some(200));
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn test_custom_task_serde() {
    let task = TaskType::Custom {
        agent_task_type: "python".to_string(),
        data: serde_json::json!({"script": "print(1)"}),
    };
    let json = serde_json::to_string(&task).unwrap();
    assert!(json.contains("\"type\":\"custom\""));
    assert!(json.contains("\"agent_task_type\":\"python\""));
}

#[test]
fn test_file_push_task_serde() {
    let task = TaskType::FilePush {
        filename: "app.conf".to_string(),
        destination: "/opt/app.conf".to_string(),
        content_base64: "dGVzdA==".to_string(),
        permissions: Some("644".to_string()),
        overwrite: true,
    };
    let json = serde_json::to_string(&task).unwrap();
    assert!(json.contains("\"type\":\"file_push\""));
    let back: TaskType = serde_json::from_str(&json).unwrap();
    if let TaskType::FilePush {
        filename,
        overwrite,
        ..
    } = back
    {
        assert_eq!(filename, "app.conf");
        assert!(overwrite);
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn test_kafka_task_serde() {
    let task = TaskType::Kafka {
        broker: "localhost:9092".to_string(),
        topic: "events".to_string(),
        message: "{}".to_string(),
        key: Some("key1".to_string()),
        properties: None,
    };
    let json = serde_json::to_string(&task).unwrap();
    assert!(json.contains("\"type\":\"kafka\""));
}

#[test]
fn test_mqtt_task_serde() {
    let task = TaskType::Mqtt {
        broker: "localhost".to_string(),
        topic: "sensors/temp".to_string(),
        message: "22.5".to_string(),
        port: Some(1883),
        qos: Some(1),
        username: None,
        password: None,
        client_id: None,
    };
    let json = serde_json::to_string(&task).unwrap();
    assert!(json.contains("\"type\":\"mqtt\""));
    let back: TaskType = serde_json::from_str(&json).unwrap();
    if let TaskType::Mqtt { qos, port, .. } = back {
        assert_eq!(qos, Some(1));
        assert_eq!(port, Some(1883));
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn test_rabbitmq_task_serde() {
    let task = TaskType::Rabbitmq {
        url: "amqp://localhost".to_string(),
        exchange: "events".to_string(),
        routing_key: "user.created".to_string(),
        message: "{}".to_string(),
        content_type: Some("application/json".to_string()),
    };
    let json = serde_json::to_string(&task).unwrap();
    assert!(json.contains("\"type\":\"rabbitmq\""));
}

#[test]
fn test_redis_task_serde() {
    let task = TaskType::Redis {
        url: "redis://localhost".to_string(),
        channel: "notifications".to_string(),
        message: "hello".to_string(),
    };
    let json = serde_json::to_string(&task).unwrap();
    assert!(json.contains("\"type\":\"redis\""));
}

// --- Schedule serialization ---

#[test]
fn test_cron_schedule_serde() {
    let sched = ScheduleKind::Cron(CronExpr("0 * * * * *".to_string()));
    let json = serde_json::to_string(&sched).unwrap();
    let back: ScheduleKind = serde_json::from_str(&json).unwrap();
    if let ScheduleKind::Cron(expr) = back {
        assert_eq!(expr.0, "0 * * * * *");
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn test_on_demand_schedule_serde() {
    let sched = ScheduleKind::OnDemand;
    let json = serde_json::to_string(&sched).unwrap();
    assert!(json.contains("on_demand"));
}

// --- ApiKeyRole ---

#[test]
fn test_role_permissions() {
    assert!(ApiKeyRole::Admin.can_write());
    assert!(ApiKeyRole::Admin.can_manage_keys());
    assert!(!ApiKeyRole::Admin.is_agent());

    assert!(ApiKeyRole::Operator.can_write());
    assert!(!ApiKeyRole::Operator.can_manage_keys());

    assert!(!ApiKeyRole::Viewer.can_write());
    assert!(!ApiKeyRole::Viewer.can_manage_keys());

    assert!(!ApiKeyRole::Agent.can_write());
    assert!(!ApiKeyRole::Agent.can_manage_keys());
    assert!(ApiKeyRole::Agent.is_agent());
}

#[test]
fn test_role_from_str() {
    assert_eq!(ApiKeyRole::from_str("admin"), Some(ApiKeyRole::Admin));
    assert_eq!(ApiKeyRole::from_str("operator"), Some(ApiKeyRole::Operator));
    assert_eq!(ApiKeyRole::from_str("viewer"), Some(ApiKeyRole::Viewer));
    assert_eq!(ApiKeyRole::from_str("agent"), Some(ApiKeyRole::Agent));
    assert_eq!(ApiKeyRole::from_str("invalid"), None);
}

// --- AgentTarget ---

#[test]
fn test_agent_target_serde() {
    let targets = vec![
        (AgentTarget::Local, "local"),
        (AgentTarget::Any, "any"),
        (AgentTarget::All, "all"),
    ];
    for (target, expected_type) in targets {
        let json = serde_json::to_string(&target).unwrap();
        assert!(json.contains(expected_type));
    }
}

// --- OutputRules ---

#[test]
fn test_output_rules_defaults() {
    let rules: OutputRules = serde_json::from_str("{}").unwrap();
    assert!(rules.extractions.is_empty());
    assert!(rules.triggers.is_empty());
    assert!(rules.assertions.is_empty());
}

// --- JobNotificationConfig ---

#[test]
fn test_notification_config_defaults() {
    let config: JobNotificationConfig = serde_json::from_str("{}").unwrap();
    assert!(!config.on_failure);
    assert!(!config.on_success);
    assert!(!config.on_assertion_failure);
    assert!(config.recipients.is_none());
}

// --- ExecutionStatus ---

#[test]
fn test_execution_status_serde() {
    let statuses = vec![
        ExecutionStatus::Pending,
        ExecutionStatus::Running,
        ExecutionStatus::Succeeded,
        ExecutionStatus::Failed,
        ExecutionStatus::TimedOut,
        ExecutionStatus::Cancelled,
        ExecutionStatus::Skipped,
    ];
    for status in statuses {
        let json = serde_json::to_string(&status).unwrap();
        let back: ExecutionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, status);
    }
}

#[test]
fn test_trigger_source_webhook_serde() {
    let trigger = TriggerSource::Webhook {
        token_prefix: "abc12345".to_string(),
    };
    let json = serde_json::to_string(&trigger).unwrap();
    assert!(json.contains("\"type\":\"webhook\""));
    assert!(json.contains("\"token_prefix\":\"abc12345\""));
    let back: TriggerSource = serde_json::from_str(&json).unwrap();
    if let TriggerSource::Webhook { token_prefix } = back {
        assert_eq!(token_prefix, "abc12345");
    } else {
        panic!("expected Webhook trigger");
    }
}

#[test]
fn test_job_parameter_serde() {
    let param = JobParameter {
        name: "version".to_string(),
        param_type: "text".to_string(),
        required: true,
        default: Some("1.0".to_string()),
        options: None,
        description: Some("The version".to_string()),
    };
    let json = serde_json::to_string(&param).unwrap();
    let back: JobParameter = serde_json::from_str(&json).unwrap();
    assert_eq!(back.name, "version");
    assert!(back.required);
    assert_eq!(back.default.as_deref(), Some("1.0"));
}
