use serde::{Deserialize, Serialize};

use crate::gpu::GpuBackend;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub version: String,
    pub project: ProjectConfig,
    pub model: ModelConfig,
    pub hardware: HardwareConfig,
    pub processing: ProcessingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub evidence_root: String,
    pub registry_db: String,
    pub intelligence_db: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub source: ModelSource,
    pub id: String,
    pub quantization: String,
    pub context_length: u32,
    pub downloaded: bool,
    pub local_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ModelSource {
    #[default]
    #[serde(rename = "huggingface")]
    HuggingFace,
    #[serde(rename = "local")]
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareConfig {
    pub gpu_backend: GpuBackend,
    pub gpu_memory_fraction: f32,
    pub cpu_workers: u32,
    pub ocr_provider: String,
    pub whisper_size: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    pub batch_size: u32,
    pub max_image_resolution: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        let app_dir = dirs::data_dir().unwrap_or_default().join("slstudio");

        AppConfig {
            version: "0.1.0".to_string(),
            project: ProjectConfig {
                name: "New Investigation".to_string(),
                evidence_root: String::new(),
                registry_db: app_dir.join("registry.db").to_string_lossy().to_string(),
                intelligence_db: app_dir
                    .join("intelligence.db")
                    .to_string_lossy()
                    .to_string(),
            },
            model: ModelConfig {
                source: ModelSource::HuggingFace,
                id: "Qwen/Qwen2.5-7B-Instruct-AWQ".to_string(),
                quantization: "awq".to_string(),
                context_length: 16384,
                downloaded: false,
                local_path: app_dir.join("models").to_string_lossy().to_string(),
            },
            hardware: HardwareConfig {
                gpu_backend: GpuBackend::Cpu,
                gpu_memory_fraction: 0.45,
                cpu_workers: num_cpus::get() as u32,
                ocr_provider: "onnx".to_string(),
                whisper_size: "base".to_string(),
            },
            processing: ProcessingConfig {
                batch_size: 24,
                max_image_resolution: 2048,
            },
        }
    }
}

impl AppConfig {
    pub fn config_path() -> std::path::PathBuf {
        dirs::data_dir()
            .unwrap_or_default()
            .join("slstudio")
            .join("config.json")
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: AppConfig = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn validate(&self) -> ValidationResult {
        let mut errors = Vec::new();

        if self.project.evidence_root.is_empty() {
            errors.push("Evidence root path is required".to_string());
        } else if !std::path::Path::new(&self.project.evidence_root).exists() {
            errors.push(format!(
                "Evidence root path does not exist: {}",
                self.project.evidence_root
            ));
        }

        if self.project.registry_db.is_empty() {
            errors.push("Registry database path is required".to_string());
        }

        if self.project.intelligence_db.is_empty() {
            errors.push("Intelligence database path is required".to_string());
        }

        ValidationResult {
            valid: errors.is_empty(),
            errors,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
}
