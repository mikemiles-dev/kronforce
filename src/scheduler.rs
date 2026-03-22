use std::collections::HashMap;

use chrono::{DateTime, Utc};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::config::Config;
use crate::cron_parser::CronSchedule;
use crate::dag::DagResolver;
use crate::db::Db;
use crate::executor::Executor;
use crate::models::*;

#[derive(Debug)]
pub enum SchedulerCommand {
    Reload,
    TriggerNow(Uuid),
    CancelExecution(Uuid),
}

pub struct Scheduler {
    db: Db,
    executor: Executor,
    dag: DagResolver,
    rx: mpsc::Receiver<SchedulerCommand>,
    tick_interval: std::time::Duration,
    next_fire_times: HashMap<Uuid, DateTime<Utc>>,
    jobs_cache: Option<Vec<Job>>,
}

impl Scheduler {
    pub fn new(
        db: Db,
        executor: Executor,
        dag: DagResolver,
        rx: mpsc::Receiver<SchedulerCommand>,
        config: &Config,
    ) -> Self {
        Self {
            db,
            executor,
            dag,
            rx,
            tick_interval: config.tick_interval,
            next_fire_times: HashMap::new(),
            jobs_cache: None,
        }
    }

    pub async fn run(mut self) {
        tracing::info!("scheduler started, tick interval: {:?}", self.tick_interval);
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

        // Load jobs if cache is empty
        if self.jobs_cache.is_none() {
            self.reload_jobs().await;
        }

        let jobs = match &self.jobs_cache {
            Some(jobs) => jobs.clone(),
            None => return,
        };

        for job in &jobs {
            if job.status != JobStatus::Active {
                continue;
            }

            match &job.schedule {
                ScheduleKind::Cron(expr) => {
                    self.check_cron_job(job, &expr.0, now).await;
                }
                ScheduleKind::OneShot(fire_at) => {
                    if now >= *fire_at {
                        self.fire_job(job, TriggerSource::Scheduler).await;
                        // Disable one-shot after firing
                        let mut updated = job.clone();
                        updated.status = JobStatus::Completed;
                        updated.updated_at = Utc::now();
                        let db = self.db.clone();
                        let _ = tokio::task::spawn_blocking(move || db.update_job(&updated)).await;
                        self.invalidate_cache();
                    }
                }
                ScheduleKind::Manual => {}
            }
        }
    }

    async fn check_cron_job(&mut self, job: &Job, cron_expr: &str, now: DateTime<Utc>) {
        let next_fire = self.next_fire_times.get(&job.id).copied();

        let fire_time = match next_fire {
            Some(t) => t,
            None => {
                // Compute initial next fire time
                let Ok(schedule) = CronSchedule::parse(cron_expr) else {
                    tracing::warn!("invalid cron for job {}: {}", job.name, cron_expr);
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

            // Compute next fire time
            let Ok(schedule) = CronSchedule::parse(cron_expr) else {
                return;
            };
            if let Some(next) = schedule.next_after(now) {
                self.next_fire_times.insert(job.id, next);
            }
        }
    }

    async fn fire_job(&self, job: &Job, trigger: TriggerSource) {
        // Check dependencies
        if !job.depends_on.is_empty() {
            let dag = self.dag.clone();
            let deps = job.depends_on.clone();
            let satisfied = tokio::task::spawn_blocking(move || dag.deps_satisfied(&deps))
                .await
                .unwrap_or(Ok(false));

            match satisfied {
                Ok(true) => {}
                Ok(false) => {
                    tracing::debug!(
                        "skipping job {} ({}): dependencies not satisfied",
                        job.name,
                        job.id
                    );
                    return;
                }
                Err(e) => {
                    tracing::error!("error checking deps for job {}: {e}", job.name);
                    return;
                }
            }
        }

        match self.executor.execute(job, trigger).await {
            Ok(exec_id) => {
                tracing::info!("fired job {} ({}) -> execution {}", job.name, job.id, exec_id);
            }
            Err(e) => {
                tracing::error!("failed to execute job {} ({}): {e}", job.name, job.id);
            }
        }
    }

    async fn handle_command(&mut self, cmd: SchedulerCommand) {
        match cmd {
            SchedulerCommand::Reload => {
                tracing::debug!("reloading jobs");
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
                        tracing::warn!("trigger: job {} not found", job_id);
                    }
                    Err(e) => {
                        tracing::error!("trigger: error loading job {}: {e}", job_id);
                    }
                }
            }
            SchedulerCommand::CancelExecution(exec_id) => {
                if self.executor.cancel(exec_id).await {
                    tracing::info!("cancelled execution {}", exec_id);
                } else {
                    tracing::warn!("cancel: execution {} not running", exec_id);
                }
            }
        }
    }

    async fn reload_jobs(&mut self) {
        let db = self.db.clone();
        match tokio::task::spawn_blocking(move || db.get_active_cron_jobs()).await {
            Ok(Ok(jobs)) => {
                tracing::debug!("loaded {} active jobs", jobs.len());
                self.jobs_cache = Some(jobs);
            }
            Ok(Err(e)) => {
                tracing::error!("failed to load jobs: {e}");
            }
            Err(e) => {
                tracing::error!("failed to load jobs (join): {e}");
            }
        }
    }

    fn invalidate_cache(&mut self) {
        self.jobs_cache = None;
        self.next_fire_times.clear();
    }
}
