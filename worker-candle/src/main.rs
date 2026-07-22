pub mod inference;
pub mod model_loader;

use anyhow::Context;
use clap::Parser;
use shared_ipc::memory_map::{StateHeader, HEADER_OFFSET};
use shared_ipc::protocol::WorkerStatus;
use shared_memory::ShmemConf;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};
use tracing_appender::rolling;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The OS ID of the shared memory segment
    #[arg(short, long)]
    shmem_id: String,

    #[arg(short, long, default_value = "~/.eva/models")]
    models_dir: String,
}

fn main() -> anyhow::Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let log_dir = std::path::PathBuf::from(home).join(".local/state/eva/logs");
    std::fs::create_dir_all(&log_dir)?;

    let file_appender = rolling::daily(&log_dir, "eva-worker.log");
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

    let args = Args::parse();
    info!(
        "Worker Node starting, connecting to shmem-id: {}",
        args.shmem_id
    );

    let shmem = ShmemConf::new()
        .os_id(&args.shmem_id)
        .open()
        .context("Failed to open shared memory")?;
    let ptr = shmem.as_ptr();

    // Safety: IPC contract guarantees the header is at HEADER_OFFSET
    let header = unsafe { &*(ptr.add(HEADER_OFFSET) as *const StateHeader) };

    // Read current status
    let current_status = header.status_flag.load(Ordering::SeqCst);
    info!("Initial status from Hypervisor: {}", current_status);

    info!("Entering worker event loop...");

    let loader = model_loader::ModelLoader::new(&args.models_dir);
    let mut engine = inference::InferenceEngine::new(&args.models_dir)?;
    let mut current_weights = None;

    loop {
        // Update heartbeat for hypervisor watchdog
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
        header.worker_heartbeat.store(now, Ordering::SeqCst);

        let status = WorkerStatus::from_u32(header.status_flag.load(Ordering::SeqCst));

        match status {
            Some(WorkerStatus::Idle) | Some(WorkerStatus::Done) => {
                thread::sleep(Duration::from_millis(50));
            }
            Some(WorkerStatus::LoadWeights) => {
                info!("[Worker] Command received: LoadWeights");
                // Dummy model ID for now. We will read this from the ControlBlock later.
                let model_id = "dummy_model";

                // Attempt to load tokenizer, but continue even if it fails (using stub)
                if let Err(e) = engine.load_local_tokenizer(&model_id) {
                    warn!(
                        "[Worker] Tokenizer load failed: {}. Continuing with stub.",
                        e
                    );
                }

                match loader.load_weights(&model_id) {
                    Ok(weights) => {
                        current_weights = Some(weights);
                        header
                            .status_flag
                            .store(WorkerStatus::Idle as u32, Ordering::SeqCst);
                    }
                    Err(e) => {
                        error!("[Worker] Failed to load weights: {}", e);
                        header
                            .status_flag
                            .store(WorkerStatus::Error as u32, Ordering::SeqCst);
                    }
                }
            }
            Some(WorkerStatus::ExecInfer) => {
                info!("[Worker] Command received: ExecInfer");
                if let Some(ref weights) = current_weights {
                    // Execute inference with a stub prompt
                    if let Err(e) = engine.execute(weights, header, "Test prompt") {
                        error!("[Worker] Inference failed: {}", e);
                        header
                            .status_flag
                            .store(WorkerStatus::Error as u32, Ordering::SeqCst);
                    } else {
                        header
                            .status_flag
                            .store(WorkerStatus::Done as u32, Ordering::SeqCst);
                    }
                } else {
                    error!("[Worker] Error: Tried to execute inference without loaded weights!");
                    header
                        .status_flag
                        .store(WorkerStatus::Error as u32, Ordering::SeqCst);
                }
            }
            Some(WorkerStatus::Streaming) | Some(WorkerStatus::ReqData) => {
                thread::sleep(Duration::from_millis(10));
            }
            Some(WorkerStatus::Error) => {
                info!("[Worker] Error state detected. Waiting for hypervisor.");
                thread::sleep(Duration::from_secs(1));
            }
            None => {
                warn!("[Worker] Unknown status flag.");
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}
