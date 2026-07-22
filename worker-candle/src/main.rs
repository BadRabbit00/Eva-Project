use anyhow::Context;
use clap::Parser;
use shared_ipc::memory_map::{StateHeader, HEADER_OFFSET};
use shared_memory::ShmemConf;
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

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

    // Update heartbeat
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
    header.worker_heartbeat.store(now, Ordering::SeqCst);
    println!("Worker heartbeat updated to: {}", now);

    Ok(())
}
