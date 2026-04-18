pub mod config;
pub mod core;
pub mod extractors;
pub mod gpu;
pub mod inference;
pub mod models;
pub mod utils;

use config::{AppConfig, ProjectFile, ValidationResult};
use core::{Database, IntelligenceEntry, RegistryProgress, RegistryWorker};
use gpu::HardwareStatus;
use inference::{Reasoner, ReasonerConfig};
pub use models::{ModelManager, Quantization};

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, State};

static GLOBAL_THREAD_POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();

fn get_or_create_thread_pool(workers: usize) -> &'static rayon::ThreadPool {
    GLOBAL_THREAD_POOL.get_or_init(|| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(workers)
            .build()
            .expect("Failed to create global thread pool")
    })
}
use tracing::{error, info};

#[cfg(feature = "custom-protocol")]
const IS_DEV: bool = true;

#[cfg(not(feature = "custom-protocol"))]
const IS_DEV: bool = false;

pub struct AppState {
    config: Mutex<AppConfig>,
    db: Mutex<Option<Database>>,
    registry_worker: Mutex<Option<RegistryWorker>>,
    reasoner: Mutex<Option<Arc<Reasoner>>>,
    cancel_flag: AtomicBool,
}

impl Default for AppState {
    fn default() -> Self {
        let config = match AppConfig::load() {
            Ok(cfg) => cfg,
            Err(e) => {
                error!("Failed to load config, using defaults: {}", e);
                AppConfig::default()
            }
        };
        AppState {
            config: Mutex::new(config),
            db: Mutex::new(None),
            registry_worker: Mutex::new(None),
            reasoner: Mutex::new(None),
            cancel_flag: AtomicBool::new(false),
        }
    }
}

// Response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub registry_count: i64,
    pub intelligence_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionStats {
    pub total_files: i64,
    pub total_characters: i64,
    pub average_characters: f64,
    pub average_quality: f64,
    pub partial_count: i64,
    pub files_by_type: std::collections::HashMap<String, i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    pub files_scanned: i64,
    pub files_extracted: i64,
    pub files_analyzed: i64,
    pub last_scan_time: Option<String>,
    pub last_extraction_time: Option<String>,
    pub last_analysis_time: Option<String>,
    pub current_stage: String,
}

// Commands
#[tauri::command]
fn load_config(state: State<AppState>) -> Result<AppConfig, String> {
    let guard = state
        .config
        .lock()
        .map_err(|e| format!("Failed to lock config: {}", e))?;
    let config = guard.clone();
    info!("Config loaded");
    Ok(config)
}

#[tauri::command]
fn save_config(state: State<AppState>, config: AppConfig) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())?;
    let mut guard = state
        .config
        .lock()
        .map_err(|e| format!("Failed to lock config: {}", e))?;
    *guard = config;
    info!("Config saved");
    Ok(())
}

#[tauri::command]
fn validate_config(config: AppConfig) -> ValidationResult {
    config.validate()
}

#[tauri::command]
fn detect_hardware() -> HardwareStatus {
    gpu::detect()
}

#[tauri::command]
fn get_hardware_info(state: State<AppState>) -> config::HardwareInfo {
    state
        .config
        .lock()
        .map(|g| g.get_hardware_info())
        .unwrap_or_default()
}

#[tauri::command]
fn get_recommended_settings() -> config::HardwareInfo {
    config::HardwareInfo::default()
}

#[derive(Serialize, Clone)]
pub struct SystemMonitor {
    pub cpu_usage_percent: f32,
    pub memory_used_gb: f64,
    pub memory_available_gb: f64,
    pub memory_percent: f32,
    pub gpu_usage_percent: Option<f32>,
    pub gpu_memory_used_mb: Option<u64>,
}

#[tauri::command]
fn get_system_monitor() -> SystemMonitor {
    use sysinfo::System;

    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_usage = sys.global_cpu_usage();
    let memory_used = sys.used_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
    let memory_total = sys.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
    let memory_available = sys.available_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
    let memory_percent = if memory_total > 0.0 {
        (memory_used / memory_total * 100.0) as f32
    } else {
        0.0
    };

    let gpu_status = gpu::detect();
    let gpu_usage = None;
    let gpu_memory = gpu_status.gpu_info.first().map(|g| g.vram_mb);

    SystemMonitor {
        cpu_usage_percent: cpu_usage,
        memory_used_gb: memory_used,
        memory_available_gb: memory_available,
        memory_percent,
        gpu_usage_percent: gpu_usage,
        gpu_memory_used_mb: gpu_memory,
    }
}

#[derive(Serialize, Clone)]
pub struct ProcessingStats {
    pub files_processed: i64,
    pub files_pending: i64,
    pub total_files: i64,
    pub processing_rate: f64,
}

