//! Command handling methods: manual triggers, cancellations, events, reloads, and cache management.

use std::collections::HashMap;

use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::SchedulerCommand;
use crate::db::models::*;

impl super::Scheduler {
    pub(crate) async fn handle_command(&mut self, cmd: SchedulerCommand) {
        match cmd {
            SchedulerCommand::Reload => {
                debug!("reloading jobs");
                self.invalidate_cache();
            }
            SchedulerCommand::TriggerNow {
                job_id,
                skip_deps,
                params,
            } => {
                let db = self.db.clone();
                let job = tokio::task::spawn_blocking(move || db.get_job(job_id))
                    .await
                    .unwrap_or(Ok(None));
                match job {
                    Ok(Some(job)) => {
                        self.fire_job(&job, TriggerSource::Api, skip_deps, params)
                            .await;
                    }
                    Ok(None) => {
                        warn!("trigger: job {} not found", job_id);
                    }
                    Err(e) => {
                        error!("trigger: error loading job {}: {e}", job_id);
                    }
                }
            }
            SchedulerCommand::CancelExecution(exec_id) => {
                self.cancel_execution(exec_id).await;
            }
            SchedulerCommand::EventOccurred(event) => {
                self.handle_event(event).await;
            }
            SchedulerCommand::RetryExecution {
                job_id,
                original_execution_id,
                attempt,
            } => {
                let db = self.db.clone();
                let job = tokio::task::spawn_blocking(move || db.get_job(job_id))
                    .await
                    .unwrap_or(Ok(None));
                match job {
                    Ok(Some(job)) => {
                        let trigger = TriggerSource::Retry {
                            original_execution_id,
                            attempt,
                        };
                        info!(
                            "retrying job {} ({}) attempt {}/{}",
                            job.name,
                            job.id,
                            attempt,
                            job.retry_max + 1
                        );
                        match self
                            .executor
                            .execute(&job, trigger, &self.callback_base_url, None)
                            .await
                        {
                            Ok(exec_id) => {
                                info!("retry execution {} started for job {}", exec_id, job.name);
                            }
                            Err(e) => {
                                error!("retry failed for job {}: {e}", job.name);
                            }
                        }
                    }
                    Ok(None) => {
                        warn!("retry: job {} not found", job_id);
                    }
                    Err(e) => {
                        error!("retry: error loading job {}: {e}", job_id);
                    }
                }
            }
        }
    }

    pub(crate) async fn fire_job(
        &self,
        job: &Job,
        trigger: TriggerSource,
        skip_deps: bool,
        params: Option<serde_json::Value>,
    ) {
        if !skip_deps && !job.depends_on.is_empty() {
            let dag = self.dag.clone();
            let deps = job.depends_on.clone();
            let satisfied = tokio::task::spawn_blocking(move || dag.deps_satisfied(&deps))
                .await
                .unwrap_or(Ok(false));

            match satisfied {
                Ok(true) => {}
                Ok(false) => {
                    debug!(
                        "skipping job {} ({}): dependencies not satisfied",
                        job.name, job.id
                    );
                    return;
                }
                Err(e) => {
                    error!("error checking deps for job {}: {e}", job.name);
                    return;
                }
            }
        }

        // Concurrency control: skip if at limit
        if job.max_concurrent > 0 {
            let db = self.db.clone();
            let job_id = job.id;
            let count =
                tokio::task::spawn_blocking(move || db.count_running_executions_for_job(job_id))
                    .await
                    .unwrap_or(Ok(0));
            match count {
                Ok(c) if c >= job.max_concurrent => {
                    debug!(
                        "skipping job {} ({}): at concurrency limit ({}/{})",
                        job.name, job.id, c, job.max_concurrent
                    );
                    return;
                }
                Err(e) => {
                    error!("error checking concurrency for job {}: {e}", job.name);
                    return;
                }
                _ => {}
            }
        }

        match self
            .executor
            .execute(job, trigger, &self.callback_base_url, params)
            .await
        {
            Ok(exec_id) => {
                info!(
                    "fired job {} ({}) -> execution {}",
                    job.name, job.id, exec_id
                );
            }
            Err(e) => {
                error!("failed to execute job {} ({}): {e}", job.name, job.id);
            }
        }
    }

