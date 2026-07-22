use std::path::PathBuf;
use anyhow::Context;
use hnsw_rs::prelude::*;
// Using a generic HNSW stub for now since embeddings dimension is unknown
// Hnsw<f32, DistCosine> or DistL2 would be used in a real scenario.

pub struct ContextEngine {
    tools_dir: PathBuf,
}

impl ContextEngine {
    pub fn new(tools_dir: PathBuf) -> Self {
        Self { tools_dir }
    }

    /// Scans the MCP tools directory for markdown manuals.
    /// These files define the constraints and capabilities of external tools.
    pub fn scan_mcp_tools(&self) -> anyhow::Result<Vec<String>> {
        let mut tools = Vec::new();
        if !self.tools_dir.exists() {
            tracing::warn!("Tools directory does not exist: {:?}", self.tools_dir);
            return Ok(tools);
        }

        for entry in std::fs::read_dir(&self.tools_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                let content = std::fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read tool file: {:?}", path))?;
                tools.push(content);
            }
        }
        
        tracing::info!("Context Engine loaded {} tools from {:?}", tools.len(), self.tools_dir);
        Ok(tools)
    }

    /// Retrieve relevant context from the RAG Vector Index.
    /// Stubbed method - requires embedding generation integration.
    pub fn retrieve_context(&self, _query_embedding: &[f32], _top_k: usize) -> anyhow::Result<Vec<usize>> {
        // Here we would query the HNSW index
        // self.index.search(query_embedding, top_k, EfSearch::default());
        Ok(vec![])
    }
}
