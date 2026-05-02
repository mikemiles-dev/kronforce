//! Job execution engine.
//!
//! Runs jobs locally or dispatches them to remote agents, handles post-execution
//! processing (output rules, notifications, event logging), and manages
//! variable substitution in task fields.

mod connections;
mod dispatch;
mod local;
pub mod notifications;
pub mod output_rules;
mod post_execution;
mod runner;
pub mod scripts;
pub(crate) mod tasks;
mod utils;

pub(crate) use runner::run_command;
pub use runner::{CapturedOutput, CommandResult, run_task};
pub(crate) use utils::{
    DEFAULT_SCRIPT_TIMEOUT_SECS, MAX_SCRIPT_OPERATIONS, MAX_SCRIPT_STRING_SIZE, bytes_to_hex,
    calculate_retry_delay, hex_to_bytes, shell_escape, should_retry,
};

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
    agent_notify: Arc<dashmap::DashMap<Uuid, Arc<tokio::sync::Notify>>>,
}

impl Executor {
    /// Creates a new executor with the given database, agent client, and scheduler channel.
    pub fn new(
        db: Db,
        agent_client: AgentClient,
        scheduler_tx: tokio::sync::mpsc::Sender<SchedulerCommand>,
        script_store: ScriptStore,
        live_output: Arc<dashmap::DashMap<Uuid, tokio::sync::broadcast::Sender<String>>>,
        agent_notify: Arc<dashmap::DashMap<Uuid, Arc<tokio::sync::Notify>>>,
    ) -> Self {
        Self {
            db,
            agent_client,
            scheduler_tx,
            script_store,
            running: Arc::new(Mutex::new(HashMap::new())),
            live_output,
            agent_notify,
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
            substitute_variables(&task_clone, &vars, params_clone.as_ref())
        })
        .await
        .map_err(|e| AppError::Internal(e.to_string()))??;
        if let Some(new_task) = substituted {
            job.task = new_task;
        }

        // Resolve connection references (merge named connection credentials into task)
        let db2 = self.db.clone();
        let task_for_conn = job.task.clone();
        let resolved_conn = tokio::task::spawn_blocking(move || {
            connections::resolve_connections(&task_for_conn, &db2)
        })
        .await
        .map_err(|e| AppError::Internal(e.to_string()))??;
        if let Some(new_task) = resolved_conn {
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
///
/// Returns:
/// - `Ok(None)` — no placeholders matched (nothing to do; caller should use the original task).
/// - `Ok(Some(new_task))` — every matched placeholder was resolved.
/// - `Err(_)` — at least one placeholder matched the syntax but had no corresponding
///   variable/param, or the substituted result was no longer valid JSON. Surfacing this
///   as an error prevents jobs from silently running with literal `{{X}}` in their fields.
pub fn substitute_variables(
    task: &TaskType,
    vars: &HashMap<String, String>,
    params: Option<&serde_json::Value>,
) -> Result<Option<TaskType>, AppError> {
    let json_str = serde_json::to_string(task).map_err(|e| {
        AppError::Internal(format!("failed to serialize task for substitution: {e}"))
    })?;
    if !json_str.contains("{{") {
        return Ok(None);
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
    let mut unresolved: Vec<String> = Vec::new();
    let mut result = json_str;

    // Pass 1: {{params.NAME}} — from runtime trigger params.
    // The placeholder syntax is reserved, so an unrecognized name is always an error
    // even when params=None (the job author wrote {{params.X}} expecting a runtime value).
    let re_params = Regex::new(r"\{\{params\.([A-Za-z0-9_]+)\}\}")
        .map_err(|e| AppError::Internal(format!("regex compile failed: {e}")))?;
    let params_obj = params.and_then(|p| p.as_object());
    let src = result.clone();
    result = re_params
        .replace_all(&src, |caps: &regex::Captures| {
            let param_name = &caps[1];
            match params_obj.and_then(|o| o.get(param_name)) {
                Some(val) => {
                    had_substitution = true;
                    let s = match val {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    json_escape(&s)
                }
                None => {
                    unresolved.push(format!("params.{param_name}"));
                    caps[0].to_string()
                }
            }
        })
        .to_string();

    // Pass 2: {{VAR_NAME}} — from global variables.
    let re_vars = Regex::new(r"\{\{([A-Za-z0-9_]+)\}\}")
        .map_err(|e| AppError::Internal(format!("regex compile failed: {e}")))?;
    let src = result.clone();
    result = re_vars
        .replace_all(&src, |caps: &regex::Captures| {
            let var_name = &caps[1];
            match vars.get(var_name) {
                Some(value) => {
                    had_substitution = true;
                    json_escape(value)
                }
                None => {
                    unresolved.push(var_name.to_string());
                    caps[0].to_string()
                }
            }
        })
        .to_string();

    if !unresolved.is_empty() {
        unresolved.sort();
        unresolved.dedup();
        warn!(
            "unresolved variable references in task: {{{{{}}}}}",
            unresolved.join("}}, {{")
        );
        return Err(AppError::BadRequest(format!(
            "unresolved variable reference(s): {{{{{}}}}}",
            unresolved.join("}}, {{")
        )));
    }

    if !had_substitution {
        return Ok(None);
    }

    serde_json::from_str(&result).map(Some).map_err(|e| {
        error!("variable substitution produced invalid JSON: {}", e);
        AppError::Internal(format!("variable substitution produced invalid JSON: {e}"))
    })
}
