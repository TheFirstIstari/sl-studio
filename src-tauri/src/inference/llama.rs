use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tracing::{error, info};

#[derive(Error, Debug)]
pub enum LlamaError {
    #[error("Model not available: {0}")]
    NotAvailable(String),
    #[error("Failed to load model: {0}")]
    LoadError(String),
    #[error("Inference error: {0}")]
    InferenceError(String),
    #[error("Model not loaded")]
    NotLoaded,
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
}

/// Configuration for LLM inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlamaConfig {
    pub model_path: String,
    pub context_size: u32,
    pub gpu_layers: i32,
    pub temperature: f32,
    pub max_tokens: u32,
    pub repeat_penalty: f32,
    pub use_kv_cache: bool,
    pub prompt_cache: Option<String>,
    pub n_threads: u32,
}

impl Default for LlamaConfig {
    fn default() -> Self {
        LlamaConfig {
            model_path: String::new(),
            context_size: 4096,
            gpu_layers: 0,
            temperature: 0.1,
            max_tokens: 1024,
            repeat_penalty: 1.1,
            use_kv_cache: true,
            prompt_cache: None,
            n_threads: num_cpus::get() as u32,
        }
    }
}

/// GPU backend type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum GpuBackend {
    Cpu,
    Metal,
    Cuda,
    OpenCL,
}

impl GpuBackend {
    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "metal" => GpuBackend::Metal,
            "cuda" => GpuBackend::Cuda,
            "opencl" => GpuBackend::OpenCL,
            _ => GpuBackend::Cpu,
        }
    }
}

/// Model info from GGUF file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub architecture: String,
    pub context_length: u32,
    pub embedding_length: u32,
    pub block_count: u32,
    pub parameter_count: f64,
    pub quantization: String,
}

/// Main LLM model wrapper using llama_cpp 0.3 API
pub struct LlamaModel {
    config: LlamaConfig,
    loaded: bool,
    model: Option<llama_cpp::LlamaModel>,
}

/// Generation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResult {
    pub text: String,
    pub tokens_generated: u32,
    pub prompt_tokens: u32,
    pub duration_ms: u64,
}

