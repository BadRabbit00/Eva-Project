use std::sync::atomic::{AtomicU32, AtomicU64};

pub const HEADER_OFFSET: usize = 0;
pub const CONTROL_BLOCK_OFFSET: usize = 64;
pub const CONTROL_BLOCK_SIZE: usize = 1024 - 64;

/// The memory layout of the shared memory header.
/// Must be #[repr(C)] to ensure consistent layout across independent processes.
#[repr(C)]
pub struct StateHeader {
    /// Status flag indicating what the worker is doing. Maps to `WorkerStatus`.
    pub status_flag: AtomicU32,
    /// Heartbeat timestamp updated by the worker. Allows hypervisor to detect stalled processes.
    pub worker_heartbeat: AtomicU64,
    /// Padding to align the struct to exactly 64 bytes to prevent cache-line false sharing.
    pub _reserved: [u8; 48],
}

/// The memory layout of the control block.
#[repr(C)]
pub struct ControlBlock {
    /// A hash, identifier, or absolute path for the requested model weights/LoRA.
    pub model_id: [u8; 256],
    /// The size of the input prompt/context currently loaded in the input buffer.
    pub context_length: u32,
    /// Maximum number of tokens to generate.
    pub max_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, size_of};

    #[test]
    fn test_struct_layouts() {
        // Ensure structs have expected predictable sizes for safe IPC

        // StateHeader: AtomicU32 (4) + Padding (4) + AtomicU64 (8) + _reserved (48) = 64 bytes
        assert_eq!(size_of::<StateHeader>(), 64);
        assert_eq!(align_of::<StateHeader>(), 8);

        // ControlBlock: [u8; 256] (256) + u32 (4) + u32 (4) = 264 bytes
        assert_eq!(size_of::<ControlBlock>(), 264);
        assert_eq!(align_of::<ControlBlock>(), 4);
    }
}
