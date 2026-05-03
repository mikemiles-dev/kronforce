use uuid::Uuid;

use crate::agent::protocol::{CancelRequest, JobDispatchRequest, JobDispatchResponse};
use crate::error::AppError;

/// HTTP client used by the controller to communicate with remote agents.
#[derive(Clone)]
pub struct AgentClient {
    client: reqwest::Client,
    /// Shared secret sent to agents to authenticate controller → agent requests.
    /// Resolved from `KRONFORCE_DISPATCH_KEY`, `KRONFORCE_AGENT_KEY`, or
    /// `KRONFORCE_BOOTSTRAP_AGENT_KEY` (in that order). Most deployments set
    /// `KRONFORCE_AGENT_KEY` to the same value on the controller and the agent.
    dispatch_key: Option<String>,
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
        }
    }

    /// Sends a job dispatch request to an agent and returns its acceptance response.
    pub async fn dispatch_job(
        &self,
        agent_address: &str,
        agent_port: u16,
        request: &JobDispatchRequest,
    ) -> Result<JobDispatchResponse, AppError> {
        let scheme = if agent_port == 443 { "https" } else { "http" };
        let url = format!("{scheme}://{}:{}/execute", agent_address, agent_port);
        let mut req = self.client.post(&url).json(request);
        if let Some(ref key) = self.dispatch_key {
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
        agent_address: &str,
        agent_port: u16,
        execution_id: Uuid,
    ) -> Result<(), AppError> {
        let scheme = if agent_port == 443 { "https" } else { "http" };
        let url = format!("{scheme}://{}:{}/cancel", agent_address, agent_port);
        let cancel = CancelRequest { execution_id };
        let mut req = self.client.post(&url).json(&cancel);
        if let Some(ref key) = self.dispatch_key {
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
        agent_address: &str,
        agent_port: u16,
    ) -> Result<(), AppError> {
        let scheme = if agent_port == 443 { "https" } else { "http" };
        let url = format!("{scheme}://{}:{}/shutdown", agent_address, agent_port);
        let mut req = self.client.post(&url);
        if let Some(ref key) = self.dispatch_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        let _ = req.send().await; // Best-effort, don't fail if agent is already down
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

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
}