#[tauri::command]
fn get_processing_stats(state: State<AppState>) -> Result<ProcessingStats, String> {
    let db_guard = state
        .db
        .lock()
        .map_err(|e| format!("Failed to lock database: {}", e))?;
    if let Some(db) = db_guard.as_ref() {
        let stats = db.get_overall_statistics().map_err(|e| e.to_string())?;

        Ok(ProcessingStats {
            files_processed: stats.total_facts,
            files_pending: 0,
            total_files: stats.total_facts,
            processing_rate: 0.0,
        })
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
async fn init_project(
    app: AppHandle,
    state: State<'_, AppState>,
    config: AppConfig,
) -> Result<bool, String> {
    info!("Initializing project: {}", config.project.name);

    // Ensure app directories exist
    utils::ensure_app_dirs().map_err(|e| e.to_string())?;

    // Initialize database
    let db = Database::new(&config.project.registry_db, &config.project.intelligence_db)
        .map_err(|e| e.to_string())?;

    {
        let mut db_guard = state
            .db
            .lock()
            .map_err(|e| format!("Failed to lock database: {}", e))?;
        *db_guard = Some(db);
    }

    // Create registry worker
    let worker = RegistryWorker::new(
        &config.project.evidence_root,
        &config.project.registry_db,
        &config.project.intelligence_db,
    )
    .map_err(|e| e.to_string())?;

    {
        let mut worker_guard = state
            .registry_worker
            .lock()
            .map_err(|e| format!("Failed to lock worker: {}", e))?;
        *worker_guard = Some(worker);
    }

    // Save config
    config.save().map_err(|e| e.to_string())?;
    {
        let mut config_guard = state
            .config
            .lock()
            .map_err(|e| format!("Failed to lock config: {}", e))?;
        *config_guard = config;
    }

    info!("Project initialized successfully");
    app.emit("project_initialized", true).ok();

    Ok(true)
}

#[tauri::command]
async fn start_registry(app: AppHandle, state: State<'_, AppState>) -> Result<usize, String> {
    let (evidence_root, registry_db, intelligence_db) = {
        let config_guard = state
            .config
            .lock()
            .map_err(|e| format!("Failed to lock config: {}", e))?;
        (
            config_guard.project.evidence_root.clone(),
            config_guard.project.registry_db.clone(),
            config_guard.project.intelligence_db.clone(),
        )
    };

    if evidence_root.is_empty() {
        return Err("Evidence root not set".to_string());
    }

    let mut worker = RegistryWorker::new(&evidence_root, &registry_db, &intelligence_db)
        .map_err(|e| e.to_string())?;

    let (tx, rx) = std::sync::mpsc::channel::<RegistryProgress>();

    let app_clone = app.clone();
    std::thread::spawn(move || {
        for progress in rx {
            app_clone.emit("registry_progress", progress).ok();
        }
    });

    let result = worker.scan(tx).map_err(|e| e.to_string())?;

    info!("Registry scan complete: {} files", result);
    app.emit("registry_complete", result).ok();
    Ok(result)
}

#[tauri::command]
fn get_stats(state: State<AppState>) -> Result<Stats, String> {
    let db_guard = state
        .db
        .lock()
        .map_err(|e| format!("Failed to lock database: {}", e))?;
    if let Some(db) = db_guard.as_ref() {
        Ok(Stats {
            registry_count: db.get_registry_count().unwrap_or(0),
            intelligence_count: db.get_intelligence_count().unwrap_or(0),
        })
    } else {
        Ok(Stats {
            registry_count: 0,
            intelligence_count: 0,
        })
    }
}

#[tauri::command]
fn get_workflow_state(state: State<AppState>) -> Result<WorkflowState, String> {
    let db_guard = state
        .db
        .lock()
        .map_err(|e| format!("Failed to lock database: {}", e))?;
    if let Some(db) = db_guard.as_ref() {
        db.get_workflow_state().map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn get_extraction_statistics(state: State<AppState>) -> Result<ExtractionStats, String> {
    let db_guard = state
        .db
        .lock()
        .map_err(|e| format!("Failed to lock database: {}", e))?;
    if let Some(db) = db_guard.as_ref() {
        let stats = db.get_extraction_statistics().map_err(|e| e.to_string())?;
        Ok(ExtractionStats {
            total_files: stats.total_files,
            total_characters: stats.total_characters,
            average_characters: stats.average_characters,
            average_quality: stats.average_quality,
            partial_count: stats.partial_count,
            files_by_type: stats.files_by_type,
        })
    } else {
        Ok(ExtractionStats {
            total_files: 0,
            total_characters: 0,
            average_characters: 0.0,
            average_quality: 0.0,
            partial_count: 0,
            files_by_type: std::collections::HashMap::new(),
        })
    }
}

#[tauri::command]
fn get_unprocessed_files(
    state: State<AppState>,
    limit: i64,
) -> Result<Vec<core::RegistryEntry>, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.get_unprocessed_files(limit).map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn mark_processed(state: State<AppState>, fingerprint: String) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.mark_processed(&fingerprint).map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn get_app_data_dir() -> String {
    utils::app_data_dir().to_string_lossy().to_string()
}

#[tauri::command]
fn search_facts(
    state: State<AppState>,
    query: String,
    limit: i64,
) -> Result<Vec<core::SearchResult>, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.search_facts(&query, limit).map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn search_entities(
    state: State<AppState>,
    query: String,
    limit: i64,
) -> Result<Vec<core::EntitySearchResult>, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.search_entities(&query, limit).map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn search_combined(
    state: State<AppState>,
    query: String,
    limit: i64,
) -> Result<Vec<core::CombinedSearchResult>, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.search_combined(&query, limit).map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn get_timeline_events(
    state: State<AppState>,
    start_date: Option<String>,
    end_date: Option<String>,
    limit: i64,
) -> Result<Vec<core::TimelineEvent>, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.get_timeline_events(start_date.as_deref(), end_date.as_deref(), limit)
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn get_overall_statistics(state: State<AppState>) -> Result<core::OverallStatistics, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.get_overall_statistics().map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn get_category_distribution(state: State<AppState>) -> Result<Vec<core::CategoryStats>, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.get_category_distribution().map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn get_severity_distribution(state: State<AppState>) -> Result<Vec<core::SeverityStats>, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.get_severity_distribution().map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn get_entity_centrality(
    state: State<AppState>,
    entity_type: Option<String>,
    min_confidence: f64,
) -> Result<Vec<core::EntityCentrality>, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.get_entity_centrality(entity_type.as_deref(), min_confidence)
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn detect_anomalies(
    state: State<AppState>,
    metric: String,
    threshold_std: f64,
) -> Result<Vec<core::Anomaly>, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.detect_anomalies(&metric, threshold_std)
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn get_weighted_evidence(
    state: State<AppState>,
    min_weight: f64,
    limit: i64,
) -> Result<Vec<core::WeightedEvidence>, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.get_weighted_evidence(min_weight, limit)
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn get_entity_relationships(
    state: State<AppState>,
    entity_id: Option<i64>,
    min_confidence: f64,
) -> Result<Vec<core::EntityRelationship>, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.get_entity_relationships(entity_id, min_confidence)
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn get_connected_entities(
    state: State<AppState>,
    entity_id: i64,
    min_confidence: f64,
) -> Result<Vec<core::ConnectedEntity>, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.get_connected_entities(entity_id, 1, min_confidence)
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn add_tag(state: State<AppState>, intelligence_id: i64, tag: String) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.add_tag(intelligence_id, &tag).map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn remove_tag(state: State<AppState>, intelligence_id: i64, tag: String) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.remove_tag(intelligence_id, &tag)
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn get_all_tags(state: State<AppState>) -> Result<Vec<String>, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.get_all_tags().map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn add_annotation(
    state: State<AppState>,
    intelligence_id: i64,
    content: String,
    annotation_type: String,
) -> Result<i64, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.add_annotation(intelligence_id, &content, &annotation_type)
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn update_annotation(
    state: State<AppState>,
    annotation_id: i64,
    content: String,
) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.update_annotation(annotation_id, &content)
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn delete_annotation(state: State<AppState>, annotation_id: i64) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.delete_annotation(annotation_id)
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn get_annotations(
    state: State<AppState>,
    intelligence_id: i64,
) -> Result<Vec<core::Annotation>, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.get_annotations(intelligence_id)
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn search_by_tags(
    state: State<AppState>,
    tags: Vec<String>,
    match_all: bool,
    limit: i64,
) -> Result<Vec<core::SearchResult>, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.search_by_tags(&tags, match_all, limit)
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn get_location_entities(
    state: State<AppState>,
    min_confidence: f64,
) -> Result<Vec<core::LocationEntity>, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.get_location_entities(min_confidence)
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn export_facts_json(
    state: State<AppState>,
    min_weight: f64,
    limit: i64,
    categories: Option<Vec<String>>,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<String, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        let filters = core::ExportFilters {
            min_weight,
            limit,
            categories,
            start_date,
            end_date,
        };
        db.export_facts_json(&filters).map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn export_entities_csv(
    state: State<AppState>,
    entity_type: Option<String>,
    min_confidence: f64,
) -> Result<String, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.export_entities_csv(entity_type.as_deref(), min_confidence)
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn export_timeline_json(
    state: State<AppState>,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<String, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        db.export_timeline_json(start_date.as_deref(), end_date.as_deref())
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportReport {
    pub facts: Vec<core::WeightedEvidence>,
    pub statistics: core::OverallStatistics,
    pub categories: Vec<core::CategoryStats>,
}

#[tauri::command]
fn export_full_report_json(state: State<AppState>) -> Result<String, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        let facts = db
            .get_weighted_evidence(0.0, 10000)
            .map_err(|e| e.to_string())?;
        let statistics = db.get_overall_statistics().map_err(|e| e.to_string())?;
        let categories = db.get_category_distribution().map_err(|e| e.to_string())?;

        let report = ExportReport {
            facts,
            statistics,
            categories,
        };
        serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn export_facts_csv(state: State<AppState>, min_weight: f64, limit: i64) -> Result<String, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
        let facts = db
            .get_weighted_evidence(min_weight, limit)
            .map_err(|e| e.to_string())?;

        let mut csv = String::from("id,fingerprint,filename,category,severity,confidence,quality,weight,summary,created_at\n");
        for f in facts {
            csv.push_str(&format!(
                "{},{},\"{}\",\"{}\",{},{},{},{},\"{}\",\"{}\"\n",
                f.id,
                f.fingerprint,
                f.filename.replace('"', "\"\""),
                f.category.unwrap_or_default(),
                f.severity,
                f.confidence.unwrap_or(0.0),
                f.quality.unwrap_or(0.0),
                f.weight,
                f.summary.replace('"', "\"\""),
                f.created_at.unwrap_or_default()
            ));
        }
        Ok(csv)
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn write_file(path: String, contents: Vec<u8>) -> Result<(), String> {
    std::fs::write(&path, contents).map_err(|e| e.to_string())?;
    info!("Wrote file: {}", path);
    Ok(())
}

#[tauri::command]
fn export_pdf_report(state: State<AppState>) -> Result<Vec<u8>, String> {
    use printpdf::*;
    use std::io::BufWriter;

    let db = state.db.lock().unwrap();
    let db_ref = db.as_ref().ok_or("Database not initialized")?;

    let facts = db_ref
        .get_weighted_evidence(0.0, 100)
        .map_err(|e| e.to_string())?;
    let stats = db_ref.get_overall_statistics().map_err(|e| e.to_string())?;
    let categories = db_ref
        .get_category_distribution()
        .map_err(|e| e.to_string())?;

    let (doc, page1, layer1) =
        PdfDocument::new("SL Studio Forensic Report", Mm(210.0), Mm(297.0), "Layer 1");

    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| e.to_string())?;
    let font_bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| e.to_string())?;

    let current_layer = doc.get_page(page1).get_layer(layer1);

    current_layer.use_text(
        "SL Studio - Forensic Document Analysis Report",
        24.0,
        Mm(20.0),
        Mm(277.0),
        &font_bold,
    );
    current_layer.use_text(
        format!(
            "Generated: {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        ),
        10.0,
        Mm(20.0),
        Mm(268.0),
        &font,
    );

    current_layer.use_text("Summary Statistics", 16.0, Mm(20.0), Mm(250.0), &font_bold);
    current_layer.use_text(
        format!("Total Facts: {}", stats.total_facts),
        12.0,
        Mm(25.0),
        Mm(240.0),
        &font,
    );
    current_layer.use_text(
        format!("Total Entities: {}", stats.total_entities),
        12.0,
        Mm(25.0),
        Mm(232.0),
        &font,
    );
    current_layer.use_text(
        format!("Unique Entities: {}", stats.unique_entities),
        12.0,
        Mm(25.0),
        Mm(224.0),
        &font,
    );
    current_layer.use_text(
        format!("Total Chains: {}", stats.total_chains),
        12.0,
        Mm(25.0),
        Mm(216.0),
        &font,
    );

    current_layer.use_text(
        "Category Distribution",
        16.0,
        Mm(20.0),
        Mm(198.0),
        &font_bold,
    );
    let mut y_pos = 188.0;
    for cat in categories.iter().take(10) {
        current_layer.use_text(
            format!("{}: {} items", cat.category, cat.count),
            11.0,
            Mm(25.0),
            Mm(y_pos),
            &font,
        );
        y_pos -= 7.0;
    }

    current_layer.use_text("Top Facts", 16.0, Mm(20.0), Mm(y_pos - 15.0), &font_bold);
    y_pos -= 25.0;
    for (i, fact) in facts.iter().take(15).enumerate() {
        if y_pos < 30.0 {
            break;
        }
        let summary = if fact.summary.len() > 60 {
            format!("{}...", &fact.summary[..60])
        } else {
            fact.summary.clone()
        };
        current_layer.use_text(
            format!(
                "{}. [{}] {}",
                i + 1,
                fact.category.as_deref().unwrap_or("N/A"),
                summary
            ),
            9.0,
            Mm(25.0),
            Mm(y_pos),
            &font,
        );
        y_pos -= 6.0;
    }

    let mut buffer = BufWriter::new(Vec::new());
    doc.save(&mut buffer).map_err(|e| e.to_string())?;
    let pdf_bytes = buffer.into_inner().map_err(|e| e.to_string())?;

    Ok(pdf_bytes)
}

#[derive(Serialize)]
struct ExcelData {
    facts: Vec<core::WeightedEvidence>,
    categories: Vec<core::CategoryStats>,
    entities: Vec<core::EntityCentrality>,
    timeline: Vec<core::TimelineEvent>,
}

#[tauri::command]
fn export_excel_data(state: State<AppState>) -> Result<String, String> {
    let db = state.db.lock().unwrap();
    let db_ref = db.as_ref().ok_or("Database not initialized")?;

    let facts = db_ref
        .get_weighted_evidence(0.0, 1000)
        .map_err(|e| e.to_string())?;
    let categories = db_ref
        .get_category_distribution()
        .map_err(|e| e.to_string())?;
    let entities = db_ref
        .get_entity_centrality(None, 0.0)
        .map_err(|e| e.to_string())?;
    let timeline = db_ref
        .get_timeline_events(None, None, 1000)
        .map_err(|e| e.to_string())?;

    let data = ExcelData {
        facts,
        categories,
        entities,
        timeline,
    };

    serde_json::to_string_pretty(&data).map_err(|e| e.to_string())
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ProjectComparison {
    pub project1_name: String,
    pub project2_name: String,
    pub entity_overlap: Vec<EntityOverlap>,
    pub common_entities: Vec<core::EntityCentrality>,
    pub timeline_correlation: TimelineCorrelation,
    pub fact_similarity: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EntityOverlap {
    pub entity_value: String,
    pub entity_type: String,
    pub count_project1: i32,
    pub count_project2: i32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TimelineCorrelation {
    pub correlation_score: f64,
    pub aligned_events: i32,
    pub project1_date_range: (String, String),
    pub project2_date_range: (String, String),
}

fn open_project_db(path: &str) -> Result<Database, String> {
    let db_path = std::path::Path::new(path);
    if !db_path.exists() {
        return Err(format!("Database file not found: {}", path));
    }

    let registry_db = db_path.join("registry.db");
    let intelligence_db = db_path.join("intelligence.db");

    if !registry_db.exists() || !intelligence_db.exists() {
        return Err("Invalid project directory - missing database files".to_string());
    }

    Database::new(
        registry_db.to_string_lossy().as_ref(),
        intelligence_db.to_string_lossy().as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn compare_projects(
    state: State<AppState>,
    project2_path: String,
) -> Result<ProjectComparison, String> {
    // Extract data from state - need to hold lock during extraction
    let (entities1, timeline1, project1_name) = {
        let db_guard = state.db.lock().unwrap();
        let db = db_guard.as_ref().ok_or("Database not initialized")?;

        let entities1 = db
            .get_entity_centrality(None, 0.0)
            .map_err(|e| e.to_string())?;
        let timeline1 = db
            .get_timeline_events(None, None, 1000)
            .map_err(|e| e.to_string())?;

        let config = state.config.lock().unwrap();
        let project1_name = config.project.name.clone();

        (entities1, timeline1, project1_name)
    };

    // Now db2 goes out of scope and is closed when this block ends
    {
        let db2 = open_project_db(&project2_path)?;

        let entities2 = db2
            .get_entity_centrality(None, 0.0)
            .map_err(|e| e.to_string())?;
        let timeline2 = db2
            .get_timeline_events(None, None, 1000)
            .map_err(|e| e.to_string())?;

        // Calculate entity overlap
        let mut entity_overlap = Vec::new();
        let mut common_entities = Vec::new();

        let mut entity_map1: std::collections::HashMap<String, i32> =
            std::collections::HashMap::new();
        for e in &entities1 {
            *entity_map1.entry(e.value.clone()).or_insert(0) += e.occurrence_count;
        }

        let mut entity_map2: std::collections::HashMap<String, i32> =
            std::collections::HashMap::new();
        for e in &entities2 {
            *entity_map2.entry(e.value.clone()).or_insert(0) += e.occurrence_count;
        }

        for (value, count1) in &entity_map1 {
            if let Some(&count2) = entity_map2.get(value) {
                let entity_type = entities1
                    .iter()
                    .find(|e| &e.value == value)
                    .map(|e| e.entity_type.clone())
                    .unwrap_or_default();

                entity_overlap.push(EntityOverlap {
                    entity_value: value.clone(),
                    entity_type,
                    count_project1: *count1,
                    count_project2: count2,
                });

                if let Some(e1) = entities1.iter().find(|e| &e.value == value) {
                    common_entities.push(e1.clone());
                }
            }
        }

        // Calculate timeline correlation
        let mut dates1: std::collections::HashSet<String> = std::collections::HashSet::new();
        for e in &timeline1 {
            dates1.insert(e.date.clone());
        }

        let mut dates2: std::collections::HashSet<String> = std::collections::HashSet::new();
        for e in &timeline2 {
            dates2.insert(e.date.clone());
        }

        let intersection: std::collections::HashSet<_> = dates1.intersection(&dates2).collect();
        let union: std::collections::HashSet<_> = dates1.union(&dates2).collect();

        let correlation_score = if union.is_empty() {
            0.0
        } else {
            intersection.len() as f64 / union.len() as f64
        };

        let timeline_correlation = TimelineCorrelation {
            correlation_score,
            aligned_events: intersection.len() as i32,
            project1_date_range: (
                timeline1
                    .first()
                    .map(|e| e.date.clone())
                    .unwrap_or_default(),
                timeline1.last().map(|e| e.date.clone()).unwrap_or_default(),
            ),
            project2_date_range: (
                timeline2
                    .first()
                    .map(|e| e.date.clone())
                    .unwrap_or_default(),
                timeline2.last().map(|e| e.date.clone()).unwrap_or_default(),
            ),
        };

        let fact_similarity = if common_entities.is_empty() {
            0.0
        } else {
            let total_entities = entity_map1.len() + entity_map2.len();
            if total_entities == 0 {
                0.0
            } else {
                2.0 * common_entities.len() as f64 / total_entities as f64
            }
        };

        Ok(ProjectComparison {
            project1_name,
            project2_name: std::path::Path::new(&project2_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Project 2".to_string()),
            entity_overlap,
            common_entities,
            timeline_correlation,
            fact_similarity,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct ProjectSummary {
    pub name: String,
    pub path: String,
    pub fact_count: i64,
    pub entity_count: i64,
    pub timeline_count: i64,
}

#[tauri::command]
fn get_project_summary(state: State<AppState>) -> Result<ProjectSummary, String> {
    let db = state.db.lock().unwrap();
    let db_ref = db.as_ref().ok_or("Database not initialized")?;

    let stats = db_ref.get_overall_statistics().map_err(|e| e.to_string())?;
    let timeline = db_ref
        .get_timeline_events(None, None, 1000)
        .map_err(|e| e.to_string())?;

    let config = state.config.lock().unwrap();

    let project_path = std::path::Path::new(&config.project.registry_db)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    Ok(ProjectSummary {
        name: config.project.name.clone(),
        path: project_path,
        fact_count: stats.total_facts,
        entity_count: stats.total_entities,
        timeline_count: timeline.len() as i64,
    })
}

#[derive(Serialize, Clone)]
pub struct BackupInfo {
    pub backup_path: String,
    pub size_bytes: u64,
    pub created_at: String,
    pub includes_evidence: bool,
}

#[tauri::command]
fn create_backup(state: State<AppState>, include_evidence: bool) -> Result<BackupInfo, String> {
    use std::io::Write;

    let config = state.config.lock().unwrap();
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let backup_name = format!("slstudio_backup_{}.zip", timestamp);

    let export_dir = dirs::data_dir()
        .unwrap_or_default()
        .join("slstudio")
        .join("backups");

    if !export_dir.exists() {
        std::fs::create_dir_all(&export_dir).map_err(|e| e.to_string())?;
    }

    let backup_path = export_dir.join(&backup_name);
    let file = std::fs::File::create(&backup_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let registry_db = std::path::Path::new(&config.project.registry_db);
    let intelligence_db = std::path::Path::new(&config.project.intelligence_db);

    if registry_db.exists() {
        let data = std::fs::read(registry_db).map_err(|e| e.to_string())?;
        zip.start_file("registry.db", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(&data).map_err(|e| e.to_string())?;
    }

    if intelligence_db.exists() {
        let data = std::fs::read(intelligence_db).map_err(|e| e.to_string())?;
        zip.start_file("intelligence.db", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(&data).map_err(|e| e.to_string())?;
    }

    let config_data = serde_json::to_string_pretty(&*config).map_err(|e| e.to_string())?;
    zip.start_file("config.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(config_data.as_bytes())
        .map_err(|e| e.to_string())?;

    if include_evidence {
        let evidence_root = std::path::Path::new(&config.project.evidence_root);
        if evidence_root.exists() {
            for entry in walkdir::WalkDir::new(evidence_root)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    let path = entry.path();
                    let name = path.strip_prefix(evidence_root).unwrap().to_string_lossy();
                    let data = std::fs::read(path).map_err(|e| e.to_string())?;
                    zip.start_file(format!("evidence/{}", name), options)
                        .map_err(|e| e.to_string())?;
                    zip.write_all(&data).map_err(|e| e.to_string())?;
                }
            }
        }
    }

    zip.finish().map_err(|e| e.to_string())?;

    let metadata = std::fs::metadata(&backup_path).map_err(|e| e.to_string())?;

    info!("Backup created: {}", backup_path.display());

    Ok(BackupInfo {
        backup_path: backup_path.to_string_lossy().to_string(),
        size_bytes: metadata.len(),
        created_at: chrono::Local::now().to_rfc3339(),
        includes_evidence: include_evidence,
    })
}

#[tauri::command]
fn restore_backup(state: State<AppState>, backup_path: String) -> Result<(), String> {
    let file = std::fs::File::open(&backup_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    let (registry_db, intelligence_db) = {
        let config = state.config.lock().unwrap();
        (
            config.project.registry_db.clone(),
            config.project.intelligence_db.clone(),
        )
    };

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name().to_string();

        match name.as_str() {
            "registry.db" => {
                let path = std::path::Path::new(&registry_db);
                let mut out = std::fs::File::create(path).map_err(|e| e.to_string())?;
                std::io::copy(&mut file, &mut out).map_err(|e| e.to_string())?;
            }
            "intelligence.db" => {
                let path = std::path::Path::new(&intelligence_db);
                let mut out = std::fs::File::create(path).map_err(|e| e.to_string())?;
                std::io::copy(&mut file, &mut out).map_err(|e| e.to_string())?;
            }
            n if n.starts_with("evidence/") => {
                let rel_path = &n[9..];
                let config = state.config.lock().unwrap();
                let dest = std::path::Path::new(&config.project.evidence_root).join(rel_path);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let mut out = std::fs::File::create(dest).map_err(|e| e.to_string())?;
                std::io::copy(&mut file, &mut out).map_err(|e| e.to_string())?;
            }
            _ => {}
        }
    }

    // Reinitialize the database with restored files
    let db = Database::new(&registry_db, &intelligence_db)
        .map_err(|e| format!("Failed to reopen database: {}", e))?;
    *state.db.lock().unwrap() = Some(db);

    info!("Backup restored from: {}", backup_path);
    Ok(())
}

#[derive(Serialize, Clone)]
pub struct Notification {
    pub id: String,
    pub title: String,
    pub message: String,
    pub notification_type: String,
    pub timestamp: String,
    pub read: bool,
}

#[tauri::command]
fn send_notification(_app: AppHandle, title: String, message: String) -> Result<(), String> {
    info!("Notification: {} - {}", title, message);
    Ok(())
}

#[tauri::command]
fn get_models_dir() -> String {
    if IS_DEV {
        utils::dev_models_dir().to_string_lossy().to_string()
    } else {
        utils::models_dir().to_string_lossy().to_string()
    }
}

#[tauri::command]
fn create_project(path: String) -> Result<ProjectFile, String> {
    let project = ProjectFile::default();
    let file_path = std::path::Path::new(&path);
    project.save(file_path).map_err(|e| e.to_string())?;
    info!("Created new project: {}", path);
    Ok(project)
}

#[tauri::command]
fn load_project(path: String) -> Result<ProjectFile, String> {
    let file_path = std::path::Path::new(&path);
    ProjectFile::load(file_path).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_project(path: String, project: ProjectFile) -> Result<(), String> {
    let file_path = std::path::Path::new(&path);
    let mut proj = project;
    proj.update_modified();
    proj.save(file_path).map_err(|e| e.to_string())?;
    info!("Saved project: {}", path);
    Ok(())
}

#[tauri::command]
fn get_default_project() -> ProjectFile {
    ProjectFile::default()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub filename: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub filename: String,
    pub size: u64,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuggingFaceFile {
    #[serde(alias = "rfilename")]
    pub path: String,
    pub size: Option<u64>,
    #[serde(alias = "downloadUrl")]
    pub download_url: Option<String>,
}

#[allow(dead_code)]
fn get_huggingface_tree(repo_id: &str) -> Result<String, String> {
    let url = format!("https://huggingface.co/api/models/{}", repo_id);

    let client = reqwest::blocking::Client::builder()
        .user_agent("SL-Studio/0.2.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .map_err(|e| format!("Failed to fetch model info: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let text = response
        .text()
        .map_err(|e| format!("Failed to read response: {}", e))?;

    #[derive(Deserialize)]
    struct ModelInfo {
        sha: Option<String>,
        siblings: Option<Vec<HuggingFaceFile>>,
    }

    let info: ModelInfo = serde_json::from_str(&text).map_err(|e| {
        format!(
            "Failed to parse response: {}. Response preview: {}",
            e,
            &text[..text.len().min(300)]
        )
    })?;

    let sha = info.sha.unwrap_or_else(|| "main".to_string());
    Ok(sha)
}

fn get_huggingface_files_with_size(repo_id: &str) -> Result<Vec<HuggingFaceFile>, String> {
    // Use the regular API first
    let files = get_huggingface_files(repo_id)?;

    // Filter for GGUF files
    let gguf_files: Vec<HuggingFaceFile> = files
        .into_iter()
        .filter(|f| f.path.to_lowercase().ends_with(".gguf"))
        .collect();

    Ok(gguf_files)
}

fn get_huggingface_files(repo_id: &str) -> Result<Vec<HuggingFaceFile>, String> {
    let url = format!("https://huggingface.co/api/models/{}", repo_id);

    let client = reqwest::blocking::Client::builder()
        .user_agent("SL-Studio/0.2.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .map_err(|e| format!("Failed to fetch model info: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let text = response
        .text()
        .map_err(|e| format!("Failed to read response: {}", e))?;

    #[derive(Deserialize)]
    struct ModelInfo {
        siblings: Option<Vec<HuggingFaceFile>>,
    }

    let info: ModelInfo = serde_json::from_str(&text).map_err(|e| {
        format!(
            "Failed to parse response: {}. Response preview: {}",
            e,
            &text[..text.len().min(300)]
        )
    })?;

    info.siblings
        .ok_or_else(|| "No files found in model repository".to_string())
}

#[allow(dead_code)]
fn find_gguf_file(files: &[HuggingFaceFile]) -> Option<(String, u64)> {
    for file in files {
        if file.path.to_lowercase().ends_with(".gguf") {
            let url = file.download_url.as_ref()?;
            return Some((url.clone(), file.size.unwrap_or(0)));
        }
    }
    None
}

#[tauri::command]
async fn get_huggingface_models(repo_id: String) -> Result<Vec<String>, String> {
    let files = get_huggingface_files_with_size(&repo_id)?;
    let gguf_files: Vec<String> = files
        .into_iter()
        .filter(|f| f.path.to_lowercase().ends_with(".gguf"))
        .map(|f| f.path)
        .collect();
    Ok(gguf_files)
}

#[tauri::command]
async fn download_model(
    app: AppHandle,
    repo_id: String,
    filename: String,
) -> Result<ModelInfo, String> {
    let files = get_huggingface_files_with_size(&repo_id)?;

    let file = if filename.contains(".gguf") {
        files
            .iter()
            .find(|f| f.path == filename)
            .ok_or_else(|| "File not found in repository".to_string())?
    } else {
        files
            .iter()
            .find(|f| f.path.to_lowercase().ends_with(".gguf"))
            .ok_or_else(|| "No GGUF files found".to_string())?
    };

    let filename_for_url = file.path.clone();
    let actual_filename = file.path.clone();

    // Construct download URL from repo_id and filename using proper HuggingFace URL format
    let download_url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        repo_id, filename_for_url
    );
    let total_size = file.size.unwrap_or(0);

    let models_dir = utils::models_dir();

    std::fs::create_dir_all(&models_dir).map_err(|e| {
        format!(
            "Failed to create models directory: {}. Check permissions.",
            e
        )
    })?;

    let output_path = models_dir.join(&actual_filename);

    info!("Starting download from: {}", download_url);

    app.emit(
        "download_status",
        DownloadProgress {
            bytes_downloaded: 0,
            total_bytes: 0,
            filename: actual_filename.to_string(),
            status: "starting".to_string(),
        },
    )
    .ok();

    let client = reqwest::Client::builder()
        .user_agent("SL-Studio/0.2.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&download_url)
        .header("Accept", "application/octet-stream")
        .header("User-Agent", "SL-Studio/0.2.0")
        .send()
        .await
        .map_err(|e| format!("Failed to connect to HuggingFace: {}. Make sure you have accepted the model terms on the website.", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("HTTP error: {}. Response: {}", status, error_text));
    }

    let total_size = response.content_length().unwrap_or(total_size);

    let mut file =
        std::fs::File::create(&output_path).map_err(|e| format!("Failed to create file: {}", e))?;

    use futures::stream::StreamExt;
    use std::io::Write;
    let mut stream = response.bytes_stream();
    let mut bytes_downloaded = 0u64;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write: {}", e))?;

        bytes_downloaded += chunk.len() as u64;

        app.emit(
            "download_status",
            DownloadProgress {
                bytes_downloaded,
                total_bytes: total_size,
                filename: actual_filename.to_string(),
                status: "downloading".to_string(),
            },
        )
        .ok();
    }

    file.flush().map_err(|e| e.to_string())?;

    app.emit(
        "download_status",
        DownloadProgress {
            bytes_downloaded,
            total_bytes: total_size,
            filename: actual_filename.to_string(),
            status: "complete".to_string(),
        },
    )
    .ok();

    info!("Download complete: {:?}", output_path);

    Ok(ModelInfo {
        id: repo_id,
        filename,
        size: bytes_downloaded,
        path: output_path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
fn list_downloaded_models() -> Vec<ModelInfo> {
    let models_dir = if IS_DEV {
        utils::dev_models_dir()
    } else {
        utils::models_dir()
    };

    let mut models = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&models_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "gguf").unwrap_or(false) {
                if let Ok(metadata) = std::fs::metadata(&path) {
                    models.push(ModelInfo {
                        id: "local".to_string(),
                        filename: path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default(),
                        size: metadata.len(),
                        path: path.to_string_lossy().to_string(),
                    });
                }
            }
        }
    }

    models
}

#[tauri::command]
fn extract_file(path: String) -> Result<extractors::ExtractionResult, String> {
    use extractors::{Deconstructor, ExtractorConfig};

    let config = ExtractorConfig::default();
    let deconstructor = Deconstructor::new(config).map_err(|e| e.to_string())?;

    let path = std::path::Path::new(&path);
    deconstructor.extract(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_supported_extensions() -> Vec<String> {
    extractors::Deconstructor::supported_extensions()
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}

#[tauri::command]
fn init_reasoner(
    state: State<AppState>,
    model_path: String,
    context_size: u32,
    gpu_layers: Option<i32>,
) -> Result<bool, String> {
    // Default to 32 GPU layers for Apple Silicon, 0 for CPU only
    #[allow(clippy::unnecessary_lazy_evaluations)]
    let actual_gpu_layers = gpu_layers.unwrap_or({
        // Check if we have Apple Silicon for GPU acceleration
        #[cfg(target_os = "macos")]
        {
            // Use GPU layers if on macOS (Metal support)
            32
        }
        #[cfg(not(target_os = "macos"))]
        {
            0
        }
    });

    info!(
        "Initializing reasoner with GPU layers: {}",
        actual_gpu_layers
    );

    let config = ReasonerConfig {
        model_path,
        context_size,
        gpu_layers: actual_gpu_layers,
        temperature: 0.1,
        ..Default::default()
    };

    let reasoner = Reasoner::new(config).map_err(|e| e.to_string())?;

    let mut cached = state.reasoner.lock().unwrap();
    *cached = Some(Arc::new(reasoner));

    info!("Reasoner initialized and cached");
    Ok(true)
}

#[tauri::command]
fn analyze_file(state: State<AppState>, path: String) -> Result<inference::AnalysisResult, String> {
    let reasoner_arc = {
        let cached = state.reasoner.lock().unwrap();
        cached.clone()
    };

    let reasoner = reasoner_arc.ok_or("Reasoner not initialized. Call init_reasoner first.")?;

    let file_path = std::path::Path::new(&path);
    reasoner.analyze_file(file_path).map_err(|e| e.to_string())
}

#[tauri::command]
fn is_model_loaded(state: State<AppState>) -> bool {
    let cached = state.reasoner.lock().unwrap();
    cached
        .as_ref()
        .map(|r| r.is_model_loaded())
        .unwrap_or(false)
}

#[tauri::command]
fn get_reasoner_config(state: State<AppState>) -> Option<ReasonerConfig> {
    let cached = state.reasoner.lock().unwrap();
    cached.as_ref().map(|r| r.get_config())
}

#[tauri::command]
fn set_cancel_flag(state: State<AppState>, cancel: bool) -> bool {
    state.cancel_flag.store(cancel, Ordering::SeqCst);
    info!("Cancel flag set to: {}", cancel);
    cancel
}

#[tauri::command]
fn get_cancel_flag(state: State<AppState>) -> bool {
    state.cancel_flag.load(Ordering::SeqCst)
}

// Global logging guard - kept alive for app lifetime
static LOG_GUARD: std::sync::OnceLock<tracing_appender::non_blocking::WorkerGuard> =
    std::sync::OnceLock::new();

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    let guard = utils::init_logging();
    if let Ok(g) = guard {
        let _ = LOG_GUARD.set(g);
    }

    info!("SL Studio starting...");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            validate_config,
            detect_hardware,
            get_hardware_info,
            get_recommended_settings,
            get_system_monitor,
            get_processing_stats,
            init_project,
            start_registry,
            get_stats,
            get_workflow_state,
            get_extraction_statistics,
            get_unprocessed_files,
            mark_processed,
            get_app_data_dir,
            get_models_dir,
            create_project,
            load_project,
            save_project,
            get_default_project,
            download_model,
            get_huggingface_models,
            list_downloaded_models,
            extract_file,
            extract_batch,
            analyze_batch,
            get_extraction_queue,
            get_analysis_queue,
            get_supported_extensions,
            init_reasoner,
            analyze_file,
            is_model_loaded,
            get_reasoner_config,
            set_cancel_flag,
            get_cancel_flag,
            search_facts,
            search_entities,
            search_combined,
            get_timeline_events,
            get_overall_statistics,
            get_category_distribution,
            get_severity_distribution,
            get_entity_centrality,
            detect_anomalies,
            get_weighted_evidence,
            get_entity_relationships,
            get_connected_entities,
            add_tag,
            remove_tag,
            get_all_tags,
            add_annotation,
            update_annotation,
            delete_annotation,
            get_annotations,
            search_by_tags,
            get_location_entities,
            export_facts_json,
            export_entities_csv,
            export_timeline_json,
            export_full_report_json,
            export_facts_csv,
            write_file,
            export_pdf_report,
            export_excel_data,
            compare_projects,
            get_project_summary,
            create_backup,
            restore_backup,
            send_notification,
        ])
        .setup(|_app| {
            info!("Tauri app setup complete");
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            error!("Failed to run Tauri application: {}", e);
            std::process::exit(1);
        });
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub fingerprint: String,
    pub path: String,
    pub success: bool,
    pub char_count: usize,
    pub error: Option<String>,
    pub quality: Option<f64>,
    #[serde(skip)]
    pub extraction_text: Option<String>,
    #[serde(skip)]
    pub is_partial: bool,
}

impl Default for ExtractionResult {
    fn default() -> Self {
        Self {
            fingerprint: String::new(),
            path: String::new(),
            success: false,
            char_count: 0,
            error: None,
            quality: None,
            extraction_text: None,
            is_partial: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionProgress {
    pub total: usize,
    pub processed: usize,
    pub current_file: String,
    pub phase: String,
    pub success_count: usize,
    pub error_count: usize,
}

#[tauri::command]
async fn extract_batch(
    app: AppHandle,
    state: State<'_, AppState>,
    fingerprints: Vec<String>,
    cpu_workers: Option<u32>,
) -> Result<Vec<ExtractionResult>, String> {
    use extractors::{Deconstructor, ExtractorConfig};
    use rayon::prelude::*;

    let workers = {
        if let Some(w) = cpu_workers {
            w as usize
        } else {
            state.config.lock().unwrap().get_effective_workers() as usize
        }
    };

    info!(
        "Extracting batch of {} files with {} workers",
        fingerprints.len(),
        workers
    );

    let pool = get_or_create_thread_pool(workers);
    info!("Using thread pool with {} workers", workers);

    let total = fingerprints.len();

    // Phase 1: Pre-fetch ALL paths from DB BEFORE parallel (outside parallel)
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let file_data: Vec<(String, String)> = fingerprints
        .iter()
        .filter_map(|fingerprint| match db.get_registry_entry(fingerprint) {
            Ok(entry) => Some((fingerprint.clone(), entry.path)),
            Err(_) => None,
        })
        .collect();

    drop(db_guard);

    // Phase 2: Run parallel extraction using thread pool (NO locks)
    let deconstructor = {
        let config = ExtractorConfig::default();
        Deconstructor::new(config).map_err(|e| format!("Failed to create Deconstructor: {}", e))?
    };

    let results: Vec<ExtractionResult> = pool.install(|| {
        file_data
            .par_iter()
            .filter_map(|(fingerprint, path)| {
                let file_path = std::path::Path::new(path);
                if !file_path.exists() {
                    return Some(ExtractionResult {
                        fingerprint: fingerprint.clone(),
                        path: path.clone(),
                        success: false,
                        char_count: 0,
                        error: Some("File not found".to_string()),
                        quality: None,
                        extraction_text: None,
                        is_partial: false,
                    });
                }

                match deconstructor.extract(file_path) {
                    Ok(extraction) => Some(ExtractionResult {
                        fingerprint: fingerprint.clone(),
                        path: path.clone(),
                        success: true,
                        char_count: extraction.char_count,
                        error: None,
                        quality: Some(extraction.is_partial as u8 as f64),
                        extraction_text: Some(extraction.text),
                        is_partial: extraction.is_partial,
                    }),
                    Err(e) => {
                        error!("Extraction failed for {}: {}", path, e);
                        Some(ExtractionResult {
                            fingerprint: fingerprint.clone(),
                            path: path.clone(),
                            success: false,
                            char_count: 0,
                            error: Some(e.to_string()),
                            quality: None,
                            extraction_text: None,
                            is_partial: false,
                        })
                    }
                }
            })
            .collect()
    });

    // Phase 3: Write results to DB AFTER parallel completes
    let success_results: Vec<ExtractionResult> =
        results.iter().filter(|r| r.success).cloned().collect();

    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    for result in &success_results {
        if let Some(ref text) = result.extraction_text {
            let _ = db.save_text_cache(
                &result.fingerprint,
                &result.path,
                text,
                &result.fingerprint,
                0,
                result.quality.unwrap_or(0.0),
            );
            let _ = db.mark_extracted(&result.fingerprint, result.is_partial);
        }
    }

    drop(db_guard);

    let mut success_count = 0;
    let mut error_count = 0;
    let processed = results.len();

    for result in &results {
        if result.success {
            success_count += 1;
        } else {
            error_count += 1;
        }
    }

    let progress = ExtractionProgress {
        total,
        processed,
        current_file: String::new(),
        phase: "Complete".to_string(),
        success_count,
        error_count,
    };
    app.emit("extraction_progress", progress).ok();

    info!(
        "Extraction complete: {}/{} successful",
        success_count,
        results.len()
    );

    Ok(results)
}

#[tauri::command]
async fn analyze_batch(
    state: State<'_, AppState>,
    fingerprints: Vec<String>,
) -> Result<Vec<inference::AnalysisResult>, String> {
    let reasoner_arc = {
        let cached = state.reasoner.lock().unwrap();
        cached.clone()
    };

    let reasoner = reasoner_arc.ok_or("Reasoner not initialized. Call init_reasoner first.")?;

    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let mut results = Vec::new();

    for fingerprint in &fingerprints {
        // Get registry entry to find file path
        let entry = match db.get_registry_entry(fingerprint) {
            Ok(e) => e,
            Err(e) => {
                error!("Registry lookup failed for {}: {}", fingerprint, e);
                continue;
            }
        };

        // Get extracted text from cache
        let text = match db.get_extracted_text(fingerprint) {
            Ok(Some(t)) => t,
            Ok(None) => {
                error!("No extracted text found for {}", fingerprint);
                continue;
            }
            Err(e) => {
                error!("Failed to get extracted text for {}: {}", fingerprint, e);
                continue;
            }
        };

        // Run analysis on the already-extracted text
        match reasoner.analyze_text(fingerprint, &entry.file_name, &text) {
            Ok(result) => {
                // Save each fact to the intelligence database
                for fact in &result.facts {
                    // Store location and people as comma-separated strings
                    let location_str = fact.location.clone();
                    let people_str = if fact.people.is_empty() {
                        None
                    } else {
                        Some(fact.people.join(", "))
                    };

                    let intel_entry = IntelligenceEntry {
                        id: 0,
                        registry_id: entry.id,
                        fingerprint: fingerprint.clone(),
                        filename: entry.file_name.clone(),
                        source_quote: fact.source_quote.clone(),
                        page_number: None,
                        evidence_full: None,
                        evidence_hash: None,
                        associated_date: fact.date.clone(),
                        location: location_str,
                        people: people_str,
                        fact_summary: fact.summary.clone(),
                        category: Some(fact.category.clone()),
                        identified_crime: fact.identified_crime.clone(),
                        severity_score: fact.severity,
                        confidence: Some(fact.confidence as f64),
                        quality_score: Some(result.quality_score as f64),
                        source_language: None,
                        translated_quote: None,
                        pipeline_id: None,
                        pass_name: None,
                        is_deleted: false,
                        deleted_at: None,
                        processing_time_ms: None,
                        created_at: None,
                    };
                    
                    if let Err(e) = db.insert_intelligence(&intel_entry) {
                        error!("Failed to save fact for {}: {}", fingerprint, e);
                    }
                }

                // Mark as processed
                let _ = db.mark_processed(fingerprint);
                info!("Saved {} facts from {}", result.facts.len(), entry.file_name);
                results.push(result);
            }
            Err(e) => {
                error!("Analysis failed for {}: {}", fingerprint, e);
            }
        }
    }

    info!("Analysis complete: {} files processed", results.len());
    Ok(results)
}

#[tauri::command]
fn get_extraction_queue(
    state: State<AppState>,
    limit: i64,
) -> Result<Vec<core::RegistryEntry>, String> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.get_extraction_queue(limit).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_analysis_queue(
    state: State<AppState>,
    limit: i64,
) -> Result<Vec<core::RegistryEntry>, String> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.get_analysis_queue(limit).map_err(|e| e.to_string())
}
