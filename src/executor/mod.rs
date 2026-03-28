mod dispatch;
mod local;
pub mod notifications;
pub mod output_rules;
pub mod scripts;

pub use local::run_task;

use std::collections::HashMap;
use std::sync::Arc;

use regex::Regex;
use tracing::{error, warn};

use chrono::Utc;
use tokio::sync::{Mutex, oneshot};
use uuid::Uuid;

use crate::agent::AgentClient;
use crate::db::Db;
use crate::error::AppError;
use crate::models::*;
use crate::protocol::JobDispatchRequest;
use crate::scheduler::SchedulerCommand;
use crate::scripts::ScriptStore;

pub use local::{CapturedOutput, CommandResult};

struct RunningJob {
    cancel_tx: oneshot::Sender<()>,
}

/// Runs jobs locally or dispatches them to remote agents.
#[derive(Clone)]
pub struct Executor {
    db: Db,
    agent_client: AgentClient,
    scheduler_tx: tokio::sync::mpsc::Sender<SchedulerCommand>,
    script_store: ScriptStore,
    running: Arc<Mutex<HashMap<Uuid, RunningJob>>>,
}

impl Executor {
    /// Creates a new executor with the given database, agent client, and scheduler channel.
    pub fn new(
        db: Db,
        agent_client: AgentClient,
        scheduler_tx: tokio::sync::mpsc::Sender<SchedulerCommand>,
        script_store: ScriptStore,
    ) -> Self {
        Self {
            db,
            agent_client,
            scheduler_tx,
            script_store,
            running: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Executes a job, routing it locally or to an agent based on `job.target`.
    pub async fn execute(
        &self,
        job: &Job,
        trigger: TriggerSource,
        callback_base_url: &str,
    ) -> Result<Uuid, AppError> {
        // Substitute global variables in task fields
        let mut job = job.clone();
        let db = self.db.clone();
        let task_clone = job.task.clone();
        let substituted = tokio::task::spawn_blocking(move || {
            let vars = db.get_all_variables_map()?;
            Ok::<_, AppError>(substitute_variables(&task_clone, &vars))
        })
        .await
        .unwrap()?;
        if let Some(new_task) = substituted {
            job.task = new_task;
        }

        match &job.target {
            None | Some(AgentTarget::Local) => self.execute_local(&job, trigger).await,
            Some(AgentTarget::Agent { agent_id }) => {
                self.dispatch_to_agent(*agent_id, &job, trigger, callback_base_url)
                    .await
            }
            Some(AgentTarget::Tagged { tag }) => {
                self.dispatch_to_tagged(tag, &job, trigger, callback_base_url)
                    .await
            }
            Some(AgentTarget::Any) => self.dispatch_to_any(&job, trigger, callback_base_url).await,
            Some(AgentTarget::All) => self.dispatch_to_all(&job, trigger, callback_base_url).await,
        }
    }

    /// Cancels a running local execution by its ID, returning `true` if found.
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

/// Substitute `{{VAR_NAME}}` placeholders in task fields with variable values.
/// Returns None if no substitutions were needed, or Some(new_task) with resolved values.
pub fn substitute_variables(task: &TaskType, vars: &HashMap<String, String>) -> Option<TaskType> {
    if vars.is_empty() {
        return None;
    }
    let json_str = serde_json::to_string(task).ok()?;
    if !json_str.contains("{{") {
        return None;
    }

    let re = Regex::new(r"\{\{([A-Za-z0-9_]+)\}\}").unwrap();
    let mut had_substitution = false;
    let result = re.replace_all(&json_str, |caps: &regex::Captures| {
        let var_name = &caps[1];
        if let Some(value) = vars.get(var_name) {
            had_substitution = true;
            // JSON-escape the value for safe embedding in a JSON string
            let escaped = serde_json::to_string(value).unwrap();
            // Strip the surrounding quotes since we're inside an existing JSON string
            escaped[1..escaped.len() - 1].to_string()
        } else {
            warn!("unresolved variable reference: {{{{{}}}}}", var_name);
            caps[0].to_string()
        }
    });

    if !had_substitution {
        return None;
    }

    match serde_json::from_str(&result) {
        Ok(new_task) => Some(new_task),
        Err(e) => {
            error!("variable substitution produced invalid JSON: {}", e);
            None
        }
    }
}
