/// Enum representing the status of the worker connected to the shared memory segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum WorkerStatus {
    /// Worker is idle and waiting for commands.
    Idle = 0x0,
    /// Worker is currently loading weights into VRAM/RAM.
    LoadWeights = 0x1,
    /// Worker is actively generating tokens (inference).
    ExecInfer = 0x2,
    /// Worker is writing tokens to the output stream.
    Streaming = 0x3,
    /// Worker is requesting data (e.g., from the hypervisor).
    ReqData = 0x4,
    /// Task has been completed successfully.
    Done = 0x5,
    /// An error occurred during execution.
    Error = 0x6,
    /// Execution must be interrupted (preempted by Hypervisor).
    Interrupt = 0x7,
}

impl WorkerStatus {
    /// Convert an integer value to a `WorkerStatus`, if valid.
    pub fn from_u32(val: u32) -> Option<Self> {
        match val {
            0x0 => Some(Self::Idle),
            0x1 => Some(Self::LoadWeights),
            0x2 => Some(Self::ExecInfer),
            0x3 => Some(Self::Streaming),
            0x4 => Some(Self::ReqData),
            0x5 => Some(Self::Done),
            0x6 => Some(Self::Error),
            0x7 => Some(Self::Interrupt),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_status_conversion() {
        assert_eq!(WorkerStatus::from_u32(0x2), Some(WorkerStatus::ExecInfer));
        assert_eq!(WorkerStatus::from_u32(0xFF), None);
    }
}
