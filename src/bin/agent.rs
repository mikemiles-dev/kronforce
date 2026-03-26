use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use uuid::Uuid;

use kronforce::agent_server;
use kronforce::config::AgentConfig;
use kronforce::protocol::{AgentHeartbeat, AgentRegistration, AgentRegistrationResponse};

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

    tracing::info!(
        "registering with controller at {}",
        config.controller_url
    );

    let reg_url = format!("{}/api/agents/register", config.controller_url);
    let resp: AgentRegistrationResponse = http_client
        .post(&reg_url)
        .json(&reg)
        .send()
        .await?
        .json()
        .await?;

    let agent_id = resp.agent_id;
    let heartbeat_interval = std::time::Duration::from_secs(resp.heartbeat_interval_secs);
    tracing::info!("registered as agent {} (id: {})", config.name, agent_id);

    // Start heartbeat loop
    let hb_client = http_client.clone();
    let hb_url = format!(
        "{}/api/agents/{}/heartbeat",
        config.controller_url, agent_id
    );
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
            match hb_client.post(&hb_url).json(&hb).send().await {
                Ok(_) => tracing::debug!("heartbeat sent"),
                Err(e) => tracing::warn!("heartbeat failed: {e}"),
            }
        }
    });

    // Start agent server
    let state = agent_server::AgentState {
        agent_id,
        controller_url: config.controller_url.clone(),
        http_client,
        running: running_map,
    };

    let app = agent_server::router(state);
    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!("agent listening on {}", config.bind_addr);
    axum::serve(listener, app).await?;

    Ok(())
}
