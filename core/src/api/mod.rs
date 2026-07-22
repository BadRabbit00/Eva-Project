use axum::{
    routing::{get, post},
    Router, Json, extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;
use crate::scheduler::TaskNode;
use crate::context_engine::ContextEngine;

#[derive(Clone)]
pub struct AppState {
    pub task_sender: mpsc::Sender<TaskNode>,
    pub context_engine: Arc<RwLock<ContextEngine>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TaskSubmitRequest {
    pub prompt: String,
    pub priority: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TaskSubmitResponse {
    pub job_id: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct McpRegisterRequest {
    pub name: String,
    pub cmd: String,
    pub args: Vec<String>,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/task/submit", post(submit_task))
        .route("/api/v1/task/stream/:job_id", get(stream_task))
        .route("/api/v1/hypervisor/queue", get(queue_status))
        .route("/api/v1/system/benchmark", post(trigger_benchmark))
        .route("/api/v1/mcp/register", post(register_mcp))
        .with_state(state)
}

async fn submit_task(
    State(state): State<AppState>,
    Json(payload): Json<TaskSubmitRequest>
) -> Json<TaskSubmitResponse> {
    let job_id = uuid::Uuid::new_v4().to_string();
    tracing::info!("Received task submission: {:?}", payload);
    
    let task = TaskNode {
        id: job_id.clone(),
        instruction: payload.prompt,
        priority: payload.priority,
        estimated_time_ms: 1000,
    };
    
    // Ignore error if receiver dropped (hypervisor shutdown)
    let _ = state.task_sender.send(task).await;
    
    Json(TaskSubmitResponse {
        job_id,
        status: "QUEUED".into(),
    })
}

async fn register_mcp(
    State(_state): State<AppState>,
    Json(payload): Json<McpRegisterRequest>
) -> Json<serde_json::Value> {
    tracing::info!("Registering MCP tool: {}", payload.name);
    // In a full implementation, we'd add this to ContextEngine's dynamic tool registry.
    // For now, we just acknowledge the registration for the Pipeline Architect to use.
    Json(serde_json::json!({
        "status": "Registered",
        "tool": payload.name
    }))
}

async fn stream_task(Path(job_id): Path<String>) -> &'static str {
    tracing::info!("Client requested stream for job: {}", job_id);
    // SSE streaming will go here
    "Streaming... (SSE Placeholder)"
}

async fn queue_status() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "pending": 0,
        "running": 0,
    }))
}

async fn trigger_benchmark() -> Json<serde_json::Value> {
    tracing::info!("System benchmark triggered via API");
    Json(serde_json::json!({
        "status": "Benchmark started"
    }))
}
