//! Tick-loop methods: periodic job checking for cron, calendar, interval, and one-shot schedules.

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use tracing::{debug, error, warn};

use super::calendar::{last_weekday_of_month, nth_weekday_of_month, parse_weekday};
use crate::db::models::*;
use crate::scheduler::cron_parser::CronSchedule;

impl super::Scheduler {
    pub(crate) async fn tick(&mut self) {
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
            if let Some(starts_at) = job.starts_at
                && now < starts_at
            {
                continue;
            }
            if let Some(expires_at) = job.expires_at
                && now > expires_at
            {
                debug!(
                    "job {} ({}) has expired, marking as unscheduled",
                    job.name, job.id
                );
                let mut updated = job.clone();
                updated.status = JobStatus::Unscheduled;
                updated.updated_at = Utc::now();
                let db = self.db.clone();
                if let Err(e) = tokio::task::spawn_blocking(move || db.update_job(&updated)).await {
                    error!("failed to update job status: {e}");
                }
                self.invalidate_cache();
                continue;
            }

            match &job.schedule {
                ScheduleKind::Cron(expr) => {
                    self.check_cron_job(job, &expr.0, now).await;
                }
                ScheduleKind::OneShot(fire_at) => {
                    if now >= *fire_at {
                        self.fire_job(job, TriggerSource::Scheduler, false, None)
                            .await;
                        let mut updated = job.clone();
                        updated.status = JobStatus::Unscheduled;
                        updated.updated_at = Utc::now();
                        let db = self.db.clone();
                        if let Err(e) =
                            tokio::task::spawn_blocking(move || db.update_job(&updated)).await
                        {
                            error!("failed to update job status: {e}");
                        }
                        self.invalidate_cache();
                    }
                }
                ScheduleKind::Calendar(cal) => {
                    self.check_calendar_job(job, cal, now).await;
                }
                ScheduleKind::Interval { interval_secs } => {
                    self.check_interval_job(job, *interval_secs, now).await;
                }
                ScheduleKind::OnDemand | ScheduleKind::Event(_) => {}
            }
        }

