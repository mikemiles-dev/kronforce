use chrono::{DateTime, Utc};
use rusqlite::params;
use uuid::Uuid;

use super::Db;
use super::helpers::*;
use crate::error::AppError;
use crate::db::models::*;

impl Db {
    /// Inserts a new agent or updates an existing one matched by name.
    pub fn upsert_agent(&self, agent: &Agent) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        let tags_json = serde_json::to_string(&agent.tags).unwrap();
        let task_types_json = if agent.task_types.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&agent.task_types).unwrap())
        };
        conn.execute(
            "INSERT INTO agents (id, name, tags_json, hostname, address, port, agent_type, status, last_heartbeat, registered_at, task_types_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(name) DO UPDATE SET
               tags_json=excluded.tags_json, hostname=excluded.hostname, address=excluded.address,
               port=excluded.port, agent_type=excluded.agent_type, status=excluded.status,
               last_heartbeat=excluded.last_heartbeat, registered_at=excluded.registered_at,
               task_types_json=excluded.task_types_json",
            params![
                agent.id.to_string(),
                agent.name,
                tags_json,
                agent.hostname,
                agent.address,
                agent.port as i64,
                agent.agent_type.as_str(),
                agent.status.as_str(),
                agent.last_heartbeat.map(|t| t.to_rfc3339()),
                agent.registered_at.to_rfc3339(),
                task_types_json,
            ],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    /// Looks up an agent by its UUID.
    pub fn get_agent(&self, id: Uuid) -> Result<Option<Agent>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name, tags_json, hostname, address, port, agent_type, status, last_heartbeat, registered_at, task_types_json FROM agents WHERE id = ?1")
            .map_err(AppError::Db)?;
        let mut rows = stmt
            .query_map(params![id.to_string()], row_to_agent)
            .map_err(AppError::Db)?;
        match rows.next() {
            Some(Ok(a)) => Ok(Some(a)),
            Some(Err(e)) => Err(AppError::Db(e)),
            None => Ok(None),
        }
    }

    /// Looks up an agent by its unique name.
    pub fn get_agent_by_name(&self, name: &str) -> Result<Option<Agent>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name, tags_json, hostname, address, port, agent_type, status, last_heartbeat, registered_at, task_types_json FROM agents WHERE name = ?1")
            .map_err(AppError::Db)?;
        let mut rows = stmt
            .query_map(params![name], row_to_agent)
            .map_err(AppError::Db)?;
        match rows.next() {
            Some(Ok(a)) => Ok(Some(a)),
            Some(Err(e)) => Err(AppError::Db(e)),
            None => Ok(None),
        }
    }

    /// Returns all registered agents ordered by name.
    pub fn list_agents(&self) -> Result<Vec<Agent>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name, tags_json, hostname, address, port, agent_type, status, last_heartbeat, registered_at, task_types_json FROM agents ORDER BY name")
            .map_err(AppError::Db)?;
        let rows = stmt.query_map([], row_to_agent).map_err(AppError::Db)?;
        let mut agents = Vec::new();
        for row in rows {
            agents.push(row.map_err(AppError::Db)?);
        }
        Ok(agents)
    }

    /// Returns all agents with online status.
    pub fn get_online_agents(&self) -> Result<Vec<Agent>, AppError> {
        let agents = self.list_agents()?;
        Ok(agents
            .into_iter()
            .filter(|a| a.status == AgentStatus::Online)
            .collect())
    }

    /// Returns online agents filtered by agent type (standard or custom).
    pub fn get_online_agents_by_type(&self, agent_type: AgentType) -> Result<Vec<Agent>, AppError> {
        let agents = self.list_agents()?;
        Ok(agents
            .into_iter()
            .filter(|a| a.status == AgentStatus::Online && a.agent_type == agent_type)
            .collect())
    }

    /// Returns online agents that have the specified tag.
    pub fn get_online_agents_by_tag(&self, tag: &str) -> Result<Vec<Agent>, AppError> {
        let agents = self.list_agents()?;
        Ok(agents
            .into_iter()
            .filter(|a| a.status == AgentStatus::Online && a.tags.contains(&tag.to_string()))
            .collect())
    }

    /// Updates the agent's last heartbeat timestamp and sets its status to online.
    pub fn update_agent_heartbeat(&self, id: Uuid, at: DateTime<Utc>) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE agents SET last_heartbeat = ?1, status = 'online' WHERE id = ?2",
            params![at.to_rfc3339(), id.to_string()],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    /// Marks online agents as offline if their last heartbeat is older than the given timeout.
    pub fn expire_agents(&self, timeout: std::time::Duration) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        let cutoff =
            (Utc::now() - chrono::Duration::seconds(timeout.as_secs() as i64)).to_rfc3339();
        conn.execute(
            "UPDATE agents SET status = 'offline' WHERE status = 'online' AND last_heartbeat < ?1",
            params![cutoff],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    /// Deletes an agent by its UUID.
    pub fn delete_agent(&self, id: Uuid) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM agents WHERE id = ?1", params![id.to_string()])
            .map_err(AppError::Db)?;
        Ok(())
    }

    /// Replaces the task type definitions advertised by an agent.
    pub fn update_agent_task_types(
        &self,
        id: Uuid,
        task_types: &[TaskTypeDefinition],
    ) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        let json = if task_types.is_empty() {
            None
        } else {
            Some(serde_json::to_string(task_types).unwrap())
        };
        conn.execute(
            "UPDATE agents SET task_types_json = ?1 WHERE id = ?2",
            params![json, id.to_string()],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }
}
