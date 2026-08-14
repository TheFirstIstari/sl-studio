// SL Studio — Tauri 2 + SvelteKit 5 forensic document analysis app.
// The Rust backend provides IPC commands (annotated with `#[tauri::command]`)
// that the SvelteKit frontend invokes via `@tauri-apps/api/core`.
//
// Module layout:
//   lib.rs         — public types, require_db() helper, run()
//   commands/mod.rs — all #[tauri::command] handlers
//   core/database.rs — SQLite Pool + migrations
//   core/mod.rs       — module re-export
//   extractors/       — PDF / image / audio / DOCX extractors
//   inference/        — MLX pipeline, reasoner
//   tauri/mod.rs      — AppState definition

use anyhow::Context;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use tracing::info;

mod app;
mod commands;
mod core;
mod extractors;
mod inference;

// ── Error type ───────────────────────────────────────────────────
// Tauri 2's `#[tauri::command]` macro requires the error type to implement
// `Into<InvokeError>`. `anyhow::Error` does not implement `Serialize`, so it
// cannot be converted into `InvokeError` via the blanket `From<T: Serialize>`.
// `AppError` is a thin wrapper that provides `Serialize` while preserving the
// error message.

/// Serializable error type for Tauri IPC commands.
#[derive(Debug, Clone)]
pub struct AppError(pub String);

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for AppError {}

