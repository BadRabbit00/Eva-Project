use petgraph::graph::{DiGraph, NodeIndex};

use std::collections::HashMap;
use std::time::Instant;

use crate::router::PipelineNode;

#[derive(Debug, Clone)]
pub struct TaskNode {
    pub id: String,
    pub node_def: PipelineNode,
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
    pub fn calculate_wsjf_score(
        task: &ScheduledTask,
        hw_profile: &crate::state::HardwareProfile,
        location: &crate::worker_manager::ModelLocation,
        model_size_mb: u64,
    ) -> f64 {
        let priority = task.node.priority as f64;
        let aging = task.queued_at.elapsed().as_secs_f64(); // Add weight to old tasks

        let s_penalty = match location {
            crate::worker_manager::ModelLocation::VRAM => 0.0,
            crate::worker_manager::ModelLocation::RAM => {
                hw_profile.ram_to_vram_ms_per_mb * (model_size_mb as f64)
            }
            crate::worker_manager::ModelLocation::Disk => {
                (hw_profile.disk_to_ram_ms_per_mb + hw_profile.ram_to_vram_ms_per_mb)
                    * (model_size_mb as f64)
            }
        };

        let denominator = f64::max(0.001, (task.node.estimated_time_ms as f64) + s_penalty);

        (priority + aging) / denominator
    }

