use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum ModelError {
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Model not found: {0}")]
    NotFound(String),
    #[error("Invalid model: {0}")]
    InvalidModel(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub quantization: Option<String>,
    pub context_size: u32,
    pub is_loaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quantization {
    pub id: String,
    pub name: String,
    pub bits: u32,
    pub description: String,
}

impl Quantization {
    pub fn available() -> Vec<Quantization> {
        vec![
            Quantization {
                id: "Q4_K_M".to_string(),
                name: "Q4_K_M".to_string(),
                bits: 4,
                description: "Medium quality, good speed".to_string(),
            },
            Quantization {
                id: "Q5_K_S".to_string(),
                name: "Q5_K_S".to_string(),
                bits: 5,
                description: "Small file, reasonable quality".to_string(),
            },
            Quantization {
                id: "Q8_0".to_string(),
                name: "Q8_0".to_string(),
                bits: 8,
                description: "High quality, larger file".to_string(),
            },
            Quantization {
                id: "F16".to_string(),
                name: "F16".to_string(),
                bits: 16,
                description: "Full precision, largest file".to_string(),
            },
        ]
    }
}

pub struct ModelManager {
    models_dir: PathBuf,
    current_model: Option<String>,
}

impl ModelManager {
    pub fn new(models_dir: PathBuf) -> Self {
        if !models_dir.exists() {
            let _ = fs::create_dir_all(&models_dir);
        }

        ModelManager {
            models_dir,
            current_model: None,
        }
    }

    pub fn list_models(&self) -> Result<Vec<ModelInfo>, ModelError> {
        let mut models = Vec::new();

        if !self.models_dir.exists() {
            return Ok(models);
        }

        let entries =
            fs::read_dir(&self.models_dir).map_err(|e| ModelError::IoError(e.to_string()))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "gguf").unwrap_or(false) {
                let metadata = fs::metadata(&path).ok();
                let size = metadata.map(|m| m.len()).unwrap_or(0);

                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let quantization = Self::detect_quantization(&name);
                let id = format!("model-{}", name.to_lowercase().replace(' ', "-"));

                models.push(ModelInfo {
                    id,
                    name: name.clone(),
                    path: path.to_string_lossy().to_string(),
                    size_bytes: size,
                    quantization,
                    context_size: 16384,
                    is_loaded: self
                        .current_model
                        .as_ref()
                        .map(|p| path.to_string_lossy() == *p)
                        .unwrap_or(false),
                });
            }
        }

        info!("Found {} models in {:?}", models.len(), self.models_dir);
        Ok(models)
    }

    pub fn get_model(&self, id: &str) -> Result<ModelInfo, ModelError> {
        let models = self.list_models()?;

        models
            .into_iter()
            .find(|m| m.id == id)
            .ok_or_else(|| ModelError::NotFound(id.to_string()))
    }

    pub fn select_model(&mut self, id: &str) -> Result<String, ModelError> {
        let model = self.get_model(id)?;
        self.current_model = Some(model.path.clone());
        info!("Selected model: {}", model.name);
        Ok(model.path)
    }

    pub fn delete_model(&self, id: &str) -> Result<(), ModelError> {
        let model = self.get_model(id)?;
        let path = PathBuf::from(&model.path);

        if path.exists() {
            fs::remove_file(&path).map_err(|e| ModelError::IoError(e.to_string()))?;
            info!("Deleted model: {}", model.name);
        }

        Ok(())
    }

    pub fn get_current_model(&self) -> Option<String> {
        self.current_model.clone()
    }

    pub fn get_models_dir(&self) -> &PathBuf {
        &self.models_dir
    }

    fn detect_quantization(name: &str) -> Option<String> {
        let quantizations = Quantization::available();
        quantizations
            .into_iter()
            .find(|q| name.to_uppercase().contains(&q.id))
            .map(|q| q.id)
    }
}

pub fn get_default_models_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sl-studio")
        .join("models")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantization_available() {
        let quants = Quantization::available();
        assert!(!quants.is_empty());
        assert!(quants.iter().any(|q| q.id == "Q4_K_M"));
    }

    #[test]
    fn test_detect_quantization() {
        assert_eq!(
            Some("Q4_K_M".to_string()),
            detect_quantization_test("model-Q4_K_M.gguf")
        );
        assert_eq!(
            Some("Q8_0".to_string()),
            detect_quantization_test("model-Q8_0.gguf")
        );
        assert_eq!(None, detect_quantization_test("model.gguf"));
    }

    fn detect_quantization_test(name: &str) -> Option<String> {
        ModelManager::detect_quantization(name)
    }
}
