use uuid::Uuid;

use crate::agent::protocol::{CancelRequest, JobDispatchRequest, JobDispatchResponse};
use crate::error::AppError;

/// HTTP client used by the controller to communicate with remote agents.
#[derive(Clone)]
pub struct AgentClient {
    client: reqwest::Client,
}

impl Default for AgentClient {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentClient {
    /// Creates a new agent client with a 10-second request timeout.
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { client }
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
        let resp = self
            .client
            .post(&url)
            .json(request)
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
        let req = CancelRequest { execution_id };
        let resp = self
            .client
            .post(&url)
            .json(&req)
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
        let _ = self.client.post(&url).send().await; // Best-effort, don't fail if agent is already down
        Ok(())
    }
}
