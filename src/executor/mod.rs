mod dispatch;
mod local;

pub use local::run_task;

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::{Mutex, oneshot};
use uuid::Uuid;

use crate::agent::AgentClient;
use crate::db::Db;
use crate::error::AppError;
use crate::models::*;
use crate::protocol::JobDispatchRequest;

pub use local::{CapturedOutput, CommandResult};

struct RunningJob {
    cancel_tx: oneshot::Sender<()>,
}

#[derive(Clone)]
pub struct Executor {
    db: Db,
    agent_client: AgentClient,
    scheduler_tx: tokio::sync::mpsc::Sender<crate::scheduler::SchedulerCommand>,
    script_store: crate::scripts::ScriptStore,
    running: Arc<Mutex<HashMap<Uuid, RunningJob>>>,
}

impl Executor {
    pub fn new(
        db: Db,
        agent_client: AgentClient,
        scheduler_tx: tokio::sync::mpsc::Sender<crate::scheduler::SchedulerCommand>,
        script_store: crate::scripts::ScriptStore,
    ) -> Self {
        Self {
            db,
            agent_client,
            scheduler_tx,
            script_store,
            running: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn execute(
        &self,
        job: &Job,
        trigger: TriggerSource,
        callback_base_url: &str,
    ) -> Result<Uuid, AppError> {
        match &job.target {
            None | Some(AgentTarget::Local) => self.execute_local(job, trigger).await,
            Some(AgentTarget::Agent { agent_id }) => {
                self.dispatch_to_agent(*agent_id, job, trigger, callback_base_url)
                    .await
            }
            Some(AgentTarget::Tagged { tag }) => {
                self.dispatch_to_tagged(tag, job, trigger, callback_base_url)
                    .await
            }
            Some(AgentTarget::Any) => self.dispatch_to_any(job, trigger, callback_base_url).await,
            Some(AgentTarget::All) => self.dispatch_to_all(job, trigger, callback_base_url).await,
        }
    }

    pub async fn cancel(&self, execution_id: Uuid) -> bool {
        let mut running = self.running.lock().await;
        if let Some(job) = running.remove(&execution_id) {
            let _ = job.cancel_tx.send(());
            true
        } else {
            false
        }
    }
}
