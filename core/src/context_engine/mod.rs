use anyhow::Context;
use hnsw_rs::prelude::*;
use std::path::PathBuf;
// Using a generic HNSW stub for now since embeddings dimension is unknown
// Hnsw<f32, DistCosine> or DistL2 would be used in a real scenario.

pub struct ContextEngine {
    tools_dir: PathBuf,
    index: Hnsw<'static, f32, DistL2>,
    documents: std::collections::HashMap<usize, String>,
}

impl ContextEngine {
    pub fn new(tools_dir: PathBuf) -> Self {
        let max_nb_connection = 16;
        let max_elements = 10000;
        let max_layer = 16;
        let ef_construction = 200;

        let index = Hnsw::<'static, f32, DistL2>::new(
            max_nb_connection,
            max_elements,
            max_layer,
            ef_construction,
            DistL2 {},
        );

        Self {
            tools_dir,
            index,
            documents: std::collections::HashMap::new(),
        }
    }

    /// Scans the MCP tools directory for markdown manuals.
    /// These files define the constraints and capabilities of external tools.
    pub fn scan_mcp_tools(&mut self) -> anyhow::Result<Vec<String>> {
        let mut tools = Vec::new();
        if !self.tools_dir.exists() {
            tracing::warn!("Tools directory does not exist: {:?}", self.tools_dir);
            return Ok(tools);
        }

        let mut doc_id = 0;
        for entry in std::fs::read_dir(&self.tools_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                let content = std::fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read tool file: {:?}", path))?;
                tools.push(content.clone());

                // Stub embedding (in a real system, we would embed the content)
                let dummy_embedding = vec![0.0f32; 384]; // e.g. all-MiniLM-L6-v2 dimension
                self.index.insert((&dummy_embedding, doc_id));
                self.documents.insert(doc_id, content);
                doc_id += 1;
            }
        }

        tracing::info!(
            "Context Engine loaded {} tools from {:?}",
            tools.len(),
            self.tools_dir
        );
        Ok(tools)
    }

    /// Retrieve relevant context from the RAG Vector Index.
    /// Stubbed method - requires embedding generation integration.
    pub fn retrieve_context(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> anyhow::Result<Vec<String>> {
        let mut results = Vec::new();
        // search returns Vec<Neighbor<f32>> usually
        let neighbors = self.index.search(query_embedding, top_k, 50);

        for neighbor in neighbors {
            if let Some(doc) = self.documents.get(&neighbor.d_id) {
                results.push(doc.clone());
            }
        }

        Ok(results)
    }
}
