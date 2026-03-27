use uuid::Uuid;

use crate::error::AppError;
use crate::protocol::{CancelRequest, JobDispatchRequest, JobDispatchResponse};

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
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();
        Self { client }
    }

    pub async fn dispatch_job(
        &self,
        agent_address: &str,
        agent_port: u16,
        request: &JobDispatchRequest,
    ) -> Result<JobDispatchResponse, AppError> {
        let url = format!("http://{}:{}/execute", agent_address, agent_port);
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

    pub async fn cancel_execution(
        &self,
        agent_address: &str,
        agent_port: u16,
        execution_id: Uuid,
    ) -> Result<(), AppError> {
        let url = format!("http://{}:{}/cancel", agent_address, agent_port);
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

    pub async fn shutdown_agent(
        &self,
        agent_address: &str,
        agent_port: u16,
    ) -> Result<(), AppError> {
        let url = format!("http://{}:{}/shutdown", agent_address, agent_port);
        let _ = self.client.post(&url).send().await; // Best-effort, don't fail if agent is already down
        Ok(())
    }
}
