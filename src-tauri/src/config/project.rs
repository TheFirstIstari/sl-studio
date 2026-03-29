use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub investigator: InvestigatorInfo,
    pub paths: ProjectPaths,
    pub model: ProjectModel,
    pub hardware: ProjectHardware,
    pub processing: ProjectProcessing,
    pub metadata: InvestigationMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigatorInfo {
    pub name: String,
    pub agency: String,
    pub case_number: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectPaths {
    pub evidence_root: String,
    pub registry_db: String,
    pub intelligence_db: String,
    pub export_dir: String,
    pub models_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectModel {
    pub source: String,
    pub model_id: String,
    pub quantization: String,
    pub context_length: u32,
    pub local_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectHardware {
    pub gpu_backend: String,
    pub gpu_memory_fraction: f32,
    pub cpu_workers: u32,
    pub ocr_provider: String,
    pub whisper_size: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectProcessing {
    pub batch_size: u32,
    pub max_image_resolution: u32,
    pub enable_ocr: bool,
    pub enable_audio: bool,
    pub enable_pdf_extraction: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationMetadata {
    pub total_files: u64,
    pub processed_files: u64,
    pub facts_extracted: u64,
    pub last_scan_date: Option<DateTime<Utc>>,
    pub last_analysis_date: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
}

impl Default for ProjectFile {
    fn default() -> Self {
        let app_dir = dirs::data_dir().unwrap_or_default().join("slstudio");

        let models_dir = std::path::Path::new(".")
            .join("models")
            .to_string_lossy()
            .to_string();

        ProjectFile {
            version: "1.0.0".to_string(),
            created_at: Utc::now(),
            modified_at: Utc::now(),
            investigator: InvestigatorInfo {
                name: String::new(),
                agency: String::new(),
                case_number: String::new(),
                notes: String::new(),
            },
            paths: ProjectPaths {
                evidence_root: String::new(),
                registry_db: app_dir.join("registry.db").to_string_lossy().to_string(),
                intelligence_db: app_dir
                    .join("intelligence.db")
                    .to_string_lossy()
                    .to_string(),
                export_dir: app_dir.join("exports").to_string_lossy().to_string(),
                models_dir,
            },
            model: ProjectModel {
                source: "huggingface".to_string(),
                model_id: "Qwen/Qwen2.5-7B-Instruct-GGUF".to_string(),
                quantization: "Q4_K_M".to_string(),
                context_length: 16384,
                local_path: String::new(),
            },
            hardware: ProjectHardware {
                gpu_backend: "cpu".to_string(),
                gpu_memory_fraction: 0.45,
                cpu_workers: num_cpus::get() as u32,
                ocr_provider: "onnx".to_string(),
                whisper_size: "base".to_string(),
            },
            processing: ProjectProcessing {
                batch_size: 24,
                max_image_resolution: 2048,
                enable_ocr: true,
                enable_audio: true,
                enable_pdf_extraction: true,
            },
            metadata: InvestigationMetadata {
                total_files: 0,
                processed_files: 0,
                facts_extracted: 0,
                last_scan_date: None,
                last_analysis_date: None,
                tags: Vec::new(),
            },
        }
    }
}

impl ProjectFile {
    pub fn load(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let project: ProjectFile = serde_json::from_str(&content)?;
        Ok(project)
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn update_modified(&mut self) {
        self.modified_at = Utc::now();
    }
}
