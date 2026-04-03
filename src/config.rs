use std::time::Duration;

use crate::db::models::ApiKeyRole;

/// OIDC/OAuth2 configuration for enterprise SSO login.
pub struct OidcConfig {
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scopes: String,
    pub role_claim: String,
    pub admin_values: Vec<String>,
    pub operator_values: Vec<String>,
    pub default_role: ApiKeyRole,
    pub session_ttl_secs: u64,
}

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
    pub db_pool_size: u32,
    pub db_timeout_secs: u64,
    pub mcp_enabled: bool,
    pub oidc: Option<OidcConfig>,
    pub tls_cert: Option<String>,
    pub tls_key: Option<String>,
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
            callback_base_url: callback_base_url.clone(),
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
            db_pool_size: std::env::var("KRONFORCE_DB_POOL_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8),
            db_timeout_secs: std::env::var("KRONFORCE_DB_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            mcp_enabled: std::env::var("KRONFORCE_MCP_ENABLED")
                .map(|v| v != "false" && v != "0")
                .unwrap_or(true),
            oidc: {
                let issuer = std::env::var("KRONFORCE_OIDC_ISSUER").ok();
                let client_id = std::env::var("KRONFORCE_OIDC_CLIENT_ID").ok();
                match (issuer, client_id) {
                    (Some(issuer), Some(client_id))
                        if !issuer.is_empty() && !client_id.is_empty() =>
                    {
                        Some(OidcConfig {
                            issuer,
                            client_id,
                            client_secret: std::env::var("KRONFORCE_OIDC_CLIENT_SECRET")
                                .unwrap_or_default(),
                            redirect_uri: std::env::var("KRONFORCE_OIDC_REDIRECT_URI")
                                .unwrap_or_else(|_| {
                                    format!("{}/api/auth/oidc/callback", callback_base_url)
                                }),
                            scopes: std::env::var("KRONFORCE_OIDC_SCOPES")
                                .unwrap_or_else(|_| "openid email profile".to_string()),
                            role_claim: std::env::var("KRONFORCE_OIDC_ROLE_CLAIM")
                                .unwrap_or_else(|_| "groups".to_string()),
                            admin_values: std::env::var("KRONFORCE_OIDC_ADMIN_VALUES")
                                .unwrap_or_default()
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect(),
                            operator_values: std::env::var("KRONFORCE_OIDC_OPERATOR_VALUES")
                                .unwrap_or_default()
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect(),
                            default_role: std::env::var("KRONFORCE_OIDC_DEFAULT_ROLE")
                                .ok()
                                .and_then(|s| ApiKeyRole::from_str(&s))
                                .unwrap_or(ApiKeyRole::Viewer),
                            session_ttl_secs: std::env::var("KRONFORCE_OIDC_SESSION_TTL_SECS")
                                .ok()
                                .and_then(|s| s.parse().ok())
                                .unwrap_or(86400),
                        })
                    }
                    _ => None,
                }
            },
            tls_cert: std::env::var("KRONFORCE_TLS_CERT")
                .ok()
                .filter(|s| !s.is_empty()),
            tls_key: std::env::var("KRONFORCE_TLS_KEY")
                .ok()
                .filter(|s| !s.is_empty()),
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
    pub tls_cert: Option<String>,
    pub tls_key: Option<String>,
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
            tls_cert: std::env::var("KRONFORCE_TLS_CERT")
                .ok()
                .filter(|s| !s.is_empty()),
            tls_key: std::env::var("KRONFORCE_TLS_KEY")
                .ok()
                .filter(|s| !s.is_empty()),
        }
    }
}
