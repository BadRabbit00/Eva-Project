use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct DaemonConfig {
    pub port: u16,
    pub shmem_size_mb: usize,
    pub models_dir: String,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        Self {
            port: 3000,
            shmem_size_mb: 16,
            models_dir: format!("{}/.eva/models", home),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DaemonConfig::default();
        assert_eq!(config.port, 3000);
        assert_eq!(config.shmem_size_mb, 16);
        assert!(config.models_dir.ends_with("/.eva/models"));
    }

    #[test]
    fn test_parse_config() {
        let toml_str = r#"
            port = 4000
            shmem_size_mb = 32
            models_dir = "/custom/path/models"
        "#;
        let config: DaemonConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.port, 4000);
        assert_eq!(config.shmem_size_mb, 32);
        assert_eq!(config.models_dir, "/custom/path/models");
    }
}
