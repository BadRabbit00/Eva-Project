pub mod api;
pub mod ipc;
pub mod state;
pub mod scheduler;
pub mod router;
pub mod engine;
pub mod context_engine;
pub mod config;

use tracing::info;
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender::rolling;
use tokio::net::TcpListener;
use tokio::process::Command;
use tokio::sync::mpsc;
use crate::ipc::shmem_manager::ShmemManager;
use crate::scheduler::{DagScheduler, TaskNode};
use crate::api::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let log_dir = std::path::PathBuf::from(home).join(".local/state/eva/logs");
    std::fs::create_dir_all(&log_dir)?;

    let file_appender = rolling::daily(&log_dir, "eva-daemon.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stdout))
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking).with_ansi(false))
        .init();

    info!("Hypervisor Core starting...");

    // Load config
    let daemon_config = config::load_config();
    info!("Loaded config: port={}, shmem_size_mb={}", daemon_config.port, daemon_config.shmem_size_mb);

    // Initialize IPC Shared Memory
    let shmem_size = daemon_config.shmem_size_mb * 1024 * 1024;
    let manager = ShmemManager::new(shmem_size)?;
    let os_id = manager.get_os_id().to_string();

    info!("Spawning worker-candle daemon (background)...");
    let _worker = Command::new("cargo")
        .arg("run")
        .arg("--bin")
        .arg("worker-candle")
        .arg("--")
        .arg("--shmem-id")
        .arg(&os_id)
        .spawn()?;

    // Initialize MPSC channel for task submission
    let (tx, rx) = mpsc::channel::<TaskNode>(100);

    // Spawn DagScheduler Event Loop
    let scheduler = DagScheduler::new();
    tokio::spawn(async move {
        scheduler.run_loop(rx).await;
    });

    let state = AppState { task_sender: tx };

    // Start Axum REST API
    let app = api::create_router(state);
    let bind_addr = format!("0.0.0.0:{}", daemon_config.port);
    let listener = TcpListener::bind(&bind_addr).await?;
    info!("Eva Hypervisor REST API running on {}", bind_addr);
    
    axum::serve(listener, app).await?;

    Ok(())
}