impl serde::Serialize for AppError {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError(e.to_string())
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError(s)
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError(s.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

// ── Types ──────────────────────────────────────────────────────────

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Metadata {
    pub filename: String,
    pub category: String,
    pub severity_score: u8,
    pub confidence: Option<f64>,
    pub identified_crime: Option<String>,
    pub fact_summary: String,
    pub fingerprint: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Fact {
    pub id: u64,
    pub fingerprint: String,
    pub filename: String,
    pub fact_summary: String,
    pub category: Option<String>,
    pub identified_crime: Option<String>,
    pub severity_score: u8,
    pub confidence: Option<f64>,
    pub created_at: String,
}

/// Fact row returned to the frontend via search_facts and related queries.
/// Uses the same field names as the `Fact` struct above.

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Chain {
    pub id: String,
    pub chain_name: String,
    pub chain_type: String,
    pub item_count: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FileResult {
    pub file_path: String,
    pub extracted_summary: String,
    pub category: String,
    pub severity_score: u8,
    pub confidence: Option<f64>,
    pub fingerprint: String,
    pub extracted_at: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FacetPreset {
    pub id: i64,
    pub page: String,
    pub name: String,
    pub state_json: String,
    pub updated_at: Option<String>,
}

/// Built-in pipeline pass configuration (matches the frontend PipelinePass).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PipelinePass {
    pub name: String,
    pub description: String,
    pub prompt_template: String,
    pub output_schema: Option<String>,
    pub max_tokens: usize,
    pub temperature: f64,
    pub sample_size: Option<usize>,
}

/// Pipeline returned to the frontend (id is a string per the frontend).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Pipeline {
    pub id: String,
    pub name: String,
    pub description: String,
    pub passes: Vec<PipelinePass>,
    pub is_builtin: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AppConfig {
    pub version: String,
    pub project: ProjectConfig,
    pub model: ModelConfig,
    pub hardware: HardwareConfig,
    pub processing: ProcessingConfig,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub evidence_root: String,
    pub registry_db: String,
    pub intelligence_db: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ModelConfig {
    pub source: String,
    pub id: String,
    pub mlx_model_name: String,
    pub dtype: String,
    pub context_length: usize,
    pub downloaded: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HardwareConfig {
    pub gpu_backend: String,
    pub gpu_memory_fraction: f64,
    pub cpu_workers: usize,
    pub auto_scale_workers: bool,
    pub batch_size: usize,
    pub auto_scale_batch: bool,
    pub ocr_provider: String,
    pub whisper_size: String,
    pub whisper_model_path: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ProcessingConfig {
    pub batch_size: usize,
    pub max_image_resolution: usize,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HardwareStatus {
    pub cpu_cores: usize,
    pub total_memory: usize,
    pub available_memory: usize,
    pub gpu_backend: String,
    pub gpu_name: String,
    pub gpu_memory: usize,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HardwareInfo {
    pub recommended_context: usize,
    pub recommended_batch_size: usize,
    pub worker_count: usize,
    pub backend: String,
}

/// Settings page's `get_hardware_info` return type.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HardwareInfoExt {
    pub cpu_threads: usize,
    pub total_memory_gb: f64,
    pub available_memory_gb: f64,
    pub recommended_workers: usize,
    pub recommended_batch_size: usize,
    pub cpu_workers: usize,
}

/// Settings page's `get_system_monitor` return type.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SystemMonitor {
    pub cpu_usage_percent: f64,
    pub memory_used_gb: f64,
    pub memory_available_gb: f64,
    pub memory_percent: f64,
}

/// Model info returned by `list_downloaded_models` / `download_model`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DownloadedModel {
    pub id: String,
    pub filename: String,
    pub size: u64,
    pub path: String,
    pub mlx_model_name: String, // e.g., "qwen3.5-4b-4bit"
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ProjectStats {
    pub total_files: u64,
    pub files_scanned: u64,
    pub files_extracted: u64,
    pub files_analyzed: u64,
    pub total_facts: u64,
    pub total_entities: u64,
    pub registry_count: u64,
    pub intelligence_count: u64,
    pub total_characters: u64,
    pub average_characters: f64,
    pub average_quality: f64,
    pub partial_count: u64,
    pub files_by_type: HashMap<String, u64>,
    pub files_scanned_at: Option<String>,
    pub files_extracted_at: Option<String>,
    pub files_analyzed_at: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WorkflowState {
    pub files_scanned: u64,
    pub files_extracted: u64,
    pub files_analyzed: u64,
    pub current_stage: String,
    pub is_scanning: bool,
    pub is_extracting: bool,
    pub is_analyzing: bool,
    pub scan_progress: usize,
    pub extract_progress: usize,
    pub analyze_progress: usize,
    pub current_file: String,
    pub processed_count: usize,
    pub total_count: usize,
}

// ── Frontend-facing response types ────────────────────────────────

/// Chains page — summary row in the chain list.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ChainSummary {
    pub id: i64,
    pub chain_name: String,
    pub chain_type: String,
    pub description: Option<String>,
    pub created_by: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub item_count: u64,
    pub avg_strength: Option<f64>,
}

/// Chains page — a single link inside an evidence chain.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ChainItem {
    pub link_id: i64,
    pub intelligence_id: i64,
    pub relationship_type: String,
    pub relationship_strength: f64,
    pub notes: Option<String>,
    pub linked_by: Option<String>,
    pub linked_at: Option<String>,
    pub filename: String,
    pub fact_summary: String,
    pub category: Option<String>,
}

/// Chains page — full chain detail (summary + items).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EvidenceChain {
    pub id: i64,
    pub chain_name: String,
    pub chain_type: String,
    pub description: Option<String>,
    pub created_by: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub items: Vec<ChainItem>,
}

/// Entities page — alias suggestion for entity resolution.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EntityMatchSuggestion {
    pub canonical_id: i64,
    pub canonical_value: String,
    pub alias_id: i64,
    pub alias_value: String,
    pub entity_type: String,
    pub similarity: f64,
    pub reason: String,
}

/// Stats page — aggregated statistics.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct OverallStats {
    pub total_facts: u64,
    pub avg_severity: f64,
    pub avg_confidence: f64,
    pub avg_quality: f64,
    pub total_entities: u64,
    pub unique_entities: u64,
    pub total_chains: u64,
    pub total_chain_links: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CategoryStat {
    pub category: String,
    pub count: u64,
    pub avg_severity: Option<f64>,
    pub avg_confidence: Option<f64>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SeverityStat {
    pub severity: i64,
    pub count: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EntityCentrality {
    pub entity_id: i64,
    pub entity_type: String,
    pub value: String,
    pub document_count: u64,
    pub occurrence_count: u64,
    pub avg_confidence: Option<f64>,
    pub centrality_score: f64,
}

/// Network page — entity relationship edge.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EntityRelationship {
    pub entity1_id: i64,
    pub entity1_type: String,
    pub entity1_value: String,
    pub entity2_id: i64,
    pub entity2_type: String,
    pub entity2_value: String,
    pub cooccurrence: i64,
    pub avg_confidence: Option<f64>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ConnectedEntity {
    pub entity_id: i64,
    pub entity_type: String,
    pub value: String,
    pub confidence: Option<f64>,
    pub source_file: String,
    pub distance: i64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EntityCommunity {
    pub community_id: i64,
    pub entity_ids: Vec<i64>,
    pub size: i64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EntityBetweenness {
    pub entity_id: i64,
    pub betweenness: f64,
}

/// Quality page — deduplication.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DuplicateGroup {
    pub keeper_id: i64,
    pub member_ids: Vec<i64>,
    pub similarity: f64,
}

/// Quality page — cross-validation.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CorroborationMatch {
    pub intelligence_id: i64,
    pub filename: String,
    pub fact_summary: String,
    pub similarity: f64,
    pub agreement: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CrossValidationResult {
    pub intelligence_id: i64,
    pub source_filename: String,
    pub matches: Vec<CorroborationMatch>,
    pub consensus_score: f64,
}

/// Analysis page — extraction stats / results.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ExtractionStats {
    pub total_files: u64,
    pub total_characters: u64,
    pub average_characters: f64,
    pub average_quality: f64,
    pub partial_count: u64,
    pub files_by_type: HashMap<String, u64>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ExtractionResult {
    pub fingerprint: String,
    pub path: String,
    pub success: bool,
    pub char_count: u64,
    pub error: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RegistryFile {
    pub path: String,
    pub fingerprint: String,
}

/// Timeline page — single timeline event.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TimelineEvent {
    pub id: i64,
    pub fingerprint: String,
    pub filename: String,
    pub summary: String,
    pub category: Option<String>,
    pub date: String,
    pub severity: i64,
    pub confidence: Option<f64>,
}

/// Metadata page — registry file entry and document metadata.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RegistryEntry {
    pub id: i64,
    pub fingerprint: String,
    pub path: String,
    pub file_name: String,
    pub file_type: Option<String>,
    pub file_size: i64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DocumentMetadata {
    pub source: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub keywords: Option<String>,
    pub camera_model: Option<String>,
    pub gps_latitude: Option<f64>,
    pub gps_longitude: Option<f64>,
    pub audio_duration_seconds: Option<f64>,
    pub audio_sample_rate: Option<f64>,
    pub audio_channels: Option<f64>,
    pub audio_format: Option<String>,
    pub audio_codec: Option<String>,
    pub audio_bits_per_sample: Option<f64>,
    pub raw: HashMap<String, String>,
}

/// Compare page — project summary and comparison.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ProjectSummary {
    pub name: String,
    pub path: String,
    pub fact_count: u64,
    pub entity_count: u64,
    pub timeline_count: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EntityOverlap {
    pub entity_value: String,
    pub entity_type: String,
    pub count_project1: u64,
    pub count_project2: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TimelineCorrelation {
    pub correlation_score: f64,
    pub aligned_events: u64,
    pub project1_date_range: [String; 2],
    pub project2_date_range: [String; 2],
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CommonEntity {
    pub entity_id: i64,
    pub entity_type: String,
    pub value: String,
    pub occurrence_count: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ProjectComparison {
    pub project1_name: String,
    pub project2_name: String,
    pub entity_overlap: Vec<EntityOverlap>,
    pub common_entities: Vec<CommonEntity>,
    pub timeline_correlation: TimelineCorrelation,
    pub fact_similarity: f64,
}

/// Anomalies page.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Anomaly {
    pub id: i64,
    pub fingerprint: String,
    pub filename: String,
    pub summary: String,
    pub metric: String,
    pub value: f64,
    pub expected_value: f64,
    pub deviation: f64,
    pub associated_date: Option<String>,
}

/// Maps page — location entity for the map view.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LocationEntity {
    pub id: i64,
    pub name: String,
    pub normalized_name: Option<String>,
    pub confidence: Option<f64>,
    pub fingerprint: String,
    pub source_file: String,
    pub fact_summary: Option<String>,
    pub severity: i64,
}

/// Backup result returned to the frontend by `create_backup`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BackupResult {
    pub backup_path: String,
    pub size_bytes: u64,
    pub created_at: String,
}

// ── AppState ─────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<core::database::Pool>,
    pub metadata: HashMap<String, Metadata>,
    pub facts: HashMap<String, Fact>,
    pub chains: HashMap<String, Chain>,
    pub file_results: HashMap<String, FileResult>,
    pub reasoner: Arc<Mutex<Option<inference::reasoner::Reasoner>>>,
}

// ── Database singleton ───────────────────────────────────────────

static DB_POOL: OnceLock<core::database::Pool> = OnceLock::new();

/// Return a clone of the shared database connection pool.
/// Initialises the pool (and runs migrations) on first call.
pub fn require_db() -> Result<core::database::Pool> {
    let pool = DB_POOL.get_or_init(|| {
        let db_path = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("sl-studio.db");
        let db_path_str = db_path.to_string_lossy();
        core::database::Pool::connect(&db_path_str).expect("Database initialisation failed")
    });
    Ok(pool.clone())
}

// ── Tauri Builder ─────────────────────────────────────────────────

/// Build the Tauri application and return the AppState.
pub fn build_tauri_app() -> Result<AppState> {
    let db = require_db()?;
    info!("Connected to SQLite database");

    let app = AppState {
        db: Arc::new(db),
        metadata: HashMap::new(),
        facts: HashMap::new(),
        chains: HashMap::new(),
        file_results: HashMap::new(),
        reasoner: Arc::new(Mutex::new(None)),
    };

    Ok(app)
}

/// Clean up the Tauri app and database on shutdown.
pub fn cleanup() -> Result<()> {
    info!("Shutting down SL Studio backend");
    Ok(())
}

// ── Entry point ───────────────────────────────────────────────────

/// Build and run the Tauri application.
pub fn run() -> Result<()> {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(app::setup)
        .invoke_handler(tauri::generate_handler![
            // Project / config
            commands::load_config,
            commands::save_config,
            commands::init_project,
            // Registry / extraction
            commands::start_registry,
            commands::get_extraction_queue,
            commands::extract_batch,
            commands::get_extraction_statistics,
            commands::get_analysis_queue,
            // Facts
            commands::search_facts,
            commands::export_facts_json,
            commands::export_facts_csv,
            commands::export_entities_csv,
            commands::export_timeline_json,
            commands::export_full_report_json,
            commands::export_pdf_report,
            commands::export_excel_data,
            commands::delete_facts,
            commands::update_fact_verification,
            // Entities
            commands::suggest_entity_matches,
            commands::add_entity_alias,
            commands::get_entity_relationships,
            commands::get_connected_entities,
            commands::detect_entity_communities,
            commands::compute_betweenness_centrality,
            commands::get_location_entities,
            commands::get_entity_centrality,
            // Evidence chains
            commands::list_evidence_chains,
            commands::create_evidence_chain,
            commands::get_evidence_chain,
            commands::delete_evidence_chain,
            commands::add_to_evidence_chain,
            commands::remove_from_evidence_chain,
            // Facets
            commands::list_facet_presets,
            commands::save_facet_preset,
            commands::delete_facet_preset,
            // Pipelines
            commands::list_pipelines,
            commands::save_pipeline,
            commands::delete_pipeline,
            // Quality
            commands::find_duplicate_facts,
            commands::merge_duplicate_facts,
            commands::cross_validate_fact,
            commands::get_evidence_weight,
            commands::detect_anomalies,
            // Timeline
            commands::get_timeline_events,
            // Metadata
            commands::get_registry_files,
            commands::get_cached_metadata,
            commands::extract_metadata,
            commands::cache_metadata,
            // Stats
            commands::get_stats,
            commands::get_overall_statistics,
            commands::get_category_distribution,
            commands::get_severity_distribution,
            // Hardware / model
            commands::detect_hardware,
            commands::get_hardware_info,
            commands::get_recommended_settings,
            commands::get_system_monitor,
            commands::list_downloaded_models,
            commands::download_model,
            commands::is_model_loaded,
            commands::validate_model,
            commands::init_reasoner,
            // Analysis
            commands::analyze_batch,
            commands::set_cancel_flag,
            // Workflow
            commands::get_workflow_state,
            // Compare
            commands::get_project_summary,
            commands::compare_projects,
            // Utility
            commands::write_file,
            commands::create_backup,
            commands::restore_backup,
        ])
        .run(tauri::generate_context!())
        .context("Failed to start Tauri application")?;

    Ok(())
}

// ── Re-export for convenience ─────────────────────────────────────

pub use crate::core::database::Pool;
