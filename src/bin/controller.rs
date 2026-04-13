use std::sync::Arc;

use chrono::Timelike;
use tracing::{info, warn};

use kronforce::agent::AgentClient;
use kronforce::config::ControllerConfig;
use kronforce::dag::DagResolver;
use kronforce::db::Db;
use kronforce::executor::Executor;
use kronforce::scheduler::Scheduler;

/// Parses an HH:MM string to minutes since midnight. Returns None on invalid input.
fn parse_hhmm(hhmm: &str) -> Option<i32> {
    let parts: Vec<&str> = hhmm.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let h: i32 = parts[0].parse().ok()?;
    let m: i32 = parts[1].parse().ok()?;
    Some(h * 60 + m)
}

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
    let log_format = std::env::var("KRONFORCE_LOG_FORMAT")
        .unwrap_or_default()
        .to_lowercase();
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "kronforce=debug,info".parse().unwrap());
    if log_format == "json" {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .json()
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    }

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

    kronforce::crypto::init();
    if kronforce::crypto::is_enabled() {
        info!("field encryption enabled (KRONFORCE_ENCRYPTION_KEY set)");
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
    let live_output = std::sync::Arc::new(dashmap::DashMap::new());
    let executor = Executor::new(
        db.clone(),
        agent_client.clone(),
        scheduler_tx.clone(),
        script_store.clone(),
        live_output.clone(),
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
                if let Err(e) = db_tick.expire_agents(timeout) {
                    tracing::warn!("expire_agents failed: {e}");
                }
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
                if let Err(e) = db_tick.fail_stale_pending_queue_items(300) {
                    tracing::warn!("fail_stale_pending failed: {e}");
                }
                if let Err(e) = db_tick.fail_stale_claimed_queue_items(600) {
                    tracing::warn!("fail_stale_claimed failed: {e}");
                }
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

                // SLA deadline checks for running executions
                if let Ok(jobs) = db_tick.list_jobs(None, None, None, 1000, 0) {
                    let now_utc = chrono::Utc::now();
                    let now_mins = (now_utc.hour() * 60 + now_utc.minute()) as i32;
                    for job in &jobs {
                        if let Some(ref deadline) = job.sla_deadline {
                            if job.status != kronforce::db::models::JobStatus::Scheduled {
                                continue;
                            }
                            let Some(deadline_mins) = parse_hhmm(deadline) else {
                                continue;
                            };
                            // Check if there's a running execution for this job
                            if let Ok(execs) =
                                db_tick.list_executions_for_job(job.id, 1, 0)
                            {
                                let running = execs
                                    .iter()
                                    .any(|e| e.status == kronforce::db::models::ExecutionStatus::Running);
                                if !running {
                                    continue;
                                }
                                // Warning check (fire once when crossing the threshold)
                                if job.sla_warning_mins > 0 {
                                    let warn_mins = (deadline_mins - job.sla_warning_mins as i32 + 1440) % 1440;
                                    if now_mins == warn_mins {
                                        let _ = db_tick.log_event(
                                            "sla.warning",
                                            kronforce::db::models::EventSeverity::Warning,
                                            &format!(
                                                "Job '{}' SLA warning: must complete by {} UTC ({} min remaining)",
                                                job.name, deadline, job.sla_warning_mins
                                            ),
                                            Some(job.id),
                                            None,
                                        );
                                    }
                                }
                                // Breach check
                                if now_mins == deadline_mins {
                                    let _ = db_tick.log_event(
                                        "sla.breach",
                                        kronforce::db::models::EventSeverity::Error,
                                        &format!(
                                            "Job '{}' SLA BREACHED: deadline {} UTC missed",
                                            job.name, deadline
                                        ),
                                        Some(job.id),
                                        None,
                                    );
                                }
                            }
                        }
                    }
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
        demo_mode: config.demo_mode,
        live_output,
    };
    if config.demo_mode {
        info!("DEMO MODE: auth disabled, all requests are read-only (viewer)");
    }
    // Clone DB handle before moving state into router
    let shutdown_db = state.db.clone();
    let app = kronforce::api::router(state, rate_limiters, config.mcp_enabled);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;

    let shutdown_signal = async move {
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
    };

    if let (Some(cert), Some(key)) = (config.tls_cert, config.tls_key) {
        let tls_config = kronforce::tls::load_tls_config(&cert, &key)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        info!("listening on {} (TLS)", config.bind_addr);
        kronforce::tls::serve_tls(listener, app, tls_config, shutdown_signal).await?;
    } else {
        info!("listening on {}", config.bind_addr);
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal)
            .await?;
    }

    info!("server stopped");
    Ok(())
}
