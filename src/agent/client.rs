use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use uuid::Uuid;

use crate::agent::protocol::{CancelRequest, JobDispatchRequest, JobDispatchResponse};
use crate::error::AppError;

/// HTTP client used by the controller to communicate with remote agents.
///
/// Authenticates outbound requests with a bearer token. The token is selected
/// per-agent in this order:
///   1. The token captured at registration time (raw bearer the agent sent).
///   2. The fallback dispatch key from env (`KRONFORCE_DISPATCH_KEY`,
///      `KRONFORCE_AGENT_KEY`, then `KRONFORCE_BOOTSTRAP_AGENT_KEY`).
///
/// The per-agent map lives in process memory only — API keys are hashed in the
/// DB, so the controller has no other way to recover the raw value once an
/// agent has registered.
#[derive(Clone)]
pub struct AgentClient {
    client: reqwest::Client,
    dispatch_key: Option<String>,
    dispatch_tokens: Arc<RwLock<HashMap<Uuid, String>>>,
}

/// Env vars the controller checks (in order) to find the bearer token it
/// should send to agents on `/execute`, `/cancel`, and `/shutdown`.
const DISPATCH_KEY_ENV_VARS: &[&str] = &[
    "KRONFORCE_DISPATCH_KEY",
    "KRONFORCE_AGENT_KEY",
    "KRONFORCE_BOOTSTRAP_AGENT_KEY",
];

/// Picks the first non-empty value from `DISPATCH_KEY_ENV_VARS` using the
/// provided lookup. Extracted so tests can inject env values without mutating
/// process-global state.
fn resolve_dispatch_key<F>(get_var: F) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    DISPATCH_KEY_ENV_VARS
        .iter()
        .find_map(|name| get_var(name).filter(|v| !v.is_empty()))
}

