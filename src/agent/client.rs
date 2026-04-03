use uuid::Uuid;

use crate::agent::protocol::{CancelRequest, JobDispatchRequest, JobDispatchResponse};
use crate::error::AppError;

/// HTTP client used by the controller to communicate with remote agents.
#[derive(Clone)]
pub struct AgentClient {
    client: reqwest::Client,
    /// Shared secret sent to agents to authenticate controller → agent requests.
    /// Set via KRONFORCE_AGENT_KEY on the controller (same key agents use).
    dispatch_key: Option<String>,
}

impl Default for AgentClient {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentClient {
    /// Creates a new agent client with a 10-second request timeout.
    pub fn new() -> Self {
        let dispatch_key = std::env::var("KRONFORCE_BOOTSTRAP_AGENT_KEY")
            .or_else(|_| std::env::var("KRONFORCE_DISPATCH_KEY"))
            .ok()
            .filter(|s| !s.is_empty());
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
