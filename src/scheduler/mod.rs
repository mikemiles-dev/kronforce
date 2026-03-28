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
    TriggerNow(Uuid),
    /// Cancel the execution with the given ID.
    CancelExecution(Uuid),
    /// Notify the scheduler of a new event for event-triggered jobs.
    EventOccurred(Event),
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

            match &job.schedule {
                ScheduleKind::Cron(expr) => {
                    self.check_cron_job(job, &expr.0, now).await;
                }
                ScheduleKind::OneShot(fire_at) => {
                    if now >= *fire_at {
                        self.fire_job(job, TriggerSource::Scheduler).await;
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
            self.fire_job(job, TriggerSource::Scheduler).await;

            let Ok(schedule) = CronSchedule::parse(cron_expr) else {
                return;
            };
            if let Some(next) = schedule.next_after(now) {
                self.next_fire_times.insert(job.id, next);
            }
        }
    }

    async fn fire_job(&self, job: &Job, trigger: TriggerSource) {
        if !job.depends_on.is_empty() {
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
            SchedulerCommand::TriggerNow(job_id) => {
                let db = self.db.clone();
                let job = tokio::task::spawn_blocking(move || db.get_job(job_id))
                    .await
                    .unwrap_or(Ok(None));
                match job {
                    Ok(Some(job)) => {
                        self.fire_job(&job, TriggerSource::Api).await;
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

        for job in &jobs {
            if job.status != JobStatus::Scheduled {
                continue;
            }

            if let ScheduleKind::Event(ref config) = job.schedule {
                if !event_matches(&event, config) {
                    continue;
                }

                // Avoid infinite loops: don't trigger from events caused by event-triggered jobs
                // (events from TriggerSource::Event executions)
                info!("event '{}' matched job '{}', firing", event.kind, job.name);
                self.fire_job(job, TriggerSource::Event { event_id: event.id })
                    .await;
            }
        }
    }

    async fn reload_jobs(&mut self) {
        let db = self.db.clone();
        match tokio::task::spawn_blocking(move || db.get_active_cron_jobs()).await {
            Ok(Ok(jobs)) => {
                debug!("loaded {} active jobs", jobs.len());
                self.jobs_cache = Some(jobs);
            }
            Ok(Err(e)) => {
                error!("failed to load jobs: {e}");
            }
            Err(e) => {
                error!("failed to load jobs (join): {e}");
            }
        }
    }

    fn invalidate_cache(&mut self) {
        self.jobs_cache = None;
        self.next_fire_times.clear();
    }
}

fn event_matches(event: &Event, config: &EventTriggerConfig) -> bool {
    // Match kind pattern
    if !pattern_matches(&config.kind_pattern, &event.kind) {
        return false;
    }

    // Match severity filter
    if let Some(ref sev) = config.severity
        && event.severity != *sev
    {
        return false;
    }

    // Match job name filter (prefix match on the event message or related job)
    if let Some(ref name_filter) = config.job_name_filter {
        // Check if the event message contains the job name
        if !event
            .message
            .to_lowercase()
            .contains(&name_filter.to_lowercase())
        {
            return false;
        }
    }

    true
}

fn pattern_matches(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix(".*") {
        return value.starts_with(prefix)
            && value.len() > prefix.len()
            && value.as_bytes()[prefix.len()] == b'.';
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return value.starts_with(prefix);
    }
    pattern == value
}
