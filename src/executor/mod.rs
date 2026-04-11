//! Job execution engine.
//!
//! Runs jobs locally or dispatches them to remote agents, handles post-execution
//! processing (output rules, notifications, event logging), and manages
//! variable substitution in task fields.

mod dispatch;
mod local;
pub mod notifications;
pub mod output_rules;
pub mod scripts;
pub(crate) mod tasks;

pub use local::run_task;

use std::collections::HashMap;
use std::sync::Arc;

use regex::Regex;
use tracing::{error, warn};

use chrono::Utc;
use tokio::sync::{Mutex, oneshot};
use uuid::Uuid;

use crate::agent::AgentClient;
use crate::agent::protocol::JobDispatchRequest;
use crate::db::Db;
use crate::db::models::*;
use crate::error::AppError;
use crate::executor::scripts::ScriptStore;
use crate::scheduler::SchedulerCommand;

pub use local::{CapturedOutput, CommandResult};
pub(crate) use local::{
    DEFAULT_SCRIPT_TIMEOUT_SECS, MAX_SCRIPT_OPERATIONS, MAX_SCRIPT_STRING_SIZE, bytes_to_hex,
    calculate_retry_delay, hex_to_bytes, run_command, shell_escape, should_retry,
};

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
    pub(crate) live_output: Arc<dashmap::DashMap<Uuid, tokio::sync::broadcast::Sender<String>>>,
}

impl Executor {
    /// Creates a new executor with the given database, agent client, and scheduler channel.
    pub fn new(
        db: Db,
        agent_client: AgentClient,
        scheduler_tx: tokio::sync::mpsc::Sender<SchedulerCommand>,
        script_store: ScriptStore,
        live_output: Arc<dashmap::DashMap<Uuid, tokio::sync::broadcast::Sender<String>>>,
    ) -> Self {
        Self {
            db,
            agent_client,
            scheduler_tx,
            script_store,
            running: Arc::new(Mutex::new(HashMap::new())),
            live_output,
        }
    }

    /// Executes a job, routing it locally or to an agent based on `job.target`.
    pub async fn execute(
        &self,
        job: &Job,
        trigger: TriggerSource,
        callback_base_url: &str,
        params: Option<serde_json::Value>,
    ) -> Result<Uuid, AppError> {
        // Substitute global variables and params in task fields
        let mut job = job.clone();
        let db = self.db.clone();
        let task_clone = job.task.clone();
        let params_clone = params.clone();
        let substituted = tokio::task::spawn_blocking(move || {
            let vars = db.get_all_variables_map()?;
            Ok::<_, AppError>(substitute_variables(
                &task_clone,
                &vars,
                params_clone.as_ref(),
            ))
        })
        .await
        .map_err(|e| AppError::Internal(e.to_string()))??;
        if let Some(new_task) = substituted {
            job.task = new_task;
        }

        match &job.target {
            None | Some(AgentTarget::Local) => self.execute_local(&job, trigger, params).await,
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

/// Substitute `{{VAR_NAME}}` and `{{params.NAME}}` placeholders in task fields.
/// Returns None if no substitutions were needed, or Some(new_task) with resolved values.
pub fn substitute_variables(
    task: &TaskType,
    vars: &HashMap<String, String>,
    params: Option<&serde_json::Value>,
) -> Option<TaskType> {
    let json_str = serde_json::to_string(task).ok()?;
    if !json_str.contains("{{") {
        return None;
    }

    fn json_escape(value: &str) -> String {
        // serde_json::to_string wraps in quotes and escapes all special chars
        // We strip the outer quotes since we're embedding inside an existing JSON string
        match serde_json::to_string(value) {
            Ok(s) => s[1..s.len() - 1].to_string(),
            Err(_) => {
                // Fallback: manually escape dangerous chars
                value
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\r', "\\r")
                    .replace('\t', "\\t")
            }
        }
    }

    let mut had_substitution = false;
    let mut result = json_str;

    // Pass 1: {{params.NAME}} — from runtime trigger params
    if let Some(params) = params
        && let Some(obj) = params.as_object()
    {
        let re = Regex::new(r"\{\{params\.([A-Za-z0-9_]+)\}\}").ok()?;
        let src = result.clone();
        result = re
            .replace_all(&src, |caps: &regex::Captures| {
                let param_name = &caps[1];
                if let Some(val) = obj.get(param_name) {
                    had_substitution = true;
                    let s = match val {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    json_escape(&s)
                } else {
                    caps[0].to_string()
                }
            })
            .to_string();
    }

    // Pass 2: {{VAR_NAME}} — from global variables
    if !vars.is_empty() {
        let re = Regex::new(r"\{\{([A-Za-z0-9_]+)\}\}").ok()?;
        let src = result.clone();
        result = re
            .replace_all(&src, |caps: &regex::Captures| {
                let var_name = &caps[1];
                if let Some(value) = vars.get(var_name) {
                    had_substitution = true;
                    json_escape(value)
                } else {
                    warn!("unresolved variable reference: {{{{{}}}}}", var_name);
                    caps[0].to_string()
                }
            })
            .to_string();
    }

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
