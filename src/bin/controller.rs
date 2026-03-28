use tracing::{info, warn};

use kronforce::agent::AgentClient;
use kronforce::config::ControllerConfig;
use kronforce::dag::DagResolver;
use kronforce::db::Db;
use kronforce::executor::Executor;
use kronforce::scheduler::Scheduler;

fn create_admin_key(db: &Db) -> Result<String, Box<dyn std::error::Error>> {
    let (key, raw) = kronforce::db::models::ApiKey::bootstrap(
        kronforce::db::models::ApiKeyRole::Admin,
        "admin (reset)",
        None,
    );
    db.insert_api_key(&key)?;
    Ok(raw)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kronforce=debug,info".parse().unwrap()),
        )
        .init();

    let args: Vec<String> = std::env::args().collect();
    let reset_key = args.iter().any(|a| a == "--reset-admin-key");
    let reset_key_and_run = args.iter().any(|a| a == "--reset-admin-key-and-run");

    if args.iter().any(|a| a == "--help" || a == "-h") {
        eprintln!("Usage: kronforce [OPTIONS]");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --reset-admin-key          Generate a new admin API key and exit");
        eprintln!("  --reset-admin-key-and-run  Generate a new admin API key and start the server");
        eprintln!("  -h, --help                 Show this help message");
        eprintln!();
        eprintln!("Configuration is via environment variables. See docs for details.");
        std::process::exit(0);
    }

    let config = ControllerConfig::from_env();

    info!("opening database: {}", config.db_path);
    let db = Db::open(&config.db_path)?;
    db.migrate()?;

    if reset_key || reset_key_and_run {
        let raw_key = create_admin_key(&db)?;
        eprintln!("=============================================================");
        eprintln!("  New admin API key created:");
        eprintln!("  {}", raw_key);
        eprintln!("  Save this key — it will not be shown again.");
        eprintln!("=============================================================");
        if reset_key {
            std::process::exit(0);
        }
    }

    // Bootstrap admin API key if none exist
    if db.count_api_keys()? == 0 {
        let admin_preset = std::env::var("KRONFORCE_BOOTSTRAP_ADMIN_KEY").ok();
        let (admin_key, admin_raw) = kronforce::db::models::ApiKey::bootstrap(
            kronforce::db::models::ApiKeyRole::Admin,
            "admin (bootstrap)",
            admin_preset,
        );
        db.insert_api_key(&admin_key)?;
        info!("=============================================================");
        info!("  No API keys found. Bootstrap admin key created:");
        info!("  {}", admin_raw);
        info!("  Save this key — it will not be shown again.");
        info!("=============================================================");

        let agent_preset = std::env::var("KRONFORCE_BOOTSTRAP_AGENT_KEY").ok();
        let (agent_key, agent_raw) = kronforce::db::models::ApiKey::bootstrap(
            kronforce::db::models::ApiKeyRole::Agent,
            "agent (bootstrap)",
            agent_preset,
        );
        db.insert_api_key(&agent_key)?;
        info!("=============================================================");
        info!("  Bootstrap agent key created:");
        info!("  {}", agent_raw);
        info!("  Set KRONFORCE_AGENT_KEY={} on your agents.", agent_raw);
        info!("  Save this key — it will not be shown again.");
        info!("=============================================================");

        // Write keys to file next to the database for easy retrieval
        let db_path = std::path::Path::new(&config.db_path);
        let keys_path = db_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("bootstrap-keys.txt");
        let keys_content = format!(
            "# Kronforce Bootstrap Keys\n# Generated: {}\n# Store these securely and delete this file.\n\nADMIN_KEY={}\nAGENT_KEY={}\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            admin_raw,
            agent_raw,
        );
        match std::fs::write(&keys_path, &keys_content) {
            Ok(()) => {
                // Restrict file permissions on Unix
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(
                        &keys_path,
                        std::fs::Permissions::from_mode(0o600),
                    );
                }
                info!("  Keys saved to: {}", keys_path.display());
                info!("  Retrieve with: cat {}", keys_path.display());
            }
            Err(e) => {
                warn!("  Could not write keys file: {}", e);
            }
        }
    }

    let (scheduler_tx, scheduler_rx) = tokio::sync::mpsc::channel(64);

    let agent_client = AgentClient::new();
    let script_store = kronforce::executor::scripts::ScriptStore::new(&config.scripts_dir)?;
    info!("scripts directory: {}", config.scripts_dir);
    let executor = Executor::new(
        db.clone(),
        agent_client.clone(),
        scheduler_tx.clone(),
        script_store.clone(),
    );
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
                let before: Vec<_> = db_tick
                    .list_agents()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|a| a.status == kronforce::db::models::AgentStatus::Online)
                    .map(|a| (a.id, a.name.clone()))
                    .collect();
                let _ = db_tick.expire_agents(timeout);
                let after = db_tick.list_agents().unwrap_or_default();
                let mut went_offline = Vec::new();
                for (id, name) in &before {
                    if let Some(a) = after.iter().find(|a| a.id == *id)
                        && a.status == kronforce::db::models::AgentStatus::Offline
                    {
                        let _ = db_tick.log_event(
                            "agent.offline",
                            kronforce::db::models::EventSeverity::Warning,
                            &format!("Agent '{}' went offline (heartbeat timeout)", name),
                            None,
                            Some(*id),
                        );
                        went_offline.push((name.clone(), a.hostname.clone()));
                    }
                }
                let _ = db_tick.fail_stale_pending_queue_items(300);
                let _ = db_tick.fail_stale_claimed_queue_items(600);
                if let Ok(Some(days_str)) = db_tick.get_setting("retention_days")
                    && let Ok(days) = days_str.parse::<i64>()
                    && days > 0
                {
                    let _ = db_tick.purge_old_executions(days);
                    let _ = db_tick.purge_old_events(days);
                    let _ = db_tick.purge_old_queue_items(days);
                }
                went_offline
            })
            .await
            .unwrap_or_default();

            // Send notifications for agents that went offline
            if !offline_agents.is_empty() {
                let db_notif = health_db.clone();
                let alerts = kronforce::executor::notifications::load_system_alerts(&db_notif);
                if alerts.agent_offline {
                    for (name, hostname) in &offline_agents {
                        let subject = format!("[Kronforce] Agent '{}' went offline", name);
                        let body = format!(
                            "Agent: {}\nHostname: {}\nTime: {}\n\nThe agent's heartbeat timed out.",
                            name,
                            hostname,
                            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
                        );
                        kronforce::executor::notifications::send_notification(
                            &db_notif, &subject, &body, None,
                        )
                        .await;
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
    info!("listening on {}", config.bind_addr);
    axum::serve(listener, app).await?;

    Ok(())
}
