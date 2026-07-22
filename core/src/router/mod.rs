use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use anyhow::Context;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ActionType {
    #[serde(rename = "MCP_Call")]
    McpCall,
    #[serde(rename = "LLM_Inference")]
    LlmInference,
    #[serde(rename = "Sub_Agent")]
    SubAgent,
    #[serde(rename = "Condition_If_Else")]
    ConditionIfElse,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecutionNode {
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub action_type: ActionType,
    pub target_model: Option<String>,
    pub system_prompt: Option<String>,
    pub payload: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PipelineDefinition {
    pub schema_version: Option<String>,
    pub task_priority: u8,
    pub nodes: HashMap<String, ExecutionNode>,
}

pub struct Router {
    pub templates_dir: PathBuf,
}

impl Router {
    pub fn new(templates_dir: PathBuf) -> Self {
        Self { templates_dir }
    }

    /// Fast Track: Load static YAML template
    pub fn load_template(&self, template_id: &str) -> anyhow::Result<PipelineDefinition> {
        let path = self.templates_dir.join(format!("{}.yaml", template_id));
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read template: {:?}", path))?;
        
        let pipeline: PipelineDefinition = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML template: {}", template_id))?;
            
        Ok(pipeline)
    }

    /// Deep Track: Zero-Node logic to route to light LLM
    pub fn route_dynamic_task(&self, prompt: &str) -> anyhow::Result<()> {
        // Zero-Node logic will go here:
        // Spawns a light model to classify the prompt and either return an existing template_id
        // or trigger the Pipeline Architect (heavy model) to generate a JSON DAG.
        tracing::info!("Routing dynamic task via Zero-Node: {}", prompt);
        Ok(())
    }
}
