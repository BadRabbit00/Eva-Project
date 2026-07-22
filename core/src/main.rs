pub mod api;
pub mod ipc;
pub mod state;
pub mod scheduler;
pub mod router;
pub mod engine;
pub mod context_engine;

use tracing::info;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).ok();

    info!("Hypervisor Core starting...");

    // Start Axum REST API
    let app = api::create_router();
    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    info!("API server listening on 0.0.0.0:3000");
    
    axum::serve(listener, app).await?;

    Ok(())
}
