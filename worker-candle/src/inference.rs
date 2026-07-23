use crate::model_loader::ModelWeights;
use anyhow::Context;
use candle_core::{Device, Tensor};
use hf_hub::api::sync::Api;
use shared_ipc::memory_map::StateHeader;
use shared_ipc::protocol::WorkerStatus;
use std::sync::atomic::Ordering;
use tokenizers::Tokenizer;
use tracing::{info, warn};

pub struct InferenceEngine {
    tokenizer: Option<Tokenizer>,
    device: Device,
}

impl InferenceEngine {
    pub fn new(_models_dir: &str) -> anyhow::Result<Self> {
        Ok(Self {
            tokenizer: None,
            device: Device::Cpu,
        })
    }

    pub fn load_local_tokenizer(&mut self, model_id: &str) -> anyhow::Result<()> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let mut tokenizer_path = std::path::PathBuf::from(home).join(".eva/models");

        // Check if model_id is a directory (Safetensors) with a tokenizer.json
        let dir_path = tokenizer_path.join(model_id);
        if dir_path.is_dir() && dir_path.join("tokenizer.json").exists() {
            tokenizer_path = dir_path.join("tokenizer.json");
        } else {
            // It's a GGUF, we might need a generic tokenizer or download one.
            // For now, if the user doesn't have tokenizer.json next to GGUF,
            // we will fallback to downloading TinyLlama tokenizer for basic decoding.
            info!("[InferenceEngine] GGUF detected without local tokenizer.json. Downloading generic tokenizer...");
            let api = Api::new()?;
            let repo = api.model("TinyLlama/TinyLlama-1.1B-Chat-v1.0".to_string());
            tokenizer_path = repo.get("tokenizer.json")?;
        }

        info!(
            "[InferenceEngine] Loading tokenizer from {:?}",
            tokenizer_path
        );
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Tokenizer parse error: {}", e))?;

        self.tokenizer = Some(tokenizer);
        info!("[InferenceEngine] Tokenizer loaded.");
        Ok(())
    }

    pub fn execute(
        &mut self,
        weights: &mut ModelWeights,
        header: &StateHeader,
        prompt: &str,
    ) -> anyhow::Result<()> {
        info!(
            "[InferenceEngine] Executing inference on prompt: {:?}",
            prompt
        );

        let tokenizer = match &self.tokenizer {
            Some(t) => t,
            None => anyhow::bail!("Tokenizer not loaded!"),
        };

        // 1. Tokenize prompt
        let tokens = tokenizer
            .encode(prompt, true)
            .map_err(|e| anyhow::anyhow!("Encode error: {}", e))?;

        let mut tokens = tokens.get_ids().to_vec();

        // 2. Generation Loop
        let mut generated_text = String::new();
        let max_tokens = 50; // Just generate 50 tokens for demo

        info!("[InferenceEngine] Starting generation loop...");
        for _i in 0..max_tokens {
            let current_status = header.status_flag.load(Ordering::SeqCst);
            if current_status == WorkerStatus::Interrupt as u32 {
                warn!("[InferenceEngine] Inference interrupted by Eva.");
                break;
            }

            let input = Tensor::new(tokens.as_slice(), &self.device)?.unsqueeze(0)?;

            let logits = match weights {
                ModelWeights::GgufLlama(model) => model.forward(&input, 0)?,
                ModelWeights::GgufQwen2(model) => model.forward(&input, 0)?,
                ModelWeights::Dummy(_) => {
                    // Simulate dummy output
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    Tensor::new(&[1u32], &self.device)?
                }
            };

            let next_token = if let ModelWeights::Dummy(_) = weights {
                1
            } else {
                let logits = logits.squeeze(0)?;
                let logits = logits.get(logits.dim(0)? - 1)?;
                logits.argmax_keepdim(0)?.to_vec1::<u32>()?[0]
            };

            tokens.push(next_token);

            if let Some(decoded) = tokenizer.decode(&[next_token], true).ok() {
                generated_text.push_str(&decoded);
                // IPC Streaming: Write to RingBuffer (we'll implement the actual ringbuffer write next)
                // For now, just log it so we see it's doing real math
                print!("{}", decoded);
                use std::io::Write;
                std::io::stdout().flush().unwrap();
            }
        }
        println!();
        info!("[InferenceEngine] Generation complete.");

        Ok(())
    }
}
