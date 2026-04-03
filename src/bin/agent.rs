use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use uuid::Uuid;

use tracing::{debug, error, info, warn};

use kronforce::agent;
use kronforce::agent::protocol::{AgentHeartbeat, AgentRegistration, AgentRegistrationResponse};
use kronforce::config::AgentConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kronforce_agent=debug,info".parse().unwrap()),
        )
        .init();

    let config = AgentConfig::from_env();

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    // Determine our address - use config or hostname
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    // Register with controller
    let reg = AgentRegistration {
        name: config.name.clone(),
        tags: config.tags.clone(),
        hostname: hostname.clone(),
        address: config.address.clone(),
        port: config.port,
        agent_type: Some("standard".to_string()),
        task_types: None,
    };

    info!("registering with controller at {}", config.controller_url);

    let reg_url = format!("{}/api/agents/register", config.controller_url);
    let mut req = http_client.post(&reg_url).json(&reg);
    if let Some(ref key) = config.agent_key {
        req = req.header("Authorization", format!("Bearer {}", key));
    }
    let response = req.send().await?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if status.as_u16() == 401 || status.as_u16() == 403 {
            error!("authentication failed — set KRONFORCE_AGENT_KEY with a valid agent API key");
            error!("server response: {}", body);
        } else {
            error!("registration failed ({}): {}", status, body);
        }
        std::process::exit(1);
    }
    let resp: AgentRegistrationResponse = response.json().await?;

    let agent_id = resp.agent_id;
    let heartbeat_interval = std::time::Duration::from_secs(resp.heartbeat_interval_secs);
    info!("registered as agent {} (id: {})", config.name, agent_id);

    // Start heartbeat loop
    let hb_client = http_client.clone();
    let hb_url = format!(
        "{}/api/agents/{}/heartbeat",
        config.controller_url, agent_id
    );
    let hb_key = config.agent_key.clone();
    let running_map: Arc<Mutex<HashMap<Uuid, tokio::sync::oneshot::Sender<()>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let hb_running = running_map.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(heartbeat_interval);
        loop {
            interval.tick().await;
            let running_ids: Vec<Uuid> = hb_running.lock().await.keys().copied().collect();
            let hb = AgentHeartbeat {
                agent_id,
                running_executions: running_ids,
            };
            let mut req = hb_client.post(&hb_url).json(&hb);
            if let Some(ref key) = hb_key {
                req = req.header("Authorization", format!("Bearer {}", key));
            }
            match req.send().await {
                Ok(_) => debug!("heartbeat sent"),
                Err(e) => warn!("heartbeat failed: {e}"),
            }
        }
    });

    // Start agent server
    let state = agent::server::AgentState {
        agent_id,
        controller_url: config.controller_url.clone(),
        http_client,
        running: running_map,
        agent_key: config.agent_key.clone(),
    };

    let app = agent::server::router(state);
    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;

    if let (Some(cert), Some(key)) = (config.tls_cert, config.tls_key) {
        let tls_config = kronforce::tls::load_tls_config(&cert, &key)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        info!("agent listening on {} (TLS)", config.bind_addr);
        kronforce::tls::serve_tls(listener, app, tls_config, async {
            tokio::signal::ctrl_c().await.ok();
        })
        .await?;
    } else {
        info!("agent listening on {}", config.bind_addr);
        axum::serve(listener, app).await?;
    }

    Ok(())
}
