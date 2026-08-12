// Model registry for SL Studio

#![allow(dead_code)]

use anyhow::Result;
use std::collections::HashMap;
use tracing::info;

/// Manages registered LLM models and their download status.
#[derive(Default)]
pub struct ModelRegistry {
    models: HashMap<String, ModelInfo>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub source: String,
    pub mlx_model_name: String,  // e.g., "qwen3.5-4b-4bit"
    pub dtype: String,           // e.g., "float16", "bfloat16"
    pub context_length: usize,
    pub downloaded: bool,
    pub local_path: String,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, model: ModelInfo) {
        info!("Registered model: {}", model.id);
        self.models.insert(model.id.clone(), model);
    }

    pub fn get(&self, id: &str) -> Option<&ModelInfo> {
        self.models.get(id)
    }

    pub fn list(&self) -> Vec<&ModelInfo> {
        self.models.values().collect()
    }

    pub fn is_loaded(&self, id: &str) -> Result<bool> {
        let model = self.get(id).ok_or_else(|| anyhow::anyhow!("Model not found: {}", id))?;
        Ok(model.downloaded && !model.local_path.is_empty())
    }
}