    /// The main scheduler event loop that checks for tasks and assigns them
    pub async fn run_loop(
        mut self,
        mut receiver: tokio::sync::mpsc::Receiver<TaskNode>,
        worker_mgr: std::sync::Arc<tokio::sync::RwLock<crate::worker_manager::WorkerManager>>,
        state_mgr: std::sync::Arc<tokio::sync::RwLock<crate::state::StateManager>>,
    ) {
        tracing::info!("DagScheduler Event Loop started");

        let mut queued_tasks: Vec<ScheduledTask> = Vec::new();

        loop {
            // Drain receiver for new tasks
            while let Ok(task) = receiver.try_recv() {
                tracing::info!("Scheduler received new task: {}", task.id);
                self.add_task(task.clone());
                queued_tasks.push(ScheduledTask {
                    node: task,
                    queued_at: Instant::now(),
                });
            }

            if queued_tasks.is_empty() {
                // Block if empty
                if let Some(task) = receiver.recv().await {
                    tracing::info!("Scheduler received new task: {}", task.id);
                    self.add_task(task.clone());
                    queued_tasks.push(ScheduledTask {
                        node: task,
                        queued_at: Instant::now(),
                    });
                } else {
                    break;
                }
            }

            // Calculate WSJF for all queued tasks
            let hw_profile = {
                let sm = state_mgr.read().await;
                sm.load_hardware_profile().unwrap_or_default()
            };

            let wm = worker_mgr.read().await;

            let mut best_index = 0;
            let mut best_score = -1.0;

            for (i, task) in queued_tasks.iter().enumerate() {
                let model_id = task
                    .node
                    .node_def
                    .model
                    .clone()
                    .unwrap_or_else(|| "default".into());
                let location = wm.get_model_location(&model_id);

                // Assuming model size is mocked or fetched from registry later
                let model_size_mb = 2000;

                let score = Self::calculate_wsjf_score(task, &hw_profile, &location, model_size_mb);

                if score > best_score {
                    best_score = score;
                    best_index = i;
                }
            }

            if !queued_tasks.is_empty() {
                let selected_task = queued_tasks.remove(best_index);
                tracing::info!(
                    "WSJF selected task {} with score {:.4} (aging: {}s). Dispatching...",
                    selected_task.node.id,
                    best_score,
                    selected_task.queued_at.elapsed().as_secs()
                );

                let model_id = selected_task
                    .node
                    .node_def
                    .model
                    .clone()
                    .unwrap_or_else(|| "default".into());
                let mut wm_write = worker_mgr.write().await;

                // Get HardwareProfile for max vram
                let sm_read = state_mgr.read().await;
                let hp = sm_read.load_hardware_profile().unwrap_or_default();
                drop(sm_read);

                if let Err(e) = wm_write.allocate_vram(2000, hp.max_vram_mb as u32) {
                    tracing::error!("Failed to allocate VRAM: {}", e);
                    // In a real system, we would put the task back or wait.
                }

                if let Some(agent) = wm_write.get_agent_mut("zero-node") {
                    agent.loaded_model_id = Some(model_id.clone());
                    agent.location = crate::worker_manager::ModelLocation::VRAM;

                    let shmem = &agent.shmem_manager;
                    let prompt = selected_task
                        .node
                        .node_def
                        .prompt_template
                        .clone()
                        .unwrap_or_default();

                    // Write prompt to input buffer
                    let prompt_bytes = prompt.as_bytes();
                    let prompt_len = std::cmp::min(prompt_bytes.len(), 4096 * 1024); // max 4MB for now
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            prompt_bytes.as_ptr(),
                            shmem.input_buffer_ptr(),
                            prompt_len,
                        );
                    }

                    // Write ControlBlock
                    let cb = shmem.control_block_mut();
                    cb.context_length = prompt_len as u32;
                    cb.max_tokens = 2048; // default

                    // Safety: We assume model_id fits in 256 bytes
                    let model_bytes = model_id.as_bytes();
                    let len = std::cmp::min(model_bytes.len(), 255);
                    cb.model_id.fill(0);
                    cb.model_id[..len].copy_from_slice(&model_bytes[..len]);

                    // Command Worker
                    shmem.write_status(shared_ipc::protocol::WorkerStatus::LoadWeights);

                    tracing::info!(
                        "Dispatched task {} to zero-node worker. Status set to LoadWeights.",
                        selected_task.node.id
                    );
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use crate::router::{NodeType, PipelineNode};

    #[test]
    fn test_wsjf_calculation() {
        let node = TaskNode {
            id: "test".into(),
            node_def: PipelineNode {
                id: "test".into(),
                node_type: NodeType::Inference,
                model: None,
                thinking_mode: false,
                prompt_template: None,
                depends_on: vec![],
                next: vec![],
            },
            priority: 5,
            estimated_time_ms: 100,
        };

        let mut task = ScheduledTask {
            node,
            queued_at: Instant::now(),
        };

        let hw_profile = crate::state::HardwareProfile::default();
        let loc_vram = crate::worker_manager::ModelLocation::VRAM;
        let score1 = DagScheduler::calculate_wsjf_score(&task, &hw_profile, &loc_vram, 1000);
        assert!((score1 - 0.05).abs() < 0.001);

        task.queued_at = Instant::now() - Duration::from_secs(10);
        let score2 = DagScheduler::calculate_wsjf_score(&task, &hw_profile, &loc_vram, 1000);
        assert!((score2 - 0.15).abs() < 0.001);

        // Test with RAM location (penalty applies)
        let loc_ram = crate::worker_manager::ModelLocation::RAM;
        let score3 = DagScheduler::calculate_wsjf_score(&task, &hw_profile, &loc_ram, 1000);
        // Penalty = 1000 * 0.1 = 100. Denominator = 100 + 100 = 200. Score = 15 / 200 = 0.075
        assert!((score3 - 0.075).abs() < 0.001);
    }

    #[test]
    fn test_dag_dependency() {
        let mut scheduler = DagScheduler::new();
        scheduler.add_task(TaskNode {
            id: "a".into(),
            node_def: PipelineNode {
                id: "a".into(),
                node_type: NodeType::Inference,
                model: None,
                thinking_mode: false,
                prompt_template: None,
                depends_on: vec![],
                next: vec![],
            },
            priority: 1,
            estimated_time_ms: 10,
        });
        scheduler.add_task(TaskNode {
            id: "b".into(),
            node_def: PipelineNode {
                id: "b".into(),
                node_type: NodeType::Inference,
                model: None,
                thinking_mode: false,
                prompt_template: None,
                depends_on: vec![],
                next: vec![],
            },
            priority: 1,
            estimated_time_ms: 10,
        });

        assert!(scheduler.add_dependency("a", "b").is_ok());
        assert!(scheduler.add_dependency("x", "y").is_err());

        assert_eq!(scheduler.graph.node_count(), 2);
        assert_eq!(scheduler.graph.edge_count(), 1);
    }
}
