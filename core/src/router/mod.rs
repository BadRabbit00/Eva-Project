use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Inference,
    CatExecutor,
    RagSearch,
    McpCall,
    Match,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PipelineNode {
    pub id: String,
    pub node_type: NodeType,

    // Inference specific
    pub model: Option<String>,
    #[serde(default)]
    pub thinking_mode: bool,
    pub prompt_template: Option<String>,

    // Flow control
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub next: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PipelineDefinition {
    pub id: String,
    pub description: String,
    pub nodes: Vec<PipelineNode>,
}

use crate::registry::RegistryManager;
use crate::scheduler::{DagScheduler, TaskNode};
use std::sync::Arc;

pub struct Router {
    pub pipelines_dir: PathBuf,
    pub registry: Arc<RegistryManager>,
}

impl Router {
    pub fn new(pipelines_dir: PathBuf, registry: Arc<RegistryManager>) -> Self {
        Self {
            pipelines_dir,
            registry,
        }
    }

    /// Loads a YAML pipeline definition from disk
    pub fn load_pipeline(&self, pipeline_id: &str) -> anyhow::Result<PipelineDefinition> {
        let path = self.pipelines_dir.join(format!("{}.yaml", pipeline_id));
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read pipeline: {:?}", path))?;

        let pipeline: PipelineDefinition = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML pipeline: {}", pipeline_id))?;

        Ok(pipeline)
    }

    /// Primary routing method for untagged incoming tasks
    pub fn route_dynamic_task(&self, input: &str) -> anyhow::Result<PipelineDefinition> {
        tracing::info!("Routing dynamic task via Default Ingress: {}", input);
        // Load the default pipeline which handles CAT -> RAG -> Generation
        self.load_pipeline("default_ingress")
    }

    /// Parses the PipelineDefinition into TaskNodes and submits them to the DagScheduler.
    pub fn submit_pipeline(
        &self,
        pipeline: PipelineDefinition,
        scheduler: &mut DagScheduler,
    ) -> anyhow::Result<()> {
        tracing::info!(
            "Submitting pipeline DAG: {} ({} nodes)",
            pipeline.id,
            pipeline.nodes.len()
        );

        for node in pipeline.nodes.iter() {
            // Placeholder metric calculation before benchmark module is implemented
            let estimated_time_ms = match node.node_type {
                NodeType::Inference => {
                    // Check if model exists and grab benchmark metrics (mocked for now)
                    if let Some(model_id) = &node.model {
                        if !self.registry.models.models.contains_key(model_id) {
                            tracing::warn!("Model '{}' not found in registry. Execution will suffer S_penalty.", model_id);
                        }
                    }
                    10000
                }
                NodeType::CatExecutor => 2000,
                NodeType::RagSearch => 1000,
                _ => 500,
            };

            let task_node = TaskNode {
                id: node.id.clone(),
                node_def: node.clone(),
                priority: 5, // default DAG priority
                estimated_time_ms,
            };
            scheduler.add_task(task_node);
        }

        // Establish topological edges (dependencies)
        for node in pipeline.nodes.iter() {
            for dep in &node.depends_on {
                scheduler.add_dependency(dep, &node.id)?;
            }
        }

        tracing::info!("Pipeline DAG '{}' successfully queued.", pipeline.id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_yaml_pipeline() {
        let yaml = r#"
id: "test_pipeline"
description: "A test pipeline"
nodes:
  - id: "node1"
    node_type: "inference"
    model: "phi-4"
    prompt_template: "hello"
    next: ["node2"]
  - id: "node2"
    node_type: "cat_executor"
    depends_on: ["node1"]
"#;

        let pipeline: PipelineDefinition = serde_yaml::from_str(yaml).expect("Failed to parse");
        assert_eq!(pipeline.id, "test_pipeline");
        assert_eq!(pipeline.nodes.len(), 2);

        let node1 = &pipeline.nodes[0];
        assert_eq!(node1.id, "node1");
        assert_eq!(node1.node_type, NodeType::Inference);
        assert_eq!(node1.model.as_deref(), Some("phi-4"));
    }
}
