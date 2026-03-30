pub mod config;
pub mod core;
pub mod extractors;
pub mod gpu;
pub mod inference;
pub mod models;
pub mod utils;

use config::{AppConfig, ProjectFile, ValidationResult};
use core::{Database, RegistryProgress, RegistryWorker};
use gpu::HardwareStatus;
use inference::{Reasoner, ReasonerConfig};
pub use models::{ModelManager, Quantization};

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};
use tracing::info;

#[cfg(feature = "custom-protocol")]
const IS_DEV: bool = true;

#[cfg(not(feature = "custom-protocol"))]
const IS_DEV: bool = false;

pub struct AppState {
    config: Mutex<AppConfig>,
    db: Mutex<Option<Database>>,
    registry_worker: Mutex<Option<RegistryWorker>>,
    reasoner: Mutex<Option<Arc<Reasoner>>>,
}

impl Default for AppState {
    fn default() -> Self {
        let config = AppConfig::load().unwrap_or_default();
        AppState {
            config: Mutex::new(config),
            db: Mutex::new(None),
            registry_worker: Mutex::new(None),
            reasoner: Mutex::new(None),
        }
    }
}

// Response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub registry_count: i64,
    pub intelligence_count: i64,
}

// Commands
#[tauri::command]
fn load_config(state: State<AppState>) -> Result<AppConfig, String> {
    let config = state.config.lock().unwrap().clone();
    info!("Config loaded");
    Ok(config)
}

#[tauri::command]
fn save_config(state: State<AppState>, config: AppConfig) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())?;
    *state.config.lock().unwrap() = config;
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

    *state.db.lock().unwrap() = Some(db);

    // Create registry worker
    let worker = RegistryWorker::new(
        &config.project.evidence_root,
        &config.project.registry_db,
        &config.project.intelligence_db,
    )
    .map_err(|e| e.to_string())?;

    *state.registry_worker.lock().unwrap() = Some(worker);

    // Save config
    config.save().map_err(|e| e.to_string())?;
    *state.config.lock().unwrap() = config;

    info!("Project initialized successfully");
    app.emit("project_initialized", true).ok();

    Ok(true)
}

#[tauri::command]
async fn start_registry(app: AppHandle, state: State<'_, AppState>) -> Result<usize, String> {
    let (evidence_root, registry_db, intelligence_db) = {
        let config = state.config.lock().unwrap();
        (
            config.project.evidence_root.clone(),
            config.project.registry_db.clone(),
            config.project.intelligence_db.clone(),
        )
    };

    if evidence_root.is_empty() {
        return Err("Evidence root not set".to_string());
    }

    let mut worker = RegistryWorker::new(&evidence_root, &registry_db, &intelligence_db)
        .map_err(|e| e.to_string())?;

    // Create channel for progress
    let (tx, rx) = std::sync::mpsc::channel::<RegistryProgress>();

    // Spawn progress listener
    let app_clone = app.clone();
    std::thread::spawn(move || {
        for progress in rx {
            app_clone.emit("registry_progress", progress).ok();
        }
    });

    let result = worker.scan(tx).map_err(|e| e.to_string())?;

    app.emit("registry_complete", result).ok();
    Ok(result)
}

#[tauri::command]
fn get_stats(state: State<AppState>) -> Result<Stats, String> {
    let db = state.db.lock().unwrap();
    if let Some(db) = db.as_ref() {
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
        let facts = db.get_weighted_evidence(0.0, 10000).map_err(|e| e.to_string())?;
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
        let facts = db.get_weighted_evidence(min_weight, limit).map_err(|e| e.to_string())?;
        
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

    let db = state.db.lock().unwrap();
    let db_ref = db.as_ref().ok_or("Database not initialized")?;

    let facts = db_ref.get_weighted_evidence(0.0, 100).map_err(|e| e.to_string())?;
    let stats = db_ref.get_overall_statistics().map_err(|e| e.to_string())?;
    let categories = db_ref.get_category_distribution().map_err(|e| e.to_string())?;

    let (doc, page1, layer1) = PdfDocument::new(
        "SL Studio Forensic Report",
        Mm(210.0),
        Mm(297.0),
        "Layer 1",
    );

    let font = doc.add_builtin_font(BuiltinFont::Helvetica).map_err(|e| e.to_string())?;
    let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold).map_err(|e| e.to_string())?;

    let current_layer = doc.get_page(page1).get_layer(layer1);

    current_layer.use_text("SL Studio - Forensic Document Analysis Report", 24.0, Mm(20.0), Mm(277.0), &font_bold);
    current_layer.use_text(&format!("Generated: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")), 10.0, Mm(20.0), Mm(268.0), &font);

    current_layer.use_text("Summary Statistics", 16.0, Mm(20.0), Mm(250.0), &font_bold);
    current_layer.use_text(&format!("Total Evidence: {}", stats.registry_count), 12.0, Mm(25.0), Mm(240.0), &font);
    current_layer.use_text(&format!("Total Intelligence: {}", stats.intelligence_count), 12.0, Mm(25.0), Mm(232.0), &font);
    current_layer.use_text(&format!("Processed Files: {}", stats.processed_count), 12.0, Mm(25.0), Mm(224.0), &font);

    current_layer.use_text("Category Distribution", 16.0, Mm(20.0), Mm(205.0), &font_bold);
    let mut y_pos = 195.0;
    for cat in categories.iter().take(10) {
        current_layer.use_text(&format!("{}: {} items", cat.category, cat.count), 11.0, Mm(25.0), Mm(y_pos), &font);
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
        current_layer.use_text(&format!("{}. [{}] {}", i + 1, fact.category.as_deref().unwrap_or("N/A"), summary), 9.0, Mm(25.0), Mm(y_pos), &font);
        y_pos -= 6.0;
    }

    let mut pdf_bytes = Vec::new();
    doc.save(&mut std::io::Cursor::new(&mut pdf_bytes)).map_err(|e| e.to_string())?;

    Ok(pdf_bytes)
}

