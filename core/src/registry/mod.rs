use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolDefinition {
    pub description: String,
    #[serde(default)]
    pub allowed_flags: HashMap<String, String>,
    #[serde(default)]
    pub allowed_args: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CatRegistry {
    #[serde(default)]
    pub tools: HashMap<String, ToolDefinition>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelDefinition {
    pub vram_mb: u32,
    pub context_window: u32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub supports_thinking: bool,
    #[serde(default)]
    pub green_flags: String,
    #[serde(default)]
    pub red_flags: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DefaultParams {
    pub temperature: f32,
    pub top_p: f32,
    pub repetition_penalty: f32,
    pub max_loop_iterations: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelRegistry {
    #[serde(default)]
    pub models: HashMap<String, ModelDefinition>,
    pub default_params: Option<DefaultParams>,
}

#[derive(Debug, Clone)]
pub struct RegistryManager {
    registry_dir: PathBuf,
    pub cat: CatRegistry,
    pub models: ModelRegistry,
}

impl RegistryManager {
    pub fn new<P: AsRef<Path>>(registry_dir: P) -> Result<Self> {
        let registry_dir = registry_dir.as_ref().to_path_buf();

        let cat_path = registry_dir.join("cat_registry.yaml");
        let cat = if cat_path.exists() {
            let content = std::fs::read_to_string(&cat_path)?;
            serde_yaml::from_str(&content)
                .with_context(|| format!("Failed to parse {:?}", cat_path))?
        } else {
            tracing::warn!("cat_registry.yaml not found, using empty registry");
            CatRegistry {
                tools: HashMap::new(),
            }
        };

        let model_path = registry_dir.join("model_registry.yaml");
        let models = if model_path.exists() {
            let content = std::fs::read_to_string(&model_path)?;
            serde_yaml::from_str(&content)
                .with_context(|| format!("Failed to parse {:?}", model_path))?
        } else {
            tracing::warn!("model_registry.yaml not found, using empty registry");
            ModelRegistry {
                models: HashMap::new(),
                default_params: None,
            }
        };

        Ok(Self {
            registry_dir,
            cat,
            models,
        })
    }

    pub fn reload(&mut self) -> Result<()> {
        let new_manager = Self::new(&self.registry_dir)?;
        self.cat = new_manager.cat;
        self.models = new_manager.models;
        tracing::info!("Registries reloaded from disk.");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_model_registry() {
        let yaml = r#"
models:
  test_model:
    vram_mb: 1024
    context_window: 2048
    supports_thinking: true
    tags: ["test", "fast"]
default_params:
  temperature: 0.7
  top_p: 0.9
  repetition_penalty: 1.0
  max_loop_iterations: 3
"#;
        let parsed: ModelRegistry =
            serde_yaml::from_str(yaml).expect("Failed to parse valid model registry YAML");
        assert!(parsed.models.contains_key("test_model"));
        let model = &parsed.models["test_model"];
        assert_eq!(model.vram_mb, 1024);
        assert_eq!(model.supports_thinking, true);
        assert_eq!(model.tags.len(), 2);
    }

    #[test]
    fn test_parse_cat_registry() {
        let yaml = r#"
tools:
  ls:
    description: "List directory contents"
    allowed_flags:
      "-l": "Long format"
"#;
        let parsed: CatRegistry =
            serde_yaml::from_str(yaml).expect("Failed to parse valid CAT registry YAML");
        assert!(parsed.tools.contains_key("ls"));
        let tool = &parsed.tools["ls"];
        assert_eq!(tool.allowed_flags.get("-l").unwrap(), "Long format");
    }
}
