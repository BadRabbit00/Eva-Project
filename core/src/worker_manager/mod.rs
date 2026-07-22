use crate::ipc::shmem_manager::ShmemManager;
use anyhow::Context;
use std::collections::HashMap;
use tokio::process::{Child, Command};
use tracing::info;

pub struct SubAgent {
    pub process: Child,
    pub shmem_manager: ShmemManager,
}

pub struct WorkerManager {
    agents: HashMap<String, SubAgent>,
    base_shmem_size: usize,
    models_dir: String,
}

impl WorkerManager {
    pub fn new(base_shmem_size: usize, models_dir: String) -> Self {
        Self {
            agents: HashMap::new(),
            base_shmem_size,
            models_dir,
        }
    }

    pub fn spawn_worker(&mut self, agent_id: &str) -> anyhow::Result<()> {
        info!("Spawning isolated worker for agent: {}", agent_id);

        let shmem_manager = ShmemManager::new(self.base_shmem_size)
            .with_context(|| format!("Failed to create IPC memory for agent {}", agent_id))?;

        let os_id = shmem_manager.get_os_id().to_string();

        let process = Command::new("cargo")
            .arg("run")
            .arg("--bin")
            .arg("worker-candle")
            .arg("--")
            .arg("--shmem-id")
            .arg(&os_id)
            .arg("--models-dir")
            .arg(&self.models_dir)
            .spawn()
            .with_context(|| format!("Failed to spawn worker process for agent {}", agent_id))?;

        self.agents.insert(
            agent_id.to_string(),
            SubAgent {
                process,
                shmem_manager,
            },
        );

        Ok(())
    }

    pub fn get_worker_shmem(&self, agent_id: &str) -> Option<&ShmemManager> {
        self.agents.get(agent_id).map(|agent| &agent.shmem_manager)
    }

    pub fn shutdown_all(&mut self) {
        for (id, agent) in self.agents.iter_mut() {
            info!("Shutting down worker agent: {}", id);
            let _ = agent.process.start_kill();
        }
    }
}