#[tauri::command]
fn export_excel_report(state: State<AppState>) -> Result<Vec<u8>, String> {
    use calamine::{Workbook, Xlsx, DataType};

    let db = state.db.lock().unwrap();
    let db_ref = db.as_ref().ok_or("Database not initialized")?;

    let facts = db_ref.get_weighted_evidence(0.0, 1000).map_err(|e| e.to_string())?;
    let categories = db_ref.get_category_distribution().map_err(|e| e.to_string())?;
    let entities = db_ref.get_entity_centrality(None, 0.0).map_err(|e| e.to_string())?;
    let timeline = db_ref.get_timeline_events(None, None, 1000).map_err(|e| e.to_string())?;

    let mut workbook: Workbook<Xlsx, Vec<u8>> = Workbook::new();
    
    {
        let sheet = workbook.add_worksheet("Facts");
        sheet.set_name("Facts").map_err(|e| e.to_string())?;
        
        let headers = ["ID", "Filename", "Category", "Severity", "Confidence", "Quality", "Weight", "Summary", "Created"];
        for (i, header) in headers.iter().enumerate() {
            sheet.set_cell_value(0, i as u32, *header).map_err(|e| e.to_string())?;
        }
        
        for (row, fact) in facts.iter().enumerate().take(1000) {
            let r = (row + 1) as u32;
            sheet.set_cell_value(r, 0, fact.id).map_err(|e| e.to_string())?;
            sheet.set_cell_value(r, 1, fact.filename.as_str()).map_err(|e| e.to_string())?;
            sheet.set_cell_value(r, 2, fact.category.as_deref().unwrap_or("")).map_err(|e| e.to_string())?;
            sheet.set_cell_value(r, 3, fact.severity).map_err(|e| e.to_string())?;
            sheet.set_cell_value(r, 4, fact.confidence.unwrap_or(0.0)).map_err(|e| e.to_string())?;
            sheet.set_cell_value(r, 5, fact.quality.unwrap_or(0.0)).map_err(|e| e.to_string())?;
            sheet.set_cell_value(r, 6, fact.weight).map_err(|e| e.to_string())?;
            sheet.set_cell_value(r, 7, fact.summary.as_str()).map_err(|e| e.to_string())?;
            sheet.set_cell_value(r, 8, fact.created_at.as_deref().unwrap_or("")).map_err(|e| e.to_string())?;
        }
    }

    {
        let sheet = workbook.add_worksheet("Categories");
        sheet.set_name("Categories").map_err(|e| e.to_string())?;
        
        sheet.set_cell_value(0, 0, "Category").map_err(|e| e.to_string())?;
        sheet.set_cell_value(0, 1, "Count").map_err(|e| e.to_string())?;
        sheet.set_cell_value(0, 2, "Avg Severity").map_err(|e| e.to_string())?;
        sheet.set_cell_value(0, 3, "Avg Confidence").map_err(|e| e.to_string())?;
        
        for (row, cat) in categories.iter().enumerate() {
            let r = (row + 1) as u32;
            sheet.set_cell_value(r, 0, cat.category.as_str()).map_err(|e| e.to_string())?;
            sheet.set_cell_value(r, 1, cat.count).map_err(|e| e.to_string())?;
            sheet.set_cell_value(r, 2, cat.avg_severity.unwrap_or(0.0)).map_err(|e| e.to_string())?;
            sheet.set_cell_value(r, 3, cat.avg_confidence.unwrap_or(0.0)).map_err(|e| e.to_string())?;
        }
    }

    {
        let sheet = workbook.add_worksheet("Entities");
        sheet.set_name("Entities").map_err(|e| e.to_string())?;
        
        sheet.set_cell_value(0, 0, "Entity ID").map_err(|e| e.to_string())?;
        sheet.set_cell_value(0, 1, "Type").map_err(|e| e.to_string())?;
        sheet.set_cell_value(0, 2, "Value").map_err(|e| e.to_string())?;
        sheet.set_cell_value(0, 3, "Doc Count").map_err(|e| e.to_string())?;
        sheet.set_cell_value(0, 4, "Occurrences").map_err(|e| e.to_string())?;
        sheet.set_cell_value(0, 5, "Centrality").map_err(|e| e.to_string())?;
        
        for (row, entity) in entities.iter().enumerate().take(500) {
            let r = (row + 1) as u32;
            sheet.set_cell_value(r, 0, entity.entity_id).map_err(|e| e.to_string())?;
            sheet.set_cell_value(r, 1, entity.entity_type.as_str()).map_err(|e| e.to_string())?;
            sheet.set_cell_value(r, 2, entity.value.as_str()).map_err(|e| e.to_string())?;
            sheet.set_cell_value(r, 3, entity.document_count).map_err(|e| e.to_string())?;
            sheet.set_cell_value(r, 4, entity.occurrence_count).map_err(|e| e.to_string())?;
            sheet.set_cell_value(r, 5, entity.centrality_score).map_err(|e| e.to_string())?;
        }
    }

    {
        let sheet = workbook.add_worksheet("Timeline");
        sheet.set_name("Timeline").map_err(|e| e.to_string())?;
        
        sheet.set_cell_value(0, 0, "Date").map_err(|e| e.to_string())?;
        sheet.set_cell_value(0, 1, "Event Type").map_err(|e| e.to_string())?;
        sheet.set_cell_value(0, 2, "Description").map_err(|e| e.to_string())?;
        sheet.set_cell_value(0, 3, "Filename").map_err(|e| e.to_string())?;
        
        for (row, event) in timeline.iter().enumerate().take(1000) {
            let r = (row + 1) as u32;
            sheet.set_cell_value(r, 0, event.date.as_str()).map_err(|e| e.to_string())?;
            sheet.set_cell_value(r, 1, event.event_type.as_str()).map_err(|e| e.to_string())?;
            sheet.set_cell_value(r, 2, event.description.as_str()).map_err(|e| e.to_string())?;
            sheet.set_cell_value(r, 3, event.filename.as_deref().unwrap_or("")).map_err(|e| e.to_string())?;
        }
    }

    workbook.save_new_buffer().map_err(|e| e.to_string())
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
    pub r#type: String,
    pub size: u64,
    pub download_url: Option<String>,
    pub path: String,
}

