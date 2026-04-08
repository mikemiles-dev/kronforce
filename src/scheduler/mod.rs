//! Job scheduling loop and cron expression parsing.
//!
//! Fires jobs based on cron schedules, one-shot timers, and event triggers.
//! Accepts commands via an mpsc channel for reloads, manual triggers, and cancellations.

pub mod cron_parser;

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use self::cron_parser::CronSchedule;
use crate::agent::AgentClient;
use crate::config::ControllerConfig;
use crate::dag::DagResolver;
use crate::db::Db;
use crate::db::models::*;
use crate::executor::Executor;

/// Commands sent to the scheduler via its mpsc channel.
pub enum SchedulerCommand {
    /// Invalidate the job cache and reload from the database.
    Reload,
    /// Immediately fire the job with the given ID.
    /// The bool flag indicates whether to skip dependency checks.
    TriggerNow(Uuid, bool),
    /// Cancel the execution with the given ID.
    CancelExecution(Uuid),
    /// Notify the scheduler of a new event for event-triggered jobs.
    EventOccurred(Event),
    /// Retry a failed execution with the given attempt number.
    RetryExecution {
        job_id: Uuid,
        original_execution_id: Uuid,
        attempt: u32,
    },
}

/// Core scheduling loop that fires jobs based on cron, one-shot, and event triggers.
pub struct Scheduler {
    db: Db,
    executor: Executor,
    dag: DagResolver,
    agent_client: AgentClient,
    rx: mpsc::Receiver<SchedulerCommand>,
    tick_interval: std::time::Duration,
    callback_base_url: String,
    next_fire_times: HashMap<Uuid, DateTime<Utc>>,
    last_fired: HashMap<Uuid, DateTime<Utc>>,
    jobs_cache: Option<Vec<Job>>,
}

impl Scheduler {
    /// Creates a new scheduler with the provided dependencies and configuration.
    pub fn new(
        db: Db,
        executor: Executor,
        dag: DagResolver,
        rx: mpsc::Receiver<SchedulerCommand>,
        config: &ControllerConfig,
        agent_client: AgentClient,
    ) -> Self {
        Self {
            db,
            executor,
            dag,
            agent_client,
            rx,
            tick_interval: config.tick_interval,
            callback_base_url: config.callback_base_url.clone(),
            next_fire_times: HashMap::new(),
            last_fired: HashMap::new(),
            jobs_cache: None,
        }
    }

    /// Starts the scheduler tick loop, consuming `self`. Runs until the process exits.
    pub async fn run(mut self) {
        info!("scheduler started, tick interval: {:?}", self.tick_interval);
        let mut interval = tokio::time::interval(self.tick_interval);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.tick().await;
                }
                Some(cmd) = self.rx.recv() => {
                    self.handle_command(cmd).await;
                }
            }
        }
    }

    async fn tick(&mut self) {
        let now = Utc::now();

        if self.jobs_cache.is_none() {
            self.reload_jobs().await;
        }

        let jobs = match &self.jobs_cache {
            Some(jobs) => jobs.clone(),
            None => return,
        };

        for job in &jobs {
            if job.status != JobStatus::Scheduled {
                continue;
            }

            // Check schedule window constraints
            if let Some(starts_at) = job.starts_at {
                if now < starts_at {
                    continue;
                }
            }
            if let Some(expires_at) = job.expires_at {
                if now > expires_at {
                    debug!(
                        "job {} ({}) has expired, marking as unscheduled",
                        job.name, job.id
                    );
                    let mut updated = job.clone();
                    updated.status = JobStatus::Unscheduled;
                    updated.updated_at = Utc::now();
                    let db = self.db.clone();
                    let _ =
                        tokio::task::spawn_blocking(move || db.update_job(&updated)).await;
                    self.invalidate_cache();
                    continue;
                }
            }

            match &job.schedule {
                ScheduleKind::Cron(expr) => {
                    self.check_cron_job(job, &expr.0, now).await;
                }
                ScheduleKind::OneShot(fire_at) => {
                    if now >= *fire_at {
                        self.fire_job(job, TriggerSource::Scheduler, false).await;
                        let mut updated = job.clone();
                        updated.status = JobStatus::Unscheduled;
                        updated.updated_at = Utc::now();
                        let db = self.db.clone();
                        let _ = tokio::task::spawn_blocking(move || db.update_job(&updated)).await;
                        self.invalidate_cache();
                    }
                }
                ScheduleKind::OnDemand | ScheduleKind::Event(_) => {}
            }
        }
    }

    async fn check_cron_job(&mut self, job: &Job, cron_expr: &str, now: DateTime<Utc>) {
        let next_fire = self.next_fire_times.get(&job.id).copied();

        let fire_time = match next_fire {
            Some(t) => t,
            None => {
                let Ok(schedule) = CronSchedule::parse(cron_expr) else {
                    warn!("invalid cron for job {}: {}", job.name, cron_expr);
                    return;
                };
                match schedule.next_after(now - chrono::Duration::seconds(1)) {
                    Some(t) => {
                        self.next_fire_times.insert(job.id, t);
                        t
                    }
                    None => return,
                }
            }
        };

        if now >= fire_time {
            // Prevent double-fire: skip if we already fired at this time
            if let Some(last) = self.last_fired.get(&job.id)
                && *last >= fire_time
            {
                // Already fired for this time slot, just recalculate next
                let Ok(schedule) = CronSchedule::parse(cron_expr) else {
                    return;
                };
                if let Some(next) = schedule.next_after(now) {
                    self.next_fire_times.insert(job.id, next);
                }
                return;
            }

            self.last_fired.insert(job.id, fire_time);
            self.fire_job(job, TriggerSource::Scheduler, false).await;

            let Ok(schedule) = CronSchedule::parse(cron_expr) else {
                return;
            };
            if let Some(next) = schedule.next_after(now) {
                self.next_fire_times.insert(job.id, next);
            }
        }
    }

    async fn fire_job(&self, job: &Job, trigger: TriggerSource, skip_deps: bool) {
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

        match self
            .executor
            .execute(job, trigger, &self.callback_base_url)
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

    async fn handle_command(&mut self, cmd: SchedulerCommand) {
        match cmd {
            SchedulerCommand::Reload => {
                debug!("reloading jobs");
                self.invalidate_cache();
            }
            SchedulerCommand::TriggerNow(job_id, skip_deps) => {
                let db = self.db.clone();
                let job = tokio::task::spawn_blocking(move || db.get_job(job_id))
                    .await
                    .unwrap_or(Ok(None));
                match job {
                    Ok(Some(job)) => {
                        self.fire_job(&job, TriggerSource::Api, skip_deps).await;
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
                            .execute(&job, trigger, &self.callback_base_url)
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

    /// Cancels an execution, trying locally first then on the remote agent.
    async fn cancel_execution(&self, exec_id: Uuid) {
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
                    .cancel_execution(&agent.address, agent.port, exec_id)
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

        warn!("cancel: execution {} not found or not running", exec_id);
    }

    async fn handle_event(&mut self, event: Event) {
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
                self.fire_job(job, TriggerSource::Event { event_id: event.id }, false)
                    .await;
            }
        }
    }

    async fn reload_jobs(&mut self) {
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

    fn invalidate_cache(&mut self) {
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
