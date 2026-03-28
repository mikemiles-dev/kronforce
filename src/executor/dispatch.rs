use super::*;

impl super::Executor {
    pub(crate) async fn dispatch_to_agent(
        &self,
        agent_id: Uuid,
        job: &Job,
        trigger: TriggerSource,
        callback_base_url: &str,
    ) -> Result<Uuid, AppError> {
        let db = self.db.clone();
        let agent = tokio::task::spawn_blocking(move || db.get_agent(agent_id))
            .await
            .unwrap()?
            .ok_or_else(|| AppError::AgentUnavailable(format!("agent {agent_id} not found")))?;

        if agent.status != AgentStatus::Online {
            return Err(AppError::AgentUnavailable(format!(
                "agent {} is {}",
                agent.name,
                agent.status.as_str()
            )));
        }

        self.dispatch_to_specific_agent(&agent, job, trigger, callback_base_url)
            .await
    }

    pub(crate) async fn dispatch_to_tagged(
        &self,
        tag: &str,
        job: &Job,
        trigger: TriggerSource,
        callback_base_url: &str,
    ) -> Result<Uuid, AppError> {
        let db = self.db.clone();
        let tag_owned = tag.to_string();
        let tag_for_err = tag_owned.clone();
        let required_type = Self::required_agent_type(&job.task);
        let agents: Vec<_> =
            tokio::task::spawn_blocking(move || db.get_online_agents_by_tag(&tag_owned))
                .await
                .unwrap()?
                .into_iter()
                .filter(|a| a.agent_type == required_type)
                .collect();

        if agents.is_empty() {
            return Err(AppError::AgentUnavailable(format!(
                "no online {} agents with tag '{}'",
                required_type.as_str(),
                tag_for_err
            )));
        }

        // Pick random agent
        let idx = (Utc::now().timestamp_nanos_opt().unwrap_or(0) as usize) % agents.len();
        let agent = &agents[idx];

        self.dispatch_to_specific_agent(agent, job, trigger, callback_base_url)
            .await
    }

    pub(crate) fn required_agent_type(task: &TaskType) -> AgentType {
        match task {
            TaskType::Custom { .. } => AgentType::Custom,
            _ => AgentType::Standard,
        }
    }

    pub(crate) async fn dispatch_to_any(
        &self,
        job: &Job,
        trigger: TriggerSource,
        callback_base_url: &str,
    ) -> Result<Uuid, AppError> {
        let db = self.db.clone();
        let required_type = Self::required_agent_type(&job.task);
        let agents =
            tokio::task::spawn_blocking(move || db.get_online_agents_by_type(required_type))
                .await
                .unwrap()?;

        if agents.is_empty() {
            return Err(AppError::AgentUnavailable(format!(
                "no online {} agents available",
                required_type.as_str()
            )));
        }

        let idx = (Utc::now().timestamp_nanos_opt().unwrap_or(0) as usize) % agents.len();
        let agent = &agents[idx];

        self.dispatch_to_specific_agent(agent, job, trigger, callback_base_url)
            .await
    }

    pub(crate) async fn dispatch_to_all(
        &self,
        job: &Job,
        trigger: TriggerSource,
        callback_base_url: &str,
    ) -> Result<Uuid, AppError> {
        let db = self.db.clone();
        let required_type = Self::required_agent_type(&job.task);
        let agents =
            tokio::task::spawn_blocking(move || db.get_online_agents_by_type(required_type))
                .await
                .unwrap()?;

        if agents.is_empty() {
            return Err(AppError::AgentUnavailable(format!(
                "no online {} agents available",
                required_type.as_str()
            )));
        }

        let mut first_exec_id = None;
        for agent in &agents {
            match self
                .dispatch_to_specific_agent(agent, job, trigger.clone(), callback_base_url)
                .await
            {
                Ok(exec_id) => {
                    if first_exec_id.is_none() {
                        first_exec_id = Some(exec_id);
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "failed to dispatch to agent {} ({}): {e}",
                        agent.name,
                        agent.id
                    );
                }
            }
        }

        first_exec_id
            .ok_or_else(|| AppError::AgentError("failed to dispatch to any agent".to_string()))
    }

    pub(crate) async fn dispatch_to_specific_agent(
        &self,
        agent: &Agent,
        job: &Job,
        trigger: TriggerSource,
        callback_base_url: &str,
    ) -> Result<Uuid, AppError> {
        let exec_id = Uuid::new_v4();
        let now = Utc::now();

        let rec = ExecutionRecord::new(exec_id, job.id, trigger)
            .with_agent_id(agent.id)
            .with_task_snapshot(job.task.clone())
            .with_started_at(now);

        let db = self.db.clone();
        let rec_clone = rec.clone();
        tokio::task::spawn_blocking(move || db.insert_execution(&rec_clone))
            .await
            .unwrap()?;

        let callback_url = format!("{}/api/callbacks/execution-result", callback_base_url);

        // Custom agents use pull-based queue
        if agent.agent_type == AgentType::Custom {
            let db = self.db.clone();
            let queue_id = Uuid::new_v4();
            let job_id = job.id;
            let task = job.task.clone();
            let run_as = job.run_as.clone();
            let timeout = job.timeout_secs;
            let agent_id = agent.id;
            let cb = callback_url.clone();
            tokio::task::spawn_blocking(move || {
                db.enqueue_job(
                    queue_id,
                    exec_id,
                    agent_id,
                    job_id,
                    &task,
                    run_as.as_deref(),
                    timeout,
                    &cb,
                )
            })
            .await
            .unwrap()?;
            tracing::info!(
                "queued job {} for custom agent {} -> execution {}",
                job.name,
                agent.name,
                exec_id
            );
            return Ok(exec_id);
        }

        // Standard agents use push
        let dispatch = JobDispatchRequest {
            execution_id: exec_id,
            job_id: job.id,
            task: job.task.clone(),
            run_as: job.run_as.clone(),
            timeout_secs: job.timeout_secs,
            callback_url,
        };

        match self
            .agent_client
            .dispatch_job(&agent.address, agent.port, &dispatch)
            .await
        {
            Ok(resp) if resp.accepted => {
                // Update to Running
                let db = self.db.clone();
                let mut running_rec = rec;
                running_rec.status = ExecutionStatus::Running;
                let _ =
                    tokio::task::spawn_blocking(move || db.update_execution(&running_rec)).await;
                tracing::info!(
                    "dispatched job {} to agent {} -> execution {}",
                    job.name,
                    agent.name,
                    exec_id
                );
                Ok(exec_id)
            }
            Ok(resp) => {
                let msg = resp.message.unwrap_or_else(|| "rejected".into());
                // Mark as failed
                let db = self.db.clone();
                let mut failed_rec = rec;
                failed_rec.status = ExecutionStatus::Failed;
                failed_rec.stderr = format!("agent rejected: {msg}");
                failed_rec.finished_at = Some(Utc::now());
                let _ = tokio::task::spawn_blocking(move || db.update_execution(&failed_rec)).await;
                Err(AppError::AgentError(msg))
            }
            Err(e) => {
                let db = self.db.clone();
                let mut failed_rec = rec;
                failed_rec.status = ExecutionStatus::Failed;
                failed_rec.stderr = format!("dispatch failed: {e}");
                failed_rec.finished_at = Some(Utc::now());
                let _ = tokio::task::spawn_blocking(move || db.update_execution(&failed_rec)).await;
                Err(e)
            }
        }
    }
}