fn get_huggingface_files(repo_id: &str) -> Result<Vec<HuggingFaceFile>, String> {
    let url = format!("https://huggingface.co/api/models/{}", repo_id);

    let client = reqwest::blocking::Client::new();
    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .map_err(|e| format!("Failed to fetch model info: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    #[derive(Deserialize)]
    struct ModelInfo {
        siblings: Option<Vec<HuggingFaceFile>>,
    }

    let info: ModelInfo = response.json().map_err(|e| e.to_string())?;

    info.siblings.ok_or_else(|| "No files found".to_string())
}

#[allow(dead_code)]
fn find_gguf_file(files: &[HuggingFaceFile]) -> Option<(String, u64)> {
    for file in files {
        if file.path.to_lowercase().ends_with(".gguf") {
            let url = file.download_url.as_ref()?;
            return Some((url.clone(), file.size));
        }
    }
    None
}

#[tauri::command]
async fn get_huggingface_models(repo_id: String) -> Result<Vec<String>, String> {
    let files = get_huggingface_files(&repo_id)?;
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
    let files = get_huggingface_files(&repo_id)?;

    let (download_url, total_size) = if filename.contains(".gguf") {
        files
            .iter()
            .find(|f| f.path == filename)
            .and_then(|f| f.download_url.as_ref().map(|url| (url.clone(), f.size)))
            .ok_or_else(|| "File not found in repository".to_string())?
    } else {
        let file = files
            .iter()
            .find(|f| f.path.to_lowercase().ends_with(".gguf"))
            .ok_or_else(|| "No GGUF files found".to_string())?;
        (file.download_url.as_ref().unwrap().clone(), file.size)
    };

    let actual_filename = download_url.split('/').next_back().unwrap_or(&filename);
    let models_dir = if IS_DEV {
        utils::dev_models_dir()
    } else {
        utils::models_dir()
    };

    std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;

    let output_path = models_dir.join(actual_filename);

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

    let client = reqwest::Client::new();

    let response = client
        .get(&download_url)
        .header("Accept", "application/octet-stream")
        .send()
        .await
        .map_err(|e| format!("Failed to connect: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
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
) -> Result<bool, String> {
    let config = ReasonerConfig {
        model_path,
        context_size,
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
            init_project,
            start_registry,
            get_stats,
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
            get_supported_extensions,
            init_reasoner,
            analyze_file,
            is_model_loaded,
            get_reasoner_config,
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
        ])
        .setup(|_app| {
            info!("Tauri app setup complete");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
