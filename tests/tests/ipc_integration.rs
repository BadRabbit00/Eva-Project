use shared_ipc::memory_map::StateHeader;
use shared_ipc::protocol::WorkerStatus;
use shared_memory::ShmemConf;
use std::sync::atomic::Ordering;
use std::time::Duration;

// A simple integration test verifying the IPC memory region and worker status preemption.
// Note: To run this effectively, the test should spawn the core/worker binaries or mock them.
// For now, this acts as the foundational structural mock for the workspace.

#[test]
fn test_ipc_segment_and_preemption() {
    // 1. Setup a dummy IPC block
    let shmem = ShmemConf::new().size(1024 * 1024 * 16).create().unwrap();

    let ptr = shmem.as_ptr();
    let header = unsafe { &*(ptr as *const StateHeader) };

    // 2. Validate structural alignment and write
    header
        .status_flag
        .store(WorkerStatus::Idle as u32, Ordering::SeqCst);
    assert_eq!(
        header.status_flag.load(Ordering::SeqCst),
        WorkerStatus::Idle as u32
    );

    // 3. Trigger Preemption
    header
        .status_flag
        .store(WorkerStatus::Interrupt as u32, Ordering::SeqCst);

    // The worker loop in inference.rs checks this exact flag to halt processing.
    assert_eq!(
        header.status_flag.load(Ordering::SeqCst),
        WorkerStatus::Interrupt as u32
    );
}
