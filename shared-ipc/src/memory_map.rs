use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize};

pub const HEADER_OFFSET: usize = 0x0;
pub const CONTROL_BLOCK_OFFSET: usize = 0x40; // 64 bytes
pub const INPUT_BUFFER_OFFSET: usize = 0x440; // 64 + 1024 = 1088 bytes
pub const OUTPUT_RING_BUFFER_OFFSET: usize = 0x100440; // 1088 + 1MB
pub const RING_BUFFER_CAPACITY: usize = 1024 * 1024; // 1 MB

#[repr(C)]
pub struct StateHeader {
    pub status_flag: AtomicU32,       // 4 bytes
    // implicit 4 bytes padding
    pub worker_heartbeat: AtomicU64,  // 8 bytes
    pub _reserved: [u8; 48],          // 4 + 4 + 8 + 48 = 64 bytes
}

#[repr(C)]
pub struct ControlBlock {
    pub model_id: [u8; 256],          // 256 bytes
    pub context_length: u32,          // 4 bytes
    pub max_tokens: u32,              // 4 bytes
    pub _reserved: [u8; 760],         // 256 + 4 + 4 + 760 = 1024 bytes (1 KB)
}

#[repr(C)]
pub struct OutputRingBuffer {
    pub head: AtomicUsize,            // 8 bytes (on 64-bit)
    pub tail: AtomicUsize,            // 8 bytes
    pub buffer: [u8; RING_BUFFER_CAPACITY],
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, size_of};

    #[test]
    fn test_struct_layouts() {
        assert_eq!(size_of::<StateHeader>(), 64);
        assert_eq!(align_of::<StateHeader>(), 8);

        assert_eq!(size_of::<ControlBlock>(), 1024);
        
        let ring_buffer_size = 16 + RING_BUFFER_CAPACITY;
        assert_eq!(size_of::<OutputRingBuffer>(), ring_buffer_size);
    }
}
