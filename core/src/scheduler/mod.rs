use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
        let from_idx = self
            .node_map
            .get(from_id)
            .ok_or_else(|| anyhow::anyhow!("Node {} not found", from_id))?;
        let to_idx = self
            .node_map
            .get(to_id)
            .ok_or_else(|| anyhow::anyhow!("Node {} not found", to_id))?;
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

    /// The main scheduler event loop that checks for tasks and assigns them
    pub async fn run_loop(mut self, mut receiver: tokio::sync::mpsc::Receiver<TaskNode>) {
        tracing::info!("DagScheduler Event Loop started");
        loop {
            // Check for new tasks without blocking forever if we have tasks to run
            if self.graph.node_count() > 0 {
                if let Ok(task) = receiver.try_recv() {
                    tracing::info!("Scheduler received new task: {}", task.id);
                    self.add_task(task);
                }
            } else {
                // Block until a task arrives
                if let Some(task) = receiver.recv().await {
                    tracing::info!("Scheduler received new task: {}", task.id);
                    self.add_task(task);
                } else {
                    break; // Channel closed
                }
            }

            // In a real implementation we would scan for Ready nodes, calc WSJF,
            // and write to shmem. For now, simulate taking the task.
            // TODO: dispatch to worker memory map
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_wsjf_calculation() {
        let node = TaskNode {
            id: "test".into(),
            instruction: "test".into(),
            priority: 5,
            estimated_time_ms: 100,
        };

        let mut task = ScheduledTask {
            node,
            queued_at: Instant::now(),
        };

        let score1 = DagScheduler::calculate_wsjf_score(&task);
        assert!((score1 - 0.05).abs() < 0.001);

        task.queued_at = Instant::now() - Duration::from_secs(10);
        let score2 = DagScheduler::calculate_wsjf_score(&task);
        assert!((score2 - 0.15).abs() < 0.001);
    }

    #[test]
    fn test_dag_dependency() {
        let mut scheduler = DagScheduler::new();
        scheduler.add_task(TaskNode {
            id: "a".into(),
            instruction: "do a".into(),
            priority: 1,
            estimated_time_ms: 10,
        });
        scheduler.add_task(TaskNode {
            id: "b".into(),
            instruction: "do b".into(),
            priority: 1,
            estimated_time_ms: 10,
        });

        assert!(scheduler.add_dependency("a", "b").is_ok());
        assert!(scheduler.add_dependency("x", "y").is_err());

        assert_eq!(scheduler.graph.node_count(), 2);
        assert_eq!(scheduler.graph.edge_count(), 1);
    }
}
