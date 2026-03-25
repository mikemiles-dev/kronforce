use kronforce::agent_client::AgentClient;
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
        let (raw_key, prefix) = kronforce::api::generate_api_key();
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
            let _ = tokio::task::spawn_blocking(move || {
                // Get online agents before expire
                let before: Vec<_> = db.list_agents()?.into_iter()
                    .filter(|a| a.status == kronforce::models::AgentStatus::Online)
                    .map(|a| (a.id, a.name.clone()))
                    .collect();
                db.expire_agents(timeout)?;
                // Check which went offline
                let after = db.list_agents()?;
                for (id, name) in &before {
                    if let Some(a) = after.iter().find(|a| a.id == *id) {
                        if a.status == kronforce::models::AgentStatus::Offline {
                            let _ = db.log_event(
                                "agent.offline",
                                kronforce::models::EventSeverity::Warning,
                                &format!("Agent '{}' went offline (heartbeat timeout)", name),
                                None,
                                Some(*id),
                            );
                        }
                    }
                }
                Ok::<(), kronforce::error::AppError>(())
            }).await;
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
