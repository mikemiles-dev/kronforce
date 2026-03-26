use kronforce::agent::AgentClient;
use kronforce::config::ControllerConfig;
use kronforce::dag::DagResolver;
use kronforce::db::Db;
use kronforce::executor::Executor;
use kronforce::scheduler::Scheduler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kronforce=debug,info".parse().unwrap()),
        )
        .init();

    let config = ControllerConfig::from_env();

    tracing::info!("opening database: {}", config.db_path);
    let db = Db::open(&config.db_path)?;
    db.migrate()?;

    // Bootstrap admin API key if none exist
    if db.count_api_keys()? == 0 {
        let (raw_key, prefix) = if let Ok(preset) = std::env::var("KRONFORCE_BOOTSTRAP_ADMIN_KEY") {
            if !preset.is_empty() {
                let pfx = if preset.len() >= 11 { preset[..11].to_string() } else { preset.clone() };
                (preset, pfx)
            } else {
                kronforce::api::generate_api_key()
            }
        } else {
            kronforce::api::generate_api_key()
        };
        let hash = kronforce::api::hash_api_key(&raw_key);
        let key = kronforce::models::ApiKey {
            id: uuid::Uuid::new_v4(),
            key_prefix: prefix,
            key_hash: hash,
            name: "admin (bootstrap)".to_string(),
            role: kronforce::models::ApiKeyRole::Admin,
            created_at: chrono::Utc::now(),
            last_used_at: None,
            active: true,
        };
        db.insert_api_key(&key)?;
        tracing::info!("=============================================================");
        tracing::info!("  No API keys found. Bootstrap admin key created:");
        tracing::info!("  {}", raw_key);
        tracing::info!("  Save this key — it will not be shown again.");
        tracing::info!("=============================================================");

        // Also create a bootstrap agent key — use KRONFORCE_BOOTSTRAP_AGENT_KEY if set
        let (agent_raw, agent_prefix) = if let Ok(preset) = std::env::var("KRONFORCE_BOOTSTRAP_AGENT_KEY") {
            if !preset.is_empty() {
                let prefix = if preset.len() >= 11 { preset[..11].to_string() } else { preset.clone() };
                (preset, prefix)
            } else {
                kronforce::api::generate_api_key()
            }
        } else {
            kronforce::api::generate_api_key()
        };
        let agent_hash = kronforce::api::hash_api_key(&agent_raw);
        let agent_key_record = kronforce::models::ApiKey {
            id: uuid::Uuid::new_v4(),
            key_prefix: agent_prefix,
            key_hash: agent_hash,
            name: "agent (bootstrap)".to_string(),
            role: kronforce::models::ApiKeyRole::Agent,
            created_at: chrono::Utc::now(),
            last_used_at: None,
            active: true,
        };
        db.insert_api_key(&agent_key_record)?;
        tracing::info!("=============================================================");
        tracing::info!("  Bootstrap agent key created:");
        tracing::info!("  {}", agent_raw);
        tracing::info!("  Set KRONFORCE_AGENT_KEY={} on your agents.", agent_raw);
        tracing::info!("  Save this key — it will not be shown again.");
        tracing::info!("=============================================================");
    }

    let (scheduler_tx, scheduler_rx) = tokio::sync::mpsc::channel(64);

    let agent_client = AgentClient::new();
    let script_store = kronforce::scripts::ScriptStore::new(&config.scripts_dir)?;
    tracing::info!("scripts directory: {}", config.scripts_dir);
    let executor = Executor::new(db.clone(), agent_client.clone(), scheduler_tx.clone(), script_store.clone());
    let dag = DagResolver::new(db.clone());
    let scheduler = Scheduler::new(
        db.clone(),
        executor,
        dag.clone(),
        scheduler_rx,
        &config,
        agent_client.clone(),
    );

    tokio::spawn(scheduler.run());

    // Spawn agent health monitor
    let health_db = db.clone();
    let timeout = config.agent_heartbeat_timeout;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            let db = health_db.clone();
            let timeout = timeout;
            let db_tick = db.clone();
            let offline_agents: Vec<(String, String)> = tokio::task::spawn_blocking(move || {
                let before: Vec<_> = db_tick.list_agents().unwrap_or_default().into_iter()
                    .filter(|a| a.status == kronforce::models::AgentStatus::Online)
                    .map(|a| (a.id, a.name.clone()))
                    .collect();
                let _ = db_tick.expire_agents(timeout);
                let after = db_tick.list_agents().unwrap_or_default();
                let mut went_offline = Vec::new();
                for (id, name) in &before {
                    if let Some(a) = after.iter().find(|a| a.id == *id) {
                        if a.status == kronforce::models::AgentStatus::Offline {
                            let _ = db_tick.log_event(
                                "agent.offline",
                                kronforce::models::EventSeverity::Warning,
                                &format!("Agent '{}' went offline (heartbeat timeout)", name),
                                None,
                                Some(*id),
                            );
                            went_offline.push((name.clone(), a.hostname.clone()));
                        }
                    }
                }
                let _ = db_tick.fail_stale_pending_queue_items(300);
                let _ = db_tick.fail_stale_claimed_queue_items(600);
                if let Ok(Some(days_str)) = db_tick.get_setting("retention_days") {
                    if let Ok(days) = days_str.parse::<i64>() {
                        if days > 0 {
                            let _ = db_tick.purge_old_executions(days);
                            let _ = db_tick.purge_old_events(days);
                            let _ = db_tick.purge_old_queue_items(days);
                        }
                    }
                }
                went_offline
            }).await.unwrap_or_default();

            // Send notifications for agents that went offline
            if !offline_agents.is_empty() {
                let db_notif = health_db.clone();
                let alerts = kronforce::notifications::load_system_alerts(&db_notif);
                if alerts.agent_offline {
                    for (name, hostname) in &offline_agents {
                        let subject = format!("[Kronforce] Agent '{}' went offline", name);
                        let body = format!(
                            "Agent: {}\nHostname: {}\nTime: {}\n\nThe agent's heartbeat timed out.",
                            name, hostname, chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
                        );
                        kronforce::notifications::send_notification(&db_notif, &subject, &body, None).await;
                    }
                }
            }
        }
    });

    let state = kronforce::api::AppState {
        db,
        dag,
        scheduler_tx,
        agent_client,
        callback_base_url: config.callback_base_url.clone(),
        script_store: script_store.clone(),
    };
    let app = kronforce::api::router(state);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!("listening on {}", config.bind_addr);
    axum::serve(listener, app).await?;

    Ok(())
}
