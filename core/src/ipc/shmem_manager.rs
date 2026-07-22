use shared_ipc::memory_map::{StateHeader, HEADER_OFFSET};
use shared_ipc::protocol::WorkerStatus;
use shared_memory::{Shmem, ShmemConf};
use std::sync::atomic::Ordering;
use tracing::info;

pub struct ShmemManager {
    shmem: Shmem,
}

impl ShmemManager {
    /// Creates a new shared memory segment and initializes the header.
    pub fn new(size: usize) -> anyhow::Result<Self> {
        let shmem = ShmemConf::new().size(size).create()?;
        let os_id = shmem.get_os_id().to_string();
        info!("Created shared memory segment with OS ID: {}", os_id);

        let manager = Self { shmem };
        manager.init_header();
        
        Ok(manager)
    }

    /// Returns the OS identifier for the shared memory segment, which can be passed to the worker.
    pub fn get_os_id(&self) -> &str {
        self.shmem.get_os_id()
    }

    /// Initializes the StateHeader at the beginning of the shared memory.
    fn init_header(&self) {
        let ptr = self.shmem.as_ptr();
        // Safety: We know the memory segment is at least 64 bytes and aligned by the OS.
        let header = unsafe { &*(ptr.add(HEADER_OFFSET) as *const StateHeader) };
        
        header.status_flag.store(WorkerStatus::Idle as u32, Ordering::SeqCst);
        header.worker_heartbeat.store(0, Ordering::SeqCst);
    }
    
    /// Reads the current status from the header.
    pub fn read_status(&self) -> Option<WorkerStatus> {
        let ptr = self.shmem.as_ptr();
        let header = unsafe { &*(ptr.add(HEADER_OFFSET) as *const StateHeader) };
        WorkerStatus::from_u32(header.status_flag.load(Ordering::SeqCst))
    }
    
    /// Reads the current heartbeat from the header.
    pub fn read_heartbeat(&self) -> u64 {
        let ptr = self.shmem.as_ptr();
        let header = unsafe { &*(ptr.add(HEADER_OFFSET) as *const StateHeader) };
        header.worker_heartbeat.load(Ordering::SeqCst)
    }
}