impl Default for AgentClient {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentClient {
    /// Creates a new agent client with a 10-second request timeout.
    pub fn new() -> Self {
        let dispatch_key = resolve_dispatch_key(|name| std::env::var(name).ok());
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            dispatch_key,
            dispatch_tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Records the raw bearer token an agent used when it registered, so the
    /// controller can authenticate dispatch requests back to it.
    pub fn register_dispatch_token(&self, agent_id: Uuid, token: String) {
        if token.is_empty() {
            return;
        }
        if let Ok(mut guard) = self.dispatch_tokens.write() {
            guard.insert(agent_id, token);
        }
    }

    /// Drops the per-agent token (used on deregistration).
    pub fn forget_dispatch_token(&self, agent_id: Uuid) {
        if let Ok(mut guard) = self.dispatch_tokens.write() {
            guard.remove(&agent_id);
        }
    }

    /// Returns the bearer token to use for the given agent, preferring the
    /// per-agent token from registration and falling back to the env-configured
    /// dispatch key.
    fn token_for(&self, agent_id: Uuid) -> Option<String> {
        if let Ok(guard) = self.dispatch_tokens.read()
            && let Some(t) = guard.get(&agent_id)
        {
            return Some(t.clone());
        }
        self.dispatch_key.clone()
    }

    /// Sends a job dispatch request to an agent and returns its acceptance response.
    pub async fn dispatch_job(
        &self,
        agent_id: Uuid,
        agent_address: &str,
        agent_port: u16,
        request: &JobDispatchRequest,
    ) -> Result<JobDispatchResponse, AppError> {
        let scheme = if agent_port == 443 { "https" } else { "http" };
        let url = format!("{scheme}://{}:{}/execute", agent_address, agent_port);
        let mut req = self.client.post(&url).json(request);
        if let Some(key) = self.token_for(agent_id) {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        let resp = req
            .send()
            .await
            .map_err(|e| AppError::AgentError(format!("failed to dispatch to agent: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::AgentError(format!(
                "agent rejected dispatch: {status} {body}"
            )));
        }

        resp.json()
            .await
            .map_err(|e| AppError::AgentError(format!("invalid agent response: {e}")))
    }

    /// Sends a cancellation request to an agent for the given execution.
    pub async fn cancel_execution(
        &self,
        agent_id: Uuid,
        agent_address: &str,
        agent_port: u16,
        execution_id: Uuid,
    ) -> Result<(), AppError> {
        let scheme = if agent_port == 443 { "https" } else { "http" };
        let url = format!("{scheme}://{}:{}/cancel", agent_address, agent_port);
        let cancel = CancelRequest { execution_id };
        let mut req = self.client.post(&url).json(&cancel);
        if let Some(key) = self.token_for(agent_id) {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        let resp = req
            .send()
            .await
            .map_err(|e| AppError::AgentError(format!("failed to cancel on agent: {e}")))?;

        if !resp.status().is_success() {
            return Err(AppError::AgentError("agent cancel failed".into()));
        }
        Ok(())
    }

    /// Sends a best-effort shutdown request to an agent. Does not fail if the agent is unreachable.
    pub async fn shutdown_agent(
        &self,
        agent_id: Uuid,
        agent_address: &str,
        agent_port: u16,
    ) -> Result<(), AppError> {
        let scheme = if agent_port == 443 { "https" } else { "http" };
        let url = format!("{scheme}://{}:{}/shutdown", agent_address, agent_port);
        let mut req = self.client.post(&url);
        if let Some(key) = self.token_for(agent_id) {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        let _ = req.send().await; // Best-effort, don't fail if agent is already down
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup<'a>(
        map: &'a HashMap<&'static str, &'static str>,
    ) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| map.get(name).map(|s| s.to_string())
    }

    #[test]
    fn dispatch_key_none_when_no_env_set() {
        let env: HashMap<&str, &str> = HashMap::new();
        assert_eq!(resolve_dispatch_key(lookup(&env)), None);
    }

    #[test]
    fn dispatch_key_picks_up_agent_key() {
        // Regression: controller used to ignore KRONFORCE_AGENT_KEY, so dispatch
        // failed with 401 even when both sides set the same key per the docs.
        let env = HashMap::from([("KRONFORCE_AGENT_KEY", "kf_agent_secret")]);
        assert_eq!(
            resolve_dispatch_key(lookup(&env)),
            Some("kf_agent_secret".to_string())
        );
    }

    #[test]
    fn dispatch_key_picks_up_bootstrap_key() {
        let env = HashMap::from([("KRONFORCE_BOOTSTRAP_AGENT_KEY", "kf_bootstrap")]);
        assert_eq!(
            resolve_dispatch_key(lookup(&env)),
            Some("kf_bootstrap".to_string())
        );
    }

    #[test]
    fn dispatch_key_prefers_explicit_dispatch_var() {
        let env = HashMap::from([
            ("KRONFORCE_DISPATCH_KEY", "kf_explicit"),
            ("KRONFORCE_AGENT_KEY", "kf_agent"),
            ("KRONFORCE_BOOTSTRAP_AGENT_KEY", "kf_bootstrap"),
        ]);
        assert_eq!(
            resolve_dispatch_key(lookup(&env)),
            Some("kf_explicit".to_string())
        );
    }

    #[test]
    fn dispatch_key_skips_empty_value_and_falls_through() {
        let env = HashMap::from([
            ("KRONFORCE_DISPATCH_KEY", ""),
            ("KRONFORCE_AGENT_KEY", "kf_agent"),
        ]);
        assert_eq!(
            resolve_dispatch_key(lookup(&env)),
            Some("kf_agent".to_string())
        );
    }

    #[test]
    fn token_for_returns_per_agent_token_when_registered() {
        let client = AgentClient::new();
        let agent_id = Uuid::new_v4();
        client.register_dispatch_token(agent_id, "kf_per_agent".into());
        assert_eq!(client.token_for(agent_id), Some("kf_per_agent".into()));
    }

    #[test]
    fn token_for_falls_back_to_dispatch_key_when_no_per_agent_token() {
        let client = AgentClient {
            client: reqwest::Client::new(),
            dispatch_key: Some("kf_env".into()),
            dispatch_tokens: Arc::new(RwLock::new(HashMap::new())),
        };
        let agent_id = Uuid::new_v4();
        assert_eq!(client.token_for(agent_id), Some("kf_env".into()));
    }

    #[test]
    fn token_for_per_agent_overrides_dispatch_key() {
        let client = AgentClient {
            client: reqwest::Client::new(),
            dispatch_key: Some("kf_env".into()),
            dispatch_tokens: Arc::new(RwLock::new(HashMap::new())),
        };
        let agent_id = Uuid::new_v4();
        client.register_dispatch_token(agent_id, "kf_per_agent".into());
        assert_eq!(client.token_for(agent_id), Some("kf_per_agent".into()));
    }

    #[test]
    fn forget_dispatch_token_reverts_to_env_fallback() {
        let client = AgentClient {
            client: reqwest::Client::new(),
            dispatch_key: Some("kf_env".into()),
            dispatch_tokens: Arc::new(RwLock::new(HashMap::new())),
        };
        let agent_id = Uuid::new_v4();
        client.register_dispatch_token(agent_id, "kf_per_agent".into());
        client.forget_dispatch_token(agent_id);
        assert_eq!(client.token_for(agent_id), Some("kf_env".into()));
    }

    #[test]
    fn register_dispatch_token_ignores_empty() {
        let client = AgentClient {
            client: reqwest::Client::new(),
            dispatch_key: None,
            dispatch_tokens: Arc::new(RwLock::new(HashMap::new())),
        };
        let agent_id = Uuid::new_v4();
        client.register_dispatch_token(agent_id, String::new());
        assert_eq!(client.token_for(agent_id), None);
    }
}