impl LlamaModel {
    pub fn new(config: LlamaConfig) -> Self {
        LlamaModel {
            config,
            loaded: false,
            model: None,
        }
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    pub fn model_path(&self) -> &str {
        &self.config.model_path
    }

    /// Load model from GGUF file
    pub fn load(&mut self) -> Result<(), LlamaError> {
        let model_path = Path::new(&self.config.model_path);

        if !model_path.exists() {
            return Err(LlamaError::LoadError(format!(
                "Model file not found: {}",
                self.config.model_path
            )));
        }

        info!("Loading GGUF model: {}", self.config.model_path);

        let params = llama_cpp::LlamaParams {
            n_gpu_layers: self.config.gpu_layers as u32,
            ..Default::default()
        };

        let model = llama_cpp::LlamaModel::load_from_file(model_path, params).map_err(|e| {
            error!("Failed to load model: {}", e);
            LlamaError::LoadError(e.to_string())
        })?;

        self.model = Some(model);
        self.loaded = true;
        info!("Model loaded successfully");
        Ok(())
    }

    pub fn unload(&mut self) {
        self.loaded = false;
        self.model = None;
        info!("Model unloaded");
    }

    /// Generate text completion using session-based API
    pub fn generate(&self, prompt: &str) -> Result<GenerationResult, LlamaError> {
        if !self.loaded {
            return Err(LlamaError::NotLoaded);
        }

        let model = self.model.as_ref().ok_or(LlamaError::NotLoaded)?;

        let start = std::time::Instant::now();

        // Tokenize the prompt
        let prompt_bytes = prompt.as_bytes();
        let tokens = model
            .tokenize_bytes(prompt_bytes, true, true)
            .map_err(|e| LlamaError::InferenceError(format!("Failed to tokenize: {}", e)))?;
        let prompt_tokens = tokens.len() as u32;

        // Create session params
        let session_params = llama_cpp::SessionParams {
            n_ctx: self.config.context_size,
            n_threads: self.config.n_threads,
            ..Default::default()
        };

        let mut session = model
            .create_session(session_params)
            .map_err(|e| LlamaError::InferenceError(e.to_string()))?;

        // Advance context with tokens
        session
            .advance_context_with_tokens(&tokens)
            .map_err(|e| LlamaError::InferenceError(e.to_string()))?;

        // Start completion - handle Result
        let mut completion = session
            .start_completing()
            .map_err(|e| LlamaError::InferenceError(e.to_string()))?;

        let mut generated_tokens = Vec::new();
        let max_tokens = self.config.max_tokens as usize;

        // Generate tokens
        for _ in 0..max_tokens {
            match completion.next_token() {
                Some(token) => {
                    if token == model.eos() {
                        break;
                    }
                    generated_tokens.push(token);
                }
                None => break,
            }
        }

        // Convert to string using into_string() on CompletionHandle
        let text = completion.into_string();

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(GenerationResult {
            text,
            tokens_generated: generated_tokens.len() as u32,
            prompt_tokens,
            duration_ms,
        })
    }

    pub fn generate_structured(&self, prompt: &str) -> Result<GenerationResult, LlamaError> {
        self.generate(prompt)
    }

    pub fn get_info(&self) -> Option<ModelInfo> {
        if !self.loaded {
            return None;
        }

        let model = self.model.as_ref()?;

        Some(ModelInfo {
            architecture: "llama".to_string(),
            context_length: self.config.context_size,
            embedding_length: model.embed_len() as u32,
            block_count: model.layers() as u32,
            parameter_count: 7.0,
            quantization: "Q4_K_M".to_string(),
        })
    }
}

impl Drop for LlamaModel {
    fn drop(&mut self) {
        self.unload();
    }
}

/// Model manager
pub struct ModelManager {
    models: std::collections::HashMap<String, Arc<Mutex<LlamaModel>>>,
    active_model: Option<String>,
}

impl ModelManager {
    pub fn new() -> Self {
        ModelManager {
            models: std::collections::HashMap::new(),
            active_model: None,
        }
    }

    pub fn add_model(&mut self, name: String, config: LlamaConfig) -> Result<(), LlamaError> {
        let mut model = LlamaModel::new(config);
        model.load()?;

        self.models
            .insert(name.clone(), Arc::new(Mutex::new(model)));

        if self.active_model.is_none() {
            self.active_model = Some(name);
        }

        Ok(())
    }

    pub fn get_active(&self) -> Option<Arc<Mutex<LlamaModel>>> {
        self.active_model
            .as_ref()
            .and_then(|name| self.models.get(name).cloned())
    }

    pub fn set_active(&mut self, name: &str) -> Result<(), LlamaError> {
        if self.models.contains_key(name) {
            self.active_model = Some(name.to_string());
            Ok(())
        } else {
            Err(LlamaError::NotAvailable(format!(
                "Model '{}' not found",
                name
            )))
        }
    }

    pub fn list_models(&self) -> Vec<String> {
        self.models.keys().cloned().collect()
    }
}

impl Default for ModelManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple inference
pub struct SimpleInference {
    model: Arc<Mutex<LlamaModel>>,
}

impl SimpleInference {
    pub fn new(model: LlamaModel) -> Self {
        SimpleInference {
            model: Arc::new(Mutex::new(model)),
        }
    }

    pub fn generate(&self, prompt: &str) -> Result<GenerationResult, LlamaError> {
        self.model.lock().unwrap().generate(prompt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llama_config_default() {
        let config = LlamaConfig::default();
        assert_eq!(config.context_size, 4096);
        assert_eq!(config.max_tokens, 1024);
        assert_eq!(config.temperature, 0.1);
    }

    #[test]
    fn test_gpu_backend_from_str() {
        assert_eq!(GpuBackend::from_string("cpu"), GpuBackend::Cpu);
        assert_eq!(GpuBackend::from_string("metal"), GpuBackend::Metal);
        assert_eq!(GpuBackend::from_string("cuda"), GpuBackend::Cuda);
    }
}
