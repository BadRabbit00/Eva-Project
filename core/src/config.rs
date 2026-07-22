use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct DaemonConfig {
    pub port: u16,
    pub shmem_size_mb: usize,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            shmem_size_mb: 16,
        }
    }
}

pub fn load_config() -> DaemonConfig {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let config_path = PathBuf::from(home).join(".config/eva/daemon.toml");

    if config_path.exists() {
        if let Ok(contents) = fs::read_to_string(&config_path) {
            if let Ok(config) = toml::from_str(&contents) {
                return config;
            }
        }
    }
    
    tracing::warn!("Could not load daemon.toml, using defaults");
    DaemonConfig::default()
}