    /// Cancels an execution, trying locally first then on the remote agent.
    pub(crate) async fn cancel_execution(&self, exec_id: Uuid) {
        if self.executor.cancel(exec_id).await {
            info!("cancelled local execution {}", exec_id);
            return;
        }

        let db = self.db.clone();
        let exec = tokio::task::spawn_blocking(move || db.get_execution(exec_id))
            .await
            .unwrap_or(Ok(None));

        if let Ok(Some(exec)) = exec
            && let Some(agent_id) = exec.agent_id
        {
            let db = self.db.clone();
            if let Ok(Some(agent)) = tokio::task::spawn_blocking(move || db.get_agent(agent_id))
                .await
                .unwrap_or(Ok(None))
            {
                match self
                    .agent_client
                    .cancel_execution(agent.id, &agent.address, agent.port, exec_id)
                    .await
                {
                    Ok(_) => info!(
                        "cancelled remote execution {} on agent {}",
                        exec_id, agent.name
                    ),
                    Err(e) => error!("failed to cancel on agent: {e}"),
                }
                return;
            }
        }

        // Force-cancel: if the execution is still marked as running/pending in the DB
        // but we can't find it in the running map (e.g., controller restarted, process died),
        // update the DB status directly.
        let db = self.db.clone();
        let force_result = tokio::task::spawn_blocking(move || {
            if let Ok(Some(exec)) = db.get_execution(exec_id) {
                if exec.status == ExecutionStatus::Running
                    || exec.status == ExecutionStatus::Pending
                {
                    db.update_execution_status(exec_id, ExecutionStatus::Cancelled)
                } else {
                    Ok(())
                }
            } else {
                Ok(())
            }
        })
        .await;
        match force_result {
            Ok(Ok(())) => info!("force-cancelled execution {} in database", exec_id),
            _ => warn!("cancel: execution {} could not be cancelled", exec_id),
        }
    }

    pub(crate) async fn handle_event(&mut self, event: Event) {
        // Load jobs if not cached
        if self.jobs_cache.is_none() {
            self.reload_jobs().await;
        }

        let jobs = match &self.jobs_cache {
            Some(jobs) => jobs.clone(),
            None => return,
        };

        // Build job name lookup from cache for event matching
        let job_names: HashMap<Uuid, String> =
            jobs.iter().map(|j| (j.id, j.name.clone())).collect();

        for job in &jobs {
            if job.status != JobStatus::Scheduled {
                continue;
            }

            if let ScheduleKind::Event(ref config) = job.schedule {
                if !Self::event_matches(&event, config, &job_names) {
                    continue;
                }

                // Avoid infinite loops: don't trigger from events caused by event-triggered jobs
                // (events from TriggerSource::Event executions)
                info!("event '{}' matched job '{}', firing", event.kind, job.name);
                self.fire_job(
                    job,
                    TriggerSource::Event { event_id: event.id },
                    false,
                    None,
                )
                .await;
            }
        }
    }

    pub(crate) async fn reload_jobs(&mut self) {
        let db = self.db.clone();
        match tokio::task::spawn_blocking(move || db.get_active_cron_jobs()).await {
            Ok(Ok(mut jobs)) => {
                // Sort by priority descending so higher priority jobs fire first
                jobs.sort_by(|a, b| b.priority.cmp(&a.priority));
                debug!("loaded {} active jobs", jobs.len());
                self.jobs_cache = Some(jobs);
            }
            Ok(Err(e)) => {
                error!("failed to load jobs: {e}");
                // Keep stale cache so jobs still fire rather than silently skipping
            }
            Err(e) => {
                error!("failed to load jobs (join): {e}");
                // Keep stale cache so jobs still fire rather than silently skipping
            }
        }
    }

    pub(crate) fn invalidate_cache(&mut self) {
        self.jobs_cache = None;
        self.next_fire_times.clear();
    }

    fn event_matches(
        event: &Event,
        config: &EventTriggerConfig,
        job_names: &HashMap<Uuid, String>,
    ) -> bool {
        if !Self::pattern_matches(&config.kind_pattern, &event.kind) {
            return false;
        }
        if let Some(ref sev) = config.severity
            && event.severity != *sev
        {
            return false;
        }
        if let Some(ref name_filter) = config.job_name_filter {
            let filter_lower = name_filter.to_lowercase();
            // Check against the source job's actual name (via job_id on the event)
            let job_name_match = event
                .job_id
                .and_then(|id| job_names.get(&id))
                .map(|name| name.to_lowercase().contains(&filter_lower))
                .unwrap_or(false);
            // Also check message text as fallback for non-job events
            let message_match = event.message.to_lowercase().contains(&filter_lower);
            if !job_name_match && !message_match {
                return false;
            }
        }
        true
    }

    /// Matches event kind patterns:
    /// - `"*"` matches everything
    /// - `"foo.*"` matches `"foo.bar"`, `"foo.baz.qux"` (namespace match: requires dot separator)
    /// - `"foo*"` matches anything starting with `"foo"` (prefix match)
    /// - exact string equality otherwise
    fn pattern_matches(pattern: &str, value: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        if let Some(prefix) = pattern.strip_suffix(".*") {
            // Namespace match: value must start with prefix followed by a dot
            return value.starts_with(prefix)
                && value.len() > prefix.len()
                && value.as_bytes()[prefix.len()] == b'.';
        }
        if let Some(prefix) = pattern.strip_suffix('*') {
            return value.starts_with(prefix);
        }
        pattern == value
    }
}
