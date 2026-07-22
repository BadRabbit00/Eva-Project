pub mod model_loader;
pub mod inference;

use anyhow::Context;
use clap::Parser;
use shared_ipc::memory_map::{StateHeader, HEADER_OFFSET};
use shared_ipc::protocol::WorkerStatus;
use shared_memory::ShmemConf;
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::thread;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The OS ID of the shared memory segment
    #[arg(long)]
    shmem_id: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    println!(
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
    println!("Initial status from Hypervisor: {}", current_status);

    println!("Entering worker event loop...");
    
    let loader = model_loader::ModelLoader::new()?;
    let mut engine = inference::InferenceEngine::new();
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
                println!("[Worker] Command received: LoadWeights");
                // Dummy model ID for now. We will read this from the ControlBlock later.
                let model_id = "dummy_model";
                
                // Attempt to load tokenizer, but continue even if it fails (using stub)
                if let Err(e) = engine.load_local_tokenizer(model_id) {
                    println!("[Worker] Tokenizer load failed: {}. Continuing with stub.", e);
                }

                match loader.load_safetensors(model_id) {
                    Ok(weights) => {
                        current_weights = Some(weights);
                        header.status_flag.store(WorkerStatus::Idle as u32, Ordering::SeqCst);
                    }
                    Err(e) => {
                        println!("[Worker] Failed to load weights: {}", e);
                        header.status_flag.store(WorkerStatus::Error as u32, Ordering::SeqCst);
                    }
                }
            }
            Some(WorkerStatus::ExecInfer) => {
                println!("[Worker] Command received: ExecInfer");
                if let Some(ref weights) = current_weights {
                    // Execute inference with a stub prompt
                    if let Err(e) = engine.execute(weights, header, "Test prompt") {
                        println!("[Worker] Inference failed: {}", e);
                        header.status_flag.store(WorkerStatus::Error as u32, Ordering::SeqCst);
                    } else {
                        header.status_flag.store(WorkerStatus::Done as u32, Ordering::SeqCst);
                    }
                } else {
                    println!("[Worker] Error: Tried to execute inference without loaded weights!");
                    header.status_flag.store(WorkerStatus::Error as u32, Ordering::SeqCst);
                }
            }
            Some(WorkerStatus::Streaming) | Some(WorkerStatus::ReqData) => {
                thread::sleep(Duration::from_millis(10));
            }
            Some(WorkerStatus::Error) => {
                println!("[Worker] Error state detected. Waiting for hypervisor.");
                thread::sleep(Duration::from_secs(1));
            }
            None => {
                println!("[Worker] Unknown status flag.");
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}
