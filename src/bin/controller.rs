use std::sync::Arc;

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

    info!(
        "opening database: {} (pool size: {})",
        config.db_path, config.db_pool_size
    );
    let db = Db::open_with_pool_size(&config.db_path, config.db_pool_size, config.db_timeout_secs)?;
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

        // Keys are only printed to stderr above — never written to disk for security.
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
                // Audit log retention (separate from events, default 90 days)
                let audit_days = db_tick
                    .get_setting("audit_retention_days")
                    .ok()
                    .flatten()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(90);
                if audit_days > 0 {
                    let _ = db_tick.purge_old_audit_log(audit_days);
                }
                // Cleanup expired OIDC sessions and auth states
                let _ = db_tick.cleanup_expired_sessions();
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

    // Build rate limiters
    let rate_limiters = {
        use kronforce::api::rate_limit::{RateLimiter, RateLimiters};
        let window_secs = 60;
        let mk = |limit: u32| -> Option<RateLimiter> {
            if config.rate_limit_enabled && limit > 0 {
                Some(RateLimiter::new(limit, window_secs))
            } else {
                None
            }
        };
        RateLimiters {
            public: mk(config.rate_limit_public),
            authenticated: mk(config.rate_limit_authenticated),
            agent: mk(config.rate_limit_agent),
        }
    };

    // Spawn rate limit cleanup task
    if config.rate_limit_enabled {
        let rl = rate_limiters.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                if let Some(ref l) = rl.public {
                    l.cleanup();
                }
                if let Some(ref l) = rl.authenticated {
                    l.cleanup();
                }
                if let Some(ref l) = rl.agent {
                    l.cleanup();
                }
            }
        });
    }

    // Initialize OIDC if configured
    let oidc_state = if let Some(oidc_config) = config.oidc {
        info!(
            "OIDC configured, discovering provider at {}",
            oidc_config.issuer
        );
        match kronforce::api::oidc::discover(&oidc_config.issuer).await {
            Ok(provider) => {
                info!("OIDC provider discovered: {}", provider.issuer);
                Some(Arc::new(kronforce::api::oidc::OidcState {
                    config: oidc_config,
                    provider,
                }))
            }
            Err(e) => {
                warn!("OIDC discovery failed, SSO disabled: {}", e);
                None
            }
        }
    } else {
        None
    };

    let state = kronforce::api::AppState {
        db,
        dag,
        scheduler_tx,
        agent_client,
        callback_base_url: config.callback_base_url.clone(),
        script_store: script_store.clone(),
        oidc: oidc_state,
    };
    // Clone DB handle before moving state into router
    let shutdown_db = state.db.clone();
    let app = kronforce::api::router(state, rate_limiters, config.mcp_enabled);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    info!("listening on {}", config.bind_addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let ctrl_c = tokio::signal::ctrl_c();
            #[cfg(unix)]
            {
                let mut sigterm =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                        .expect("failed to install SIGTERM handler");
                tokio::select! {
                    _ = ctrl_c => {},
                    _ = sigterm.recv() => {},
                }
            }
            #[cfg(not(unix))]
            {
                ctrl_c.await.ok();
            }
            info!("shutdown signal received, checkpointing WAL...");
            if let Err(e) = shutdown_db.checkpoint() {
                warn!("WAL checkpoint failed: {}", e);
            } else {
                info!("WAL checkpoint complete");
            }
        })
        .await?;

    info!("server stopped");
    Ok(())
}
