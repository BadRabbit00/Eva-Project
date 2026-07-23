use anyhow::Context;
use candle_core::quantized::gguf_file;
use candle_core::{Device, Tensor};
use candle_transformers::models::quantized_llama::ModelWeights as QLlama;
use candle_transformers::models::quantized_qwen2::ModelWeights as QQwen2;
use std::fs::File;
use std::path::PathBuf;
use tracing::{info, warn};

pub enum ModelWeights {
    GgufLlama(QLlama),
    GgufQwen2(QQwen2),
    Dummy(Tensor), // Fallback for unsupported Safetensors architectures for now
}

pub struct ModelLoader {
    models_dir: String,
    device: Device,
}

impl ModelLoader {
    pub fn new(models_dir: &str) -> Self {
        // Fallback models dir if not set properly
        let mut dir = models_dir.to_string();
        if dir == "/tmp" || dir == "" {
            if let Ok(home) = std::env::var("HOME") {
                dir = format!("{}/.eva/models", home);
            }
        }
        Self {
            models_dir: dir,
            device: Device::Cpu,
        }
    }

    pub fn load_weights(&self, model_id: &str) -> anyhow::Result<ModelWeights> {
        info!(
            "[ModelLoader] Request to load model: {} from {}",
            model_id, self.models_dir
        );

        let base_path = PathBuf::from(&self.models_dir);
        let gguf_path = base_path.join(format!("{}.gguf", model_id));
        let dir_path = base_path.join(model_id);

        if gguf_path.exists() {
            info!("[ModelLoader] Found local GGUF: {:?}", gguf_path);
            let mut file = File::open(&gguf_path)?;
            let gguf = gguf_file::Content::read(&mut file)?;

            // Detect architecture based on name or just try Qwen2 first
            let model_id_lower = model_id.to_lowercase();
            if model_id_lower.contains("qwen") || model_id_lower.contains("qwq") {
                info!("[ModelLoader] Loading as Qwen2 GGUF");
                let model = QQwen2::from_gguf(gguf, &mut file, &self.device)?;
                return Ok(ModelWeights::GgufQwen2(model));
            } else {
                info!("[ModelLoader] Loading as Llama GGUF");
                let model = QLlama::from_gguf(gguf, &mut file, &self.device)?;
                return Ok(ModelWeights::GgufLlama(model));
            }
        } else if dir_path.is_dir() {
            info!("[ModelLoader] Found local model directory: {:?}", dir_path);
            warn!("[ModelLoader] Safetensors dynamic architecture matching is WIP. Loading Dummy tensor for {:?}", dir_path);
            let dummy = Tensor::zeros((1024, 1024), candle_core::DType::F32, &self.device)?;
            return Ok(ModelWeights::Dummy(dummy));
        }

        anyhow::bail!("Model {} not found in ~/.eva/models", model_id);
    }
}
