//! Job scheduling loop and cron expression parsing.
//!
//! Fires jobs based on cron schedules, one-shot timers, and event triggers.
//! Accepts commands via an mpsc channel for reloads, manual triggers, and cancellations.

pub mod calendar;
mod commands;
pub mod cron_parser;
mod tick;

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use tokio::sync::mpsc;
use tracing::info;
use uuid::Uuid;

use crate::agent::AgentClient;
use crate::config::ControllerConfig;
use crate::dag::DagResolver;
use crate::db::Db;
use crate::db::models::*;
use crate::executor::Executor;

pub use calendar::{last_weekday_of_month, nth_weekday_of_month, parse_weekday};

/// Commands sent to the scheduler via its mpsc channel.
pub enum SchedulerCommand {
    /// Invalidate the job cache and reload from the database.
    Reload,
    /// Immediately fire the job with the given ID.
    TriggerNow {
        job_id: Uuid,
        skip_deps: bool,
        params: Option<serde_json::Value>,
    },
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
}
