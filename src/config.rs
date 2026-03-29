use std::time::Duration;

/// Configuration for the Kronforce controller server.
pub struct ControllerConfig {
    pub db_path: String,
    pub bind_addr: String,
    pub tick_interval: Duration,
    pub agent_heartbeat_timeout: Duration,
    pub callback_base_url: String,
    pub scripts_dir: String,
    pub rate_limit_enabled: bool,
    pub rate_limit_public: u32,
    pub rate_limit_authenticated: u32,
    pub rate_limit_agent: u32,
}

impl ControllerConfig {
    /// Builds a `ControllerConfig` from `KRONFORCE_*` environment variables with defaults.
    pub fn from_env() -> Self {
        let bind_addr =
            std::env::var("KRONFORCE_BIND").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
        let callback_base_url = std::env::var("KRONFORCE_CALLBACK_URL")
            .unwrap_or_else(|_| format!("http://{}", bind_addr));
        Self {
            db_path: std::env::var("KRONFORCE_DB").unwrap_or_else(|_| "kronforce.db".to_string()),
            bind_addr,
            tick_interval: Duration::from_secs(
                std::env::var("KRONFORCE_TICK_SECS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1),
            ),
            agent_heartbeat_timeout: Duration::from_secs(
                std::env::var("KRONFORCE_HEARTBEAT_TIMEOUT_SECS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(30),
            ),
            callback_base_url,
            scripts_dir: std::env::var("KRONFORCE_SCRIPTS_DIR")
                .unwrap_or_else(|_| "./scripts".to_string()),
            rate_limit_enabled: std::env::var("KRONFORCE_RATE_LIMIT_ENABLED")
                .map(|v| v != "false" && v != "0")
                .unwrap_or(true),
            rate_limit_public: std::env::var("KRONFORCE_RATE_LIMIT_PUBLIC")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            rate_limit_authenticated: std::env::var("KRONFORCE_RATE_LIMIT_AUTHENTICATED")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(120),
            rate_limit_agent: std::env::var("KRONFORCE_RATE_LIMIT_AGENT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(600),
        }
    }
}

/// Configuration for a Kronforce agent process.
pub struct AgentConfig {
    pub controller_url: String,
    pub bind_addr: String,
    pub name: String,
    pub address: String,
    pub tags: Vec<String>,
    pub port: u16,
    pub heartbeat_interval: Duration,
    pub agent_key: Option<String>,
}

impl AgentConfig {
    /// Builds an `AgentConfig` from `KRONFORCE_AGENT_*` environment variables with defaults.
    pub fn from_env() -> Self {
        let bind_addr =
            std::env::var("KRONFORCE_AGENT_BIND").unwrap_or_else(|_| "0.0.0.0:8081".to_string());
        let port: u16 = bind_addr
            .rsplit(':')
            .next()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8081);
        let address = std::env::var("KRONFORCE_AGENT_ADDRESS").unwrap_or_else(|_| {
            hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "127.0.0.1".to_string())
        });
        Self {
            controller_url: std::env::var("KRONFORCE_CONTROLLER_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            bind_addr,
            name: std::env::var("KRONFORCE_AGENT_NAME").unwrap_or_else(|_| {
                hostname::get()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "agent-1".to_string())
            }),
            address,
            tags: std::env::var("KRONFORCE_AGENT_TAGS")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            port,
            heartbeat_interval: Duration::from_secs(
                std::env::var("KRONFORCE_HEARTBEAT_SECS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10),
            ),
            agent_key: std::env::var("KRONFORCE_AGENT_KEY")
                .ok()
                .filter(|s| !s.is_empty()),
        }
    }
}
