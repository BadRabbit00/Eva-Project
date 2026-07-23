use crate::ipc::shmem_manager::ShmemManager;
use anyhow::Context;
use std::collections::HashMap;
use tokio::process::{Child, Command};
use tracing::info;

#[derive(Debug, Clone, PartialEq)]
pub enum ModelLocation {
    Disk,
    RAM,
    VRAM,
}

pub struct SubAgent {
    pub process: Child,
    pub shmem_manager: ShmemManager,
    pub loaded_model_id: Option<String>,
    pub location: ModelLocation,
}

pub struct WorkerManager {
    pub agents: HashMap<String, SubAgent>,
    shmem_size: usize,
    models_dir: String,
    pub used_vram_mb: u32,
}

impl WorkerManager {
    pub fn new(shmem_size: usize, models_dir: String) -> Self {
        Self {
            agents: HashMap::new(),
            shmem_size,
            models_dir,
            used_vram_mb: 0,
        }
    }

    pub fn spawn_worker(&mut self, agent_id: &str) -> anyhow::Result<()> {
        info!("Spawning isolated worker for agent: {}", agent_id);

        let shmem_manager = ShmemManager::new(self.shmem_size)
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
                loaded_model_id: None,
                location: ModelLocation::Disk,
            },
        );

        Ok(())
    }

    pub fn get_worker_shmem(&self, agent_id: &str) -> Option<&ShmemManager> {
        self.agents.get(agent_id).map(|agent| &agent.shmem_manager)
    }

    pub fn get_agent_mut(&mut self, agent_id: &str) -> Option<&mut SubAgent> {
        self.agents.get_mut(agent_id)
    }

    pub fn get_model_location(&self, target_model_id: &str) -> ModelLocation {
        for agent in self.agents.values() {
            if let Some(id) = &agent.loaded_model_id {
                if id == target_model_id {
                    return agent.location.clone();
                }
            }
        }
        ModelLocation::Disk
    }

    pub fn allocate_vram(&mut self, required_mb: u32, max_vram_mb: u32) -> anyhow::Result<()> {
        tracing::info!(
            "VRAM Allocation requested: {} MB. Currently used: {} MB / {} MB",
            required_mb,
            self.used_vram_mb,
            max_vram_mb
        );

        if self.used_vram_mb + required_mb > max_vram_mb {
            tracing::warn!("VRAM full! Attempting to evict oldest model...");

            // Find an agent to evict (currently naive: pick first that is in VRAM)
            let mut evicted = false;
            for agent in self.agents.values_mut() {
                if agent.location == ModelLocation::VRAM {
                    tracing::info!(
                        "Evicting model {} from VRAM to RAM...",
                        agent.loaded_model_id.as_deref().unwrap_or("unknown")
                    );

                    // Signal worker to unload (Interrupt/Idle signal is one way, or a specific Unload command)
                    // For now we simulate eviction in state tracking
                    agent.location = ModelLocation::RAM;
                    agent.loaded_model_id = None;

                    // Assume we freed up its size (hardcoded to 2000 for this PoC)
                    let freed_mb = 2000;
                    self.used_vram_mb = self.used_vram_mb.saturating_sub(freed_mb);
                    evicted = true;
                    break;
                }
            }

            if !evicted {
                anyhow::bail!("Could not free enough VRAM!");
            }
        }

        self.used_vram_mb += required_mb;
        Ok(())
    }

    pub fn shutdown_all(&mut self) {
        for (id, agent) in self.agents.iter_mut() {
            info!("Shutting down worker agent: {}", id);
            let _ = agent.process.start_kill();
        }
    }
}
