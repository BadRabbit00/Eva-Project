pub mod api;
pub mod config;
pub mod context_engine;
pub mod discovery;
pub mod engine;
pub mod ipc;
pub mod registry;
pub mod router;
pub mod scheduler;
pub mod state;
pub mod worker_manager;

use crate::context_engine::ContextEngine;
use crate::registry::RegistryManager;
use crate::scheduler::{DagScheduler, TaskNode};
use crate::worker_manager::WorkerManager;
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tokio_stream::wrappers::UnixListenerStream;
use tracing::info;
use tracing_appender::rolling;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let log_dir = std::path::PathBuf::from(home.clone()).join(".local/state/eva/logs");
    std::fs::create_dir_all(&log_dir)?;

    let file_appender = rolling::daily(&log_dir, "eva.log");
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

    info!("Eva Core starting...");

    // Load config
    let daemon_config = config::load_config();
    info!(
        "Loaded config: port={}, shmem_size_mb={}",
        daemon_config.port, daemon_config.shmem_size_mb
    );

    // Initialize Registry Manager
    let registry_dir = std::env::current_dir()?.join("registry");
    let registry_manager = match RegistryManager::new(&registry_dir) {
        Ok(rm) => rm,
        Err(e) => {
            tracing::warn!(
                "Failed to initialize registries: {}. Running with empty registries.",
                e
            );
            RegistryManager::new("/tmp").unwrap_or_else(|_| RegistryManager::new(".").unwrap())
        }
    };
    info!(
        "Loaded CAT registry with {} tools",
        registry_manager.cat.tools.len()
    );
    info!(
        "Loaded Model registry with {} models",
        registry_manager.models.models.len()
    );

    // Initialize State Manager
    let state_db_path = log_dir.join("state.db");
    let state_manager = Arc::new(RwLock::new(crate::state::StateManager::new(state_db_path)?));

    // Initialize Worker Manager for Sub-Agents
    let shmem_size = daemon_config.shmem_size_mb * 1024 * 1024;
    let worker_manager = Arc::new(RwLock::new(WorkerManager::new(
        shmem_size,
        daemon_config.models_dir.clone(),
    )));

    // Spawn the primary Zero-Node worker
    info!("Spawning primary Zero-Node worker-candle daemon...");
    if let Err(e) = worker_manager.write().await.spawn_worker("zero-node") {
        tracing::error!("Failed to spawn zero-node: {}", e);
        return Err(e);
    }

    // Initialize MPSC channel for task submission
    let (tx, rx) = mpsc::channel::<TaskNode>(100);

    // Spawn DagScheduler Event Loop
    let scheduler = DagScheduler::new();

    let wm_clone = worker_manager.clone();
    let sm_clone = state_manager.clone();
    tokio::spawn(async move {
        scheduler.run_loop(rx, wm_clone, sm_clone).await;
    });

    // Initialize Context Engine
    let tools_dir = std::path::PathBuf::from(home).join(".config/eva/tools");
    let context_engine = Arc::new(RwLock::new(ContextEngine::new(tools_dir)));

    // Start gRPC API over Unix Domain Socket
    let socket_path = "/tmp/eva.sock";
    let _ = std::fs::remove_file(socket_path);
    let uds = UnixListener::bind(socket_path)?;
    let uds_stream = UnixListenerStream::new(uds);

    info!("Eva Eva gRPC running on {}", socket_path);

    tonic::transport::Server::builder()
        .add_service(shared_ipc::eva::eva_server::EvaServer::new(
            api::EvaService {
                task_sender: tx,
                context_engine,
                worker_manager,
            },
        ))
        .serve_with_incoming(uds_stream)
        .await?;

    Ok(())
}
