use std::collections::HashMap;

use chrono::Utc;
use petgraph::algo::is_cyclic_directed;
use petgraph::graph::{DiGraph, NodeIndex};
use uuid::Uuid;

use crate::db::Db;
use crate::db::models::{Dependency, ExecutionStatus};
use crate::error::AppError;

/// Resolves job dependency graphs and detects cycles.
#[derive(Clone)]
pub struct DagResolver {
    db: Db,
}

impl DagResolver {
    /// Creates a new resolver backed by the given database.
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Check if all dependencies have a successful execution within their time window.
    pub fn deps_satisfied(&self, depends_on: &[Dependency]) -> Result<bool, AppError> {
        let now = Utc::now();
        for dep in depends_on {
            match self.db.get_latest_execution_for_job(dep.job_id)? {
                Some(exec) if exec.status == ExecutionStatus::Succeeded => {
                    // Check time window if specified
                    if let Some(within_secs) = dep.within_secs {
                        if let Some(finished) = exec.finished_at {
                            let elapsed = (now - finished).num_seconds();
                            if elapsed < 0 || elapsed > within_secs as i64 {
                                return Ok(false);
                            }
                        } else {
                            return Ok(false);
                        }
                    }
                }
                _ => return Ok(false),
            }
        }
        Ok(true)
    }

    /// Validate that adding/updating a job with the given dependencies won't create a cycle.
    pub fn validate_no_cycle(
        &self,
        job_id: Uuid,
        depends_on: &[Dependency],
    ) -> Result<(), AppError> {
        let all_jobs = self.db.get_all_jobs_for_dag()?;

        let mut graph = DiGraph::<Uuid, ()>::new();
        let mut node_map: HashMap<Uuid, NodeIndex> = HashMap::new();

        for (id, _) in &all_jobs {
            let idx = graph.add_node(*id);
            node_map.insert(*id, idx);
        }

        node_map
            .entry(job_id)
            .or_insert_with(|| graph.add_node(job_id));

        for dep in depends_on {
            if !node_map.contains_key(&dep.job_id) {
                return Err(AppError::BadRequest(format!(
                    "dependency job {} does not exist",
                    dep.job_id
                )));
            }
        }

        for (id, deps) in &all_jobs {
            if *id == job_id {
                continue;
            }
            for dep_id in deps {
                if let (Some(&from), Some(&to)) = (node_map.get(dep_id), node_map.get(id)) {
                    graph.add_edge(from, to, ());
                }
            }
        }

        for dep in depends_on {
            let from = node_map[&dep.job_id];
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
