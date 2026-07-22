use axum::{
    routing::{get, post},
    Router, Json, extract::Path,
};
use serde::{Deserialize, Serialize};

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

pub fn create_router() -> Router {
    Router::new()
        .route("/api/v1/task/submit", post(submit_task))
        .route("/api/v1/task/stream/:job_id", get(stream_task))
        .route("/api/v1/hypervisor/queue", get(queue_status))
        .route("/api/v1/system/benchmark", post(trigger_benchmark))
}

async fn submit_task(Json(payload): Json<TaskSubmitRequest>) -> Json<TaskSubmitResponse> {
    let job_id = uuid::Uuid::new_v4().to_string();
    tracing::info!("Received task submission: {:?}", payload);
    
    Json(TaskSubmitResponse {
        job_id,
        status: "QUEUED".into(),
    })
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
