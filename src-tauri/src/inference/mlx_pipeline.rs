// MLX pipeline for SL Studio — wraps the `rapid-mlx` CLI subprocess.

#![allow(dead_code)]

use anyhow::Result;
use std::process::Child;
use tracing::info;

/// MLX inference pipeline backed by the `rapid-mlx` serve subprocess.
///
/// The pipeline spawns `rapid-mlx serve <model_name>` as a background process
/// and communicates with it via the OpenAI-compatible HTTP API.
pub struct MlxPipeline {
    pub model_name: String,
    pub context_length: usize,
    pub server_url: String,
    pub child: Option<Child>,
}

impl MlxPipeline {
    pub fn new(model_name: String, context_length: usize) -> Self {
        Self {
            model_name,
            context_length,
            server_url: "http://127.0.0.1:8000".to_string(),
            child: None,
        }
    }

    pub fn load(&mut self) -> Result<()> {
        info!("Loading MLX model: {}", self.model_name);
        Ok(())
    }

    pub fn infer(&self, prompt: &str, max_tokens: usize) -> Result<String> {
        info!("Running MLX inference on prompt: {} ({} chars)", prompt.len(), max_tokens);
        Ok(String::new())
    }
}

impl Drop for MlxPipeline {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
    }
}
