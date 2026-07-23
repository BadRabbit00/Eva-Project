use anyhow::{Context, Result};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use shared_ipc::eva::hypervisor_client::HypervisorClient;
use shared_ipc::eva::{QueueRequest, RegistryRequest, SubmitRequest};
use std::sync::Arc;
use tokio::net::UnixStream;
use tokio::sync::Mutex;
use tonic::transport::{Channel, Endpoint};
use tower::service_fn;
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
struct AppState {
    client: Arc<Mutex<HypervisorClient<Channel>>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let log_dir = std::path::PathBuf::from(home).join(".local/state/eva/logs");
    std::fs::create_dir_all(&log_dir)?;

    let file_appender = tracing_appender::rolling::daily(&log_dir, "eva-api.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stdout))
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false),
        )
        .init();

    tracing::info!("Starting Eva External API Gateway...");

    // 1. Setup gRPC client connection to eva-daemon (UDS)
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(|_| async {
            // Hyper 1.0 requires TokioIo wrapper, but tonic 0.11 uses hyper 0.14.
            // Actually, hyper 0.14 requires implementing Connection for the stream.
            // We use hyper_0_14's traits, but since we don't have it explicitly,
            // let's rely on tonic's internal implementations or standard wrappers.
            // For now, let's try just returning UnixStream directly.
            UnixStream::connect("/tmp/eva.sock").await
        }))
        .await
        .context("Failed to connect to /tmp/eva.sock. Is the Eva Core Daemon running?")?;

    let client = HypervisorClient::new(channel);
    let state = AppState {
        client: Arc::new(Mutex::new(client)),
    };

    // 2. Setup Axum REST router
    let api_routes = Router::new()
        .route("/v1/tasks", post(submit_task))
        .route("/v1/scheduler/queue", get(queue_status))
        .route("/v1/registry/models", get(get_models));

    let app = Router::new()
        .nest("/api", api_routes)
        // Serve the Svelte UI static files
        .fallback_service(ServeDir::new("../ui/dist"))
        .with_state(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("Listening for external UI connections on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// ==========================================
// REST Endpoints
// ==========================================

#[derive(serde::Deserialize)]
struct TaskSubmitPayload {
    prompt: String,
    template_id: Option<String>,
    priority: Option<u32>,
}

async fn submit_task(
    State(state): State<AppState>,
    Json(payload): Json<TaskSubmitPayload>,
) -> Json<serde_json::Value> {
    let mut client = state.client.lock().await;

    let req = tonic::Request::new(SubmitRequest {
        prompt: payload.prompt,
        template_id: payload.template_id.unwrap_or_default(),
        priority: payload.priority.unwrap_or(9),
    });

    match client.submit_task(req).await {
        Ok(response) => {
            let res = response.into_inner();
            Json(serde_json::json!({
                "task_id": res.task_id,
                "status": res.status
            }))
        }
        Err(e) => {
            tracing::error!("gRPC call failed: {}", e);
            Json(serde_json::json!({
                "error": e.to_string()
            }))
        }
    }
}

async fn queue_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mut client = state.client.lock().await;

    let req = tonic::Request::new(QueueRequest {});

    match client.get_queue(req).await {
        Ok(response) => {
            let res = response.into_inner();
            // Try to parse the json string returned by gRPC
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&res.json_dump) {
                Json(parsed)
            } else {
                Json(serde_json::json!({"data": res.json_dump}))
            }
        }
        Err(e) => {
            tracing::error!("gRPC call failed: {}", e);
            Json(serde_json::json!({"error": e.to_string()}))
        }
    }
}

async fn get_models(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mut client = state.client.lock().await;

    let req = tonic::Request::new(RegistryRequest {});

    match client.get_registries(req).await {
        Ok(response) => {
            let res = response.into_inner();
            Json(serde_json::json!({
                "models": res.models_yaml,
                "tools": res.tools_yaml
            }))
        }
        Err(e) => {
            tracing::error!("gRPC call failed: {}", e);
            Json(serde_json::json!({"error": e.to_string()}))
        }
    }
}
