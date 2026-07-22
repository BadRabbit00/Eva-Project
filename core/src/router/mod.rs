use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ActionType {
    #[serde(rename = "MCP_Call")]
    McpCall,
    #[serde(rename = "LLM_Inference")]
    LlmInference,
    #[serde(rename = "Sub_Agent")]
    SubAgent,
    #[serde(rename = "Match")]
    Match,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecutionNode {
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub action_type: ActionType,
    pub target_model: Option<String>,
    pub system_prompt: Option<String>,

    // Standard payload
    pub payload: Option<String>,

    // Match routing
    pub target_node: Option<String>,
    #[serde(default)]
    pub cases: Vec<MatchCase>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MatchCase {
    pub r#match: String,
    pub activate: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_yaml_pipeline() {
        let yaml = r#"
schema_version: "1.0"
task_priority: 8
nodes:
  test_node:
    dependencies: []
    action_type: MCP_Call
    payload: "echo hello"
  branch_node:
    dependencies: ["test_node"]
    action_type: Match
    target_node: test_node
    cases:
      - match: "OOM"
        activate: "restart_node"
"#;

        let pipeline: PipelineDefinition = serde_yaml::from_str(yaml).expect("Failed to parse");
        assert_eq!(pipeline.task_priority, 8);
        assert!(pipeline.nodes.contains_key("test_node"));

        let node = &pipeline.nodes["test_node"];
        assert_eq!(node.action_type, ActionType::McpCall);
        assert_eq!(node.payload, Some("echo hello".to_string()));
        assert!(node.dependencies.is_empty());

        let branch = &pipeline.nodes["branch_node"];
        assert_eq!(branch.action_type, ActionType::Match);
        assert_eq!(branch.target_node, Some("test_node".to_string()));
        assert_eq!(branch.cases[0].r#match, "OOM");
        assert_eq!(branch.cases[0].activate, "restart_node");
    }
}
