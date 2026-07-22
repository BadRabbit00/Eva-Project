pub mod ipc;
pub mod state;

use crate::ipc::shmem_manager::ShmemManager;
use tokio::process::Command;
use tracing::info;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).ok();

    info!("Hypervisor Core starting...");

    let shmem_size = 16 * 1024 * 1024; // 16 MB buffer
    let manager = ShmemManager::new(shmem_size)?;

    info!("Initial heartbeat: {}", manager.read_heartbeat());

    info!("Spawning worker-candle...");
    let mut child = Command::new("cargo")
        .arg("run")
        .arg("--bin")
        .arg("worker-candle")
        .arg("--")
        .arg("--shmem-id")
        .arg(manager.get_os_id())
        .spawn()?;

    child.wait().await?;

    let new_heartbeat = manager.read_heartbeat();
    info!("New heartbeat after worker execution: {}", new_heartbeat);

    if new_heartbeat > 0 {
        info!(
            "Handshake successful! The hypervisor and worker are communicating via Shared Memory."
        );
    } else {
        tracing::error!("Handshake failed, heartbeat is zero.");
    }

    Ok(())
}
