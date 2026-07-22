use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskNode {
    pub id: String,
    pub instruction: String,
    pub priority: u32,
    pub estimated_time_ms: u64,
}

#[derive(Debug)]
pub struct ScheduledTask {
    pub node: TaskNode,
    pub queued_at: Instant,
}

pub struct DagScheduler {
    pub graph: DiGraph<TaskNode, ()>,
    pub node_map: HashMap<String, NodeIndex>,
}

impl DagScheduler {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    pub fn add_task(&mut self, task: TaskNode) -> NodeIndex {
        let id = task.id.clone();
        let idx = self.graph.add_node(task);
        self.node_map.insert(id, idx);
        idx
    }

    pub fn add_dependency(&mut self, from_id: &str, to_id: &str) -> anyhow::Result<()> {
        let from_idx = self.node_map.get(from_id).ok_or_else(|| anyhow::anyhow!("Node {} not found", from_id))?;
        let to_idx = self.node_map.get(to_id).ok_or_else(|| anyhow::anyhow!("Node {} not found", to_id))?;
        self.graph.add_edge(*from_idx, *to_idx, ());
        Ok(())
    }

    /// Calculates WSJF: (Priority + Aging) / Estimated Time
    /// WSJF (Weighted Shortest Job First) ensures short/important tasks run first,
    /// but aging prevents starvation of longer tasks.
    pub fn calculate_wsjf_score(task: &ScheduledTask) -> f64 {
        let priority = task.node.priority as f64;
        let aging = task.queued_at.elapsed().as_secs_f64(); // Add weight to old tasks
        let estimate = (task.node.estimated_time_ms as f64).max(1.0); // Avoid division by zero
        
        (priority + aging) / estimate
    }
}
