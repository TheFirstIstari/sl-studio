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
    pub auto_scale_workers: bool,
    pub batch_size: u32,
    pub auto_scale_batch: bool,
    pub ocr_provider: String,
    pub whisper_size: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub cpu_threads: u32,
    pub total_memory_gb: f64,
    pub available_memory_gb: f64,
    pub recommended_workers: u32,
    pub recommended_batch_size: u32,
}

impl Default for HardwareInfo {
    fn default() -> Self {
        use sysinfo::System;
        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu_threads = sys.cpus().len() as u32;
        let total_memory_gb = sys.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
        let available_memory_gb = sys.available_memory() as f64 / (1024.0 * 1024.0 * 1024.0);

        let recommended_workers = (cpu_threads as i32 - 2).max(1) as u32;
        let recommended_batch_size = if total_memory_gb > 12.0 {
            10
        } else if total_memory_gb > 6.0 {
            6
        } else {
            4
        };

        HardwareInfo {
            cpu_threads,
            total_memory_gb,
            available_memory_gb,
            recommended_workers,
            recommended_batch_size,
        }
    }
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
            version: "0.2.0".to_string(),
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
                context_length: 8192,
                downloaded: false,
                local_path: app_dir.join("models").to_string_lossy().to_string(),
            },
            hardware: HardwareConfig {
                gpu_backend: GpuBackend::Cpu,
                gpu_memory_fraction: 0.40,
                cpu_workers: (num_cpus::get() as i32 - 2).max(1) as u32,
                auto_scale_workers: true,
                batch_size: 6,
                auto_scale_batch: true,
                ocr_provider: "onnx".to_string(),
                whisper_size: "base".to_string(),
            },
            processing: ProcessingConfig {
                batch_size: 6,
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
            let mut config: AppConfig = serde_json::from_str(&content)?;
            config.apply_auto_scaling();
            Ok(config)
        } else {
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn get_hardware_info(&self) -> HardwareInfo {
        HardwareInfo::default()
    }

    pub fn get_effective_workers(&self) -> u32 {
        if self.hardware.auto_scale_workers {
            self.get_hardware_info().recommended_workers
        } else {
            self.hardware.cpu_workers
        }
    }

    pub fn get_effective_batch_size(&self) -> u32 {
        if self.hardware.auto_scale_batch {
            self.get_hardware_info().recommended_batch_size
        } else {
            self.hardware.batch_size
        }
    }

    fn apply_auto_scaling(&mut self) {
        let info = self.get_hardware_info();
        if self.hardware.auto_scale_workers {
            self.hardware.cpu_workers = info.recommended_workers;
        }
        if self.hardware.auto_scale_batch {
            self.hardware.batch_size = info.recommended_batch_size;
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
