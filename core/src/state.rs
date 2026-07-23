use serde::{Deserialize, Serialize};
use sled::Db;
use std::path::Path;
use tracing::info;

/// The alpha value for Exponential Moving Average calculation.
/// Higher value gives more weight to recent observations.
const EMA_ALPHA: f64 = 0.2;

/// Hardware profile containing base metrics for T_estimate calculations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HardwareProfile {
    pub max_vram_mb: u64,
    pub max_ram_mb: u64,

    // Bandwidth metrics (ms per MB)
    pub disk_to_ram_ms_per_mb: f64,
    pub ram_to_vram_ms_per_mb: f64,

    // Default inference speeds (can be overridden per-model)
    pub cpu_prefill_tps: f64,
    pub cpu_decode_tps: f64,
    pub gpu_prefill_tps: f64,
    pub gpu_decode_tps: f64,
}

impl Default for HardwareProfile {
    fn default() -> Self {
        Self {
            max_vram_mb: 8192,          // 8GB default
            max_ram_mb: 16384,          // 16GB default
            disk_to_ram_ms_per_mb: 0.5, // 2GB/s NVMe
            ram_to_vram_ms_per_mb: 0.1, // 10GB/s PCIe
            cpu_prefill_tps: 50.0,
            cpu_decode_tps: 10.0,
            gpu_prefill_tps: 500.0,
            gpu_decode_tps: 50.0,
        }
    }
}

/// The state manager that wraps the sled embedded database.
pub struct StateManager {
    db: Db,
}

impl StateManager {
    /// Initializes or opens the sled database at the given path.
    pub fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let db = sled::open(path)?;
        info!("State database initialized successfully.");
        Ok(Self { db })
    }

    /// Saves the hardware profile into the database.
    pub fn save_hardware_profile(&self, profile: &HardwareProfile) -> anyhow::Result<()> {
        let serialized = serde_json::to_vec(profile)?;
        self.db.insert(b"hardware_profile", serialized)?;
        self.db.flush()?;
        Ok(())
    }

    /// Loads the hardware profile from the database.
    pub fn load_hardware_profile(&self) -> anyhow::Result<HardwareProfile> {
        if let Some(data) = self.db.get(b"hardware_profile")? {
            let profile: HardwareProfile = serde_json::from_slice(&data)?;
            Ok(profile)
        } else {
            Ok(HardwareProfile::default())
        }
    }

    /// Calculates and updates the Exponential Moving Average (EMA) for Time To First Token (TTFT).
    pub fn update_ttft_ema(&self, current_ttft: f64) -> anyhow::Result<f64> {
        let old_ttft = self.get_ttft_ema()?.unwrap_or(current_ttft);
        let new_ttft = (EMA_ALPHA * current_ttft) + ((1.0 - EMA_ALPHA) * old_ttft);
        self.db
            .insert(b"ema_ttft", new_ttft.to_le_bytes().to_vec())?;
        self.db.flush()?;
        Ok(new_ttft)
    }

    /// Retrieves the current EMA for TTFT.
    pub fn get_ttft_ema(&self) -> anyhow::Result<Option<f64>> {
        if let Some(data) = self.db.get(b"ema_ttft")? {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&data);
            Ok(Some(f64::from_le_bytes(bytes)))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ema_calculation_and_storage() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let state = StateManager { db };

        // Initially no TTFT
        assert_eq!(state.get_ttft_ema().unwrap(), None);

        // First update should set it exactly to the first value
        let val1 = state.update_ttft_ema(100.0).unwrap();
        assert_eq!(val1, 100.0);

        // Second update with 200.0
        // New = 0.2 * 200 + 0.8 * 100 = 40 + 80 = 120
        let val2 = state.update_ttft_ema(200.0).unwrap();
        assert_eq!(val2, 120.0);
    }

    #[test]
    fn test_hardware_profile_persistence() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let state = StateManager { db };

        let default_profile = state.load_hardware_profile().unwrap();
        assert_eq!(default_profile.max_vram_mb, 8192);

        let mut new_profile = default_profile.clone();
        new_profile.max_vram_mb = 16384;

        state.save_hardware_profile(&new_profile).unwrap();

        let loaded = state.load_hardware_profile().unwrap();
        assert_eq!(loaded.max_vram_mb, 16384);
    }
}
