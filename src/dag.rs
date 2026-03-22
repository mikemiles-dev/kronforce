use std::collections::HashMap;

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::is_cyclic_directed;
use uuid::Uuid;

use crate::db::Db;
use crate::error::AppError;
use crate::models::ExecutionStatus;

#[derive(Clone)]
pub struct DagResolver {
    db: Db,
}

impl DagResolver {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Check if all dependencies of a job have a recent successful execution.
    pub fn deps_satisfied(&self, depends_on: &[Uuid]) -> Result<bool, AppError> {
        for dep_id in depends_on {
            match self.db.get_latest_execution_for_job(*dep_id)? {
                Some(exec) if exec.status == ExecutionStatus::Succeeded => {}
                _ => return Ok(false),
            }
        }
        Ok(true)
    }

    /// Validate that adding/updating a job with the given dependencies won't create a cycle.
    pub fn validate_no_cycle(&self, job_id: Uuid, depends_on: &[Uuid]) -> Result<(), AppError> {
        let all_jobs = self.db.get_all_jobs_for_dag()?;

        let mut graph = DiGraph::<Uuid, ()>::new();
        let mut node_map: HashMap<Uuid, NodeIndex> = HashMap::new();

        // Add all existing jobs as nodes
        for (id, _) in &all_jobs {
            let idx = graph.add_node(*id);
            node_map.insert(*id, idx);
        }

        // Ensure the current job is in the graph
        if !node_map.contains_key(&job_id) {
            let idx = graph.add_node(job_id);
            node_map.insert(job_id, idx);
        }

        // Ensure all dependencies are in the graph
        for dep_id in depends_on {
            if !node_map.contains_key(dep_id) {
                return Err(AppError::BadRequest(format!(
                    "dependency job {} does not exist",
                    dep_id
                )));
            }
        }

        // Add edges for all existing jobs (except the one being updated)
        for (id, deps) in &all_jobs {
            if *id == job_id {
                continue; // skip, we'll add our new edges instead
            }
            for dep in deps {
                if let (Some(&from), Some(&to)) = (node_map.get(dep), node_map.get(id)) {
                    graph.add_edge(from, to, ());
                }
            }
        }

        // Add edges for the job being created/updated
        for dep_id in depends_on {
            let from = node_map[dep_id];
            let to = node_map[&job_id];
            graph.add_edge(from, to, ());
        }

        if is_cyclic_directed(&graph) {
            return Err(AppError::BadRequest(
                "dependency cycle detected".to_string(),
            ));
        }

        Ok(())
    }
}