        // Evaluate pipeline schedules (group-level schedules that trigger root jobs)
        self.check_pipeline_schedules(now).await;
    }

    async fn check_pipeline_schedules(&mut self, now: DateTime<Utc>) {
        let db = self.db.clone();
        let schedules: Vec<(String, ScheduleKind)> = tokio::task::spawn_blocking(move || {
            let settings = db.get_all_settings()?;
            let mut result = Vec::new();
            for (key, value) in settings {
                if let Some(group) = key.strip_prefix("pipeline_schedule_")
                    && let Ok(sched) = serde_json::from_str::<ScheduleKind>(&value)
                {
                    result.push((group.to_string(), sched));
                }
            }
            Ok::<_, crate::error::AppError>(result)
        })
        .await
        .unwrap_or(Ok(vec![]))
        .unwrap_or_default();

        for (group, sched) in schedules {
            let should_fire = match &sched {
                ScheduleKind::Cron(expr) => {
                    let next = self.pipeline_next_fire.get(&group).copied();
                    let fire_time = match next {
                        Some(t) => t,
                        None => {
                            let Ok(schedule) = CronSchedule::parse(&expr.0) else {
                                continue;
                            };
                            let Some(t) = schedule.next_after(now - chrono::Duration::seconds(1))
                            else {
                                continue;
                            };
                            self.pipeline_next_fire.insert(group.clone(), t);
                            t
                        }
                    };
                    if now >= fire_time {
                        let already_fired = self
                            .pipeline_last_fired
                            .get(&group)
                            .is_some_and(|l| *l >= fire_time);

                        if let Ok(schedule) = CronSchedule::parse(&expr.0)
                            && let Some(next) = schedule.next_after(now)
                        {
                            self.pipeline_next_fire.insert(group.clone(), next);
                        }

                        !already_fired
                    } else {
                        false
                    }
                }
                ScheduleKind::Interval { interval_secs } => {
                    if let Some(last) = self.pipeline_last_fired.get(&group) {
                        (now - *last).num_seconds() >= *interval_secs as i64
                    } else {
                        true
                    }
                }
                _ => false,
            };

            if should_fire {
                self.pipeline_last_fired.insert(group.clone(), now);
                debug!("pipeline schedule firing for group '{}'", group);
                self.trigger_pipeline_roots(&group).await;
            }
        }
    }

    async fn trigger_pipeline_roots(&self, group: &str) {
        let db = self.db.clone();
        let group_owned = group.to_string();
        let jobs: Vec<Job> = tokio::task::spawn_blocking(move || {
            db.list_jobs(None, None, Some(&group_owned), 1000, 0)
        })
        .await
        .unwrap_or(Ok(vec![]))
        .unwrap_or_default();

        let group_ids: std::collections::HashSet<uuid::Uuid> = jobs.iter().map(|j| j.id).collect();
        let roots: Vec<&Job> = jobs
            .iter()
            .filter(|j| !j.depends_on.iter().any(|d| group_ids.contains(&d.job_id)))
            .collect();

        for job in roots {
            tracing::info!(
                "pipeline schedule: triggering root job '{}' in group '{}'",
                job.name,
                group
            );
            self.fire_job(job, TriggerSource::Scheduler, false, None)
                .await;
        }
    }

    async fn check_cron_job(&mut self, job: &Job, cron_expr: &str, now: DateTime<Utc>) {
        let next_fire = self.next_fire_times.get(&job.id).copied();

        let fire_time = match next_fire {
            Some(t) => t,
            None => {
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
            self.fire_job(job, TriggerSource::Scheduler, false, None)
                .await;

            let Ok(schedule) = CronSchedule::parse(cron_expr) else {
                return;
            };
            if let Some(next) = schedule.next_after(now) {
                self.next_fire_times.insert(job.id, next);
            }
        }
    }

    async fn check_calendar_job(&mut self, job: &Job, cal: &CalendarSchedule, now: DateTime<Utc>) {
        // Only fire once per day — check if we already fired today
        let today = now.date_naive();
        if let Some(last) = self.last_fired.get(&job.id)
            && last.date_naive() == today
        {
            return;
        }

        // Check month filter
        if !cal.months.is_empty() && !cal.months.contains(&(now.month())) {
            return;
        }

        // Check if now is past the fire time for today
        let fire_time_today = today
            .and_hms_opt(cal.hour, cal.minute, 0)
            .map(|dt| dt.and_utc());
        let Some(fire_at) = fire_time_today else {
            return;
        };
        if now < fire_at {
            return;
        }

        // Compute the anchor date for this month
        let year = now.year();
        let month = now.month();

        let anchor_date: Option<NaiveDate> = if cal.anchor == "last_day" {
            // Last day of current month
            if month == 12 {
                NaiveDate::from_ymd_opt(year + 1, 1, 1)
            } else {
                NaiveDate::from_ymd_opt(year, month + 1, 1)
            }
            .map(|d| d.pred_opt().unwrap_or(d))
        } else if cal.anchor.starts_with("day_") {
            // Specific day: day_1, day_15, etc.
            let day: u32 = cal.anchor[4..].parse().unwrap_or(1);
            NaiveDate::from_ymd_opt(year, month, day)
        } else if cal.anchor == "nth_weekday" {
            // Nth weekday of the month: e.g., 2nd Tuesday
            let nth = cal.nth.unwrap_or(1);
            let wd = parse_weekday(cal.weekday.as_deref().unwrap_or("monday"));
            nth_weekday_of_month(year, month, wd, nth)
        } else if cal.anchor.starts_with("first_") {
            let wd = parse_weekday(&cal.anchor[6..]);
            nth_weekday_of_month(year, month, wd, 1)
        } else if cal.anchor.starts_with("last_") && cal.anchor != "last_day" {
            let wd = parse_weekday(&cal.anchor[5..]);
            last_weekday_of_month(year, month, wd)
        } else {
            None
        };

        let Some(anchor) = anchor_date else {
            return;
        };

        // Apply offset
        let target = anchor + chrono::Duration::days(cal.offset_days as i64);

        // Skip weekends
        if cal.skip_weekends {
            let wd = target.weekday();
            if wd == chrono::Weekday::Sat || wd == chrono::Weekday::Sun {
                return;
            }
        }

        // Skip holidays
        if !cal.holidays.is_empty() {
            let target_str = target.format("%Y-%m-%d").to_string();
            if cal.holidays.contains(&target_str) {
                return;
            }
        }

        // Check if today matches the target
        if today == target {
            self.last_fired.insert(job.id, now);
            self.fire_job(job, TriggerSource::Scheduler, false, None)
                .await;
        }
    }

    async fn check_interval_job(&mut self, job: &Job, interval_secs: u64, now: DateTime<Utc>) {
        // Fire if enough time has passed since the last execution finished
        let db = self.db.clone();
        let job_id = job.id;
        let join =
            tokio::task::spawn_blocking(move || db.get_latest_execution_for_job(job_id)).await;
        let last_exec = match join {
            Ok(res) => res,
            Err(e) => {
                error!("interval check: spawn_blocking join failed for job {job_id}: {e}");
                return;
            }
        };

        let should_fire = match last_exec {
            Ok(Some(exec)) => {
                // Only fire after the previous execution is done
                if exec.status == ExecutionStatus::Running
                    || exec.status == ExecutionStatus::Pending
                {
                    return;
                }
                match exec.finished_at {
                    Some(finished) => {
                        let elapsed = (now - finished).num_seconds();
                        elapsed >= interval_secs as i64
                    }
                    None => {
                        // Status is terminal but finished_at was never written — likely a
                        // crash mid-finalize or schema drift. Refusing to fire prevents an
                        // infinite refire loop; surface it loudly so an operator can reconcile.
                        warn!(
                            job_id = %job_id,
                            status = ?exec.status,
                            "interval check: previous execution has terminal status but no finished_at; skipping refire"
                        );
                        false
                    }
                }
            }
            Ok(None) => true, // Never run before, fire immediately
            Err(e) => {
                error!("interval check: db query failed for job {job_id}: {e}");
                false
            }
        };

        if should_fire {
            self.fire_job(job, TriggerSource::Scheduler, false, None)
                .await;
        }
    }
}
