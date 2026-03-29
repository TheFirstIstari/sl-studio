use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum LlamaError {
    #[error("Model not available: {0}")]
    NotAvailable(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlamaConfig {
    pub model_path: String,
    pub context_size: u32,
    pub gpu_layers: i32,
    pub temperature: f32,
    pub max_tokens: u32,
    pub repeat_penalty: f32,
}

impl Default for LlamaConfig {
    fn default() -> Self {
        LlamaConfig {
            model_path: String::new(),
            context_size: 16384,
            gpu_layers: 0,
            temperature: 0.0,
            max_tokens: 2000,
            repeat_penalty: 1.1,
        }
    }
}

pub struct LlamaModel {
    config: LlamaConfig,
    loaded: bool,
}

impl LlamaModel {
    pub fn new(config: LlamaConfig) -> Self {
        LlamaModel {
            config,
            loaded: false,
        }
    }

    pub fn load(&mut self) -> Result<(), LlamaError> {
        let model_path = Path::new(&self.config.model_path);
        
        if !model_path.exists() {
            return Err(LlamaError::NotAvailable(format!(
                "Model not found: {}", self.config.model_path
            )));
        }

        info!("Loading GGUF model: {}", self.config.model_path);
        self.loaded = true;
        info!("Model loaded (stub)");
        Ok(())
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    pub fn generate(&self, prompt: &str) -> Result<String, LlamaError> {
        if !self.loaded {
            return Err(LlamaError::NotAvailable("Model not loaded".to_string()));
        }
        
        info!("Generate called with prompt length: {} chars", prompt.len());
        Ok(r#"{"findings": [{"source": "stub", "date": "2024-01-01", "summary": "Test fact", "type": "General", "crime": "None", "severity": 1}]}"#.to_string())
    }

    pub fn generate_structured(&self, prompt: &str) -> Result<String, LlamaError> {
        self.generate(prompt)
    }

    pub fn unload(&mut self) {
        self.loaded = false;
        info!("Model unloaded");
    }
}

impl Drop for LlamaModel {
    fn drop(&mut self) {
        self.unload();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llama_config_default() {
        let config = LlamaConfig::default();
        assert_eq!(config.context_size, 16384);
        assert_eq!(config.max_tokens, 2000);
    }
}
