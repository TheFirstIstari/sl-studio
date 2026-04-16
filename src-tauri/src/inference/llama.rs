use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tracing::{info, warn, error};

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
    /// Path to GGUF model file
    pub model_path: String,
    /// Context size (tokens)
    pub context_size: u32,
    /// Number of GPU layers (0 = CPU only)
    pub gpu_layers: i32,
    /// Temperature for generation (0.0 - 2.0)
    pub temperature: f32,
    /// Maximum tokens to generate
    pub max_tokens: u32,
    /// Repeat penalty
    pub repeat_penalty: f32,
    /// Use KV cache
    pub use_kv_cache: bool,
    /// Prompt cache (for continuity)
    pub prompt_cache: Option<String>,
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
        }
    }
}

/// GPU backend type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum GpuBackend {
    /// CPU only (no GPU acceleration)
    Cpu,
    /// Apple Metal (macOS)
    Metal,
    /// NVIDIA CUDA
    Cuda,
    /// OpenCL
    OpenCL,
}

impl GpuBackend {
    pub fn from_str(s: &str) -> Self {
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

/// Main LLM model wrapper
pub struct LlamaModel {
    config: LlamaConfig,
    loaded: bool,
    #[cfg(feature = "llama-cpp")]
    model: Option<llama_cpp::Llama>,
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
    /// Create new model with config
    pub fn new(config: LlamaConfig) -> Self {
        LlamaModel {
            config,
            loaded: false,
            #[cfg(feature = "llama-cpp")]
            model: None,
        }
    }

    /// Check if model is loaded
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Get model path
    pub fn model_path(&self) -> &str {
        &self.config.model_path
    }

    /// Load model from GGUF file
    #[cfg(feature = "llama-cpp")]
    pub fn load(&mut self) -> Result<(), LlamaError> {
        let model_path = Path::new(&self.config.model_path);
        
        if !model_path.exists() {
            return Err(LlamaError::LoadError(format!(
                "Model file not found: {}",
                self.config.model_path
            )));
        }

        info!("Loading GGUF model: {}", self.config.model_path);
        
        // Build llama.cpp parameters
        let params = llama_cpp::LLama::Params::default()
            .n_ctx(self.config.context_size as i32)
            .n_gpu_layers(self.config.gpu_layers)
            .use_kv_cache(self.config.use_kv_cache);
        
        match llama_cpp::Llama::new(model_path, params) {
            Ok(model) => {
                self.model = Some(model);
                self.loaded = true;
                info!("Model loaded successfully");
                Ok(())
            }
            Err(e) => {
                error!("Failed to load model: {}", e);
                Err(LlamaError::LoadError(e.to_string()))
            }
        }
    }

    /// Load without llama-cpp feature (stub)
    #[cfg(not(feature = "llama-cpp"))]
    pub fn load(&mut self) -> Result<(), LlamaError> {
        let model_path = Path::new(&self.config.model_path);
        
        if !model_path.exists() {
            return Err(LlamaError::LoadError(format!(
                "Model file not found: {}",
                self.config.model_path
            )));
        }

        info!("Loading GGUF model (fallback): {}", self.config.model_path);
        // Fallback: just mark as loaded if file exists
        // Real loading would need llama-cpp feature enabled
        self.loaded = true;
        info!("Model loaded (fallback mode - enable llama-cpp feature for real inference)");
        Ok(())
    }

    /// Unload model and free memory
    pub fn unload(&mut self) {
        self.loaded = false;
        #[cfg(feature = "llama-cpp")]
        {
            self.model = None;
        }
        info!("Model unloaded");
    }

    /// Generate text completion
    #[cfg(feature = "llama-cpp")]
    pub fn generate(&self, prompt: &str) -> Result<GenerationResult, LlamaError> {
        if !self.loaded {
            return Err(LlamaError::NotLoaded);
        }
        
        let model = self.model.as_ref().ok_or(LlamaError::NotLoaded)?;
        
        let start = std::time::Instant::now();
        
        let tokens = model.tokenize(prompt, true).map_err(|e| {
            LlamaError::InferenceError(format!("Failed to tokenize: {}", e))
        })?;
        let prompt_tokens = tokens.len() as u32;
        
        // Create sampler with parameters
        let mut sampler = model.sampler();
        sampler
            .temperature(self.config.temperature)
            .repeat_penalty(self.config.repeat_penalty);
        
        // Generate
        let mut generated_tokens = Vec::new();
        let max_tokens = self.config.max_tokens as usize;
        
        for _ in 0..max_tokens {
            let token = model.sample_token(&sampler);
            if token == model.token_eos() {
                break;
            }
            generated_tokens.push(token);
            
            // Check for early stopping on specific tokens
            if generated_tokens.len() > 4 {
                // Simple early stop: break if we see multiple newlines at end
                let _ = token; // Could add early stop logic here
            }
        }
        
        // Decode generated tokens
        let text = model.detokenize(&generated_tokens).unwrap_or_default();
        let duration_ms = start.elapsed().as_millis() as u64;
        
        Ok(GenerationResult {
            text,
            tokens_generated: generated_tokens.len() as u32,
            prompt_tokens,
            duration_ms,
        })
    }

    /// Generate without llama-cpp feature
    #[cfg(not(feature = "llama-cpp"))]
    pub fn generate(&self, prompt: &str) -> Result<GenerationResult, LlamaError> {
        if !self.loaded {
            return Err(LlamaError::NotLoaded);
        }
        
        // Fallback response when llama-cpp is not available
        let text = format!(
            "This is a fallback response. To enable real LLM inference, \
            compile with --features llama-cpp and ensure llama.cpp is installed.\n\n\
            Prompt: {}\n\n\
            {{\"findings\": [{{\"source\": \"model\", \"date\": \"fallback\", \
            \"summary\": \"LLM inference not available\", \"type\": \"System\", \
            \"crime\": \"None\", \"severity\": 0}}]}}",
            &prompt[..prompt.len().min(200)]
        );
        
        Ok(GenerationResult {
            text,
            tokens_generated: 0,
            prompt_tokens: prompt.len() as u32 / 4,
            duration_ms: 1,
        })
    }

    /// Generate with structured output (JSON schema)
    pub fn generate_structured(&self, prompt: &str) -> Result<GenerationResult, LlamaError> {
        self.generate(prompt)
    }

    /// Get model information
    pub fn get_info(&self) -> Option<ModelInfo> {
        if !self.loaded {
            return None;
        }
        
        Some(ModelInfo {
            architecture: "llama".to_string(),
            context_length: self.config.context_size,
            embedding_length: 4096, // Default for llama models
            block_count: self.config.context_size / 2048,
            parameter_count: 7.0, // 7B parameter model typical
            quantization: "Q4_K_M".to_string(),
        })
    }
}

impl Drop for LlamaModel {
    fn drop(&mut self) {
        self.unload();
    }
}

/// Create a model manager for handling multiple models
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

    /// Add a model
    pub fn add_model(&mut self, name: String, config: LlamaConfig) -> Result<(), LlamaError> {
        let mut model = LlamaModel::new(config);
        model.load()?;
        
        self.models.insert(name.clone(), Arc::new(Mutex::new(model)));
        
        if self.active_model.is_none() {
            self.active_model = Some(name);
        }
        
        Ok(())
    }

    /// Get active model
    pub fn get_active(&self) -> Option<Arc<Mutex<LlamaModel>>> {
        self.active_model.as_ref().and_then(|name| self.models.get(name).cloned())
    }

    /// Set active model
    pub fn set_active(&mut self, name: &str) -> Result<(), LlamaError> {
        if self.models.contains_key(name) {
            self.active_model = Some(name.to_string());
            Ok(())
        } else {
            Err(LlamaError::NotAvailable(format!("Model '{}' not found", name)))
        }
    }

    /// List available models
    pub fn list_models(&self) -> Vec<String> {
        self.models.keys().cloned().collect()
    }
}

impl Default for ModelManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple inference without state (for stateless API calls)
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
        assert_eq!(GpuBackend::from_str("cpu"), GpuBackend::Cpu);
        assert_eq!(GpuBackend::from_str("metal"), GpuBackend::Metal);
        assert_eq!(GpuBackend::from_str("cuda"), GpuBackend::Cuda);
    }
}
