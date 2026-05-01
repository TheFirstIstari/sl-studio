pub mod commands;
pub mod config;
pub mod core;
pub mod extractors;
pub mod gpu;
pub mod inference;
pub mod models;
pub mod utils;

use config::AppConfig;
use core::{Database, RegistryWorker};
use inference::Reasoner;
pub use models::{ModelManager, Quantization};

// Re-export WorkflowState at crate root for backwards compatibility with
// `crate::WorkflowState` references in core::database (and any external code).
pub use commands::workflow::WorkflowState;

use std::sync::atomic::AtomicBool;
use std::sync::OnceLock;
use std::sync::{Arc, Mutex, RwLock};
use tracing::{error, info};

static GLOBAL_THREAD_POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();

pub(crate) fn get_or_create_thread_pool(workers: usize) -> &'static rayon::ThreadPool {
    GLOBAL_THREAD_POOL.get_or_init(|| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(workers)
            .build()
            .expect("Failed to create global thread pool")
    })
}

#[cfg(feature = "custom-protocol")]
pub(crate) const IS_DEV: bool = true;

#[cfg(not(feature = "custom-protocol"))]
pub(crate) const IS_DEV: bool = false;

pub struct AppState {
    /// Read-heavy: written only by `save_config` and `init_project`.
    pub config: RwLock<AppConfig>,
    /// Read-heavy: written only by `init_project` and `restore_backup`.
    pub db: RwLock<Option<Arc<Database>>>,
    /// Written once during `init_project`, never read after that (just held).
    pub registry_worker: RwLock<Option<RegistryWorker>>,
    /// Written once during `init_reasoner`, read by all inference commands.
    pub reasoner: RwLock<Option<Arc<Reasoner>>>,
    pub cancel_flag: AtomicBool,
    /// Updated frequently during batch runs; Mutex wrapped in an Arc so
    /// `BusyGuard` can hold its own owning handle and release the slot
    /// from its `Drop` impl even if the command panics or returns early.
    pub processing: Arc<Mutex<ProcessingState>>,
}

impl AppState {
    /// Hand out an owned reference to the processing mutex. Used by
    /// `BusyGuard` so the guard outlives the `State<AppState>` borrow.
    pub fn processing_arc(&self) -> Arc<Mutex<ProcessingState>> {
        Arc::clone(&self.processing)
    }
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
            config: RwLock::new(config),
            db: RwLock::new(None),
            registry_worker: RwLock::new(None),
            reasoner: RwLock::new(None),
            cancel_flag: AtomicBool::new(false),
            processing: Arc::new(Mutex::new(ProcessingState::default())),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProcessingState {
    pub is_scanning: bool,
    pub is_extracting: bool,
    pub is_analyzing: bool,
    pub scan_progress: f32,
    pub extract_progress: f32,
    pub analyze_progress: f32,
    pub current_file: String,
    pub processed_count: i64,
    pub total_count: i64,
}

// Global logging guard - kept alive for app lifetime
static LOG_GUARD: std::sync::OnceLock<tracing_appender::non_blocking::WorkerGuard> =
    std::sync::OnceLock::new();

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let guard = utils::init_logging();
    if let Ok(g) = guard {
        let _ = LOG_GUARD.set(g);
    }

    info!("SL Studio starting...");

    use commands::*;

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
            update_processing_state,
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
            validate_model,
            get_reasoner_config,
            set_cancel_flag,
            get_cancel_flag,
            search_facts,
            update_fact_verification,
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
            delete_facts,
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
            get_schema_version,
            // FR-DEDUP
            find_duplicate_facts,
            merge_duplicate_facts,
            // FR-WEIGHT
            get_evidence_weight,
            // FR-VERIF
            cross_validate_fact,
            // FR-FACET-004
            save_facet_preset,
            list_facet_presets,
            delete_facet_preset,
            // FR-PLP
            list_pipelines,
            save_pipeline,
            get_pipeline,
            delete_pipeline,
            // FR-ER
            suggest_entity_matches,
            add_entity_alias,
            resolve_entity_alias,
            // FR-NET
            detect_entity_communities,
            compute_betweenness_centrality,
            compute_clustering_coefficients,
            // FR-CHAIN
            create_evidence_chain,
            list_evidence_chains,
            get_evidence_chain,
            add_to_evidence_chain,
            remove_from_evidence_chain,
            update_evidence_chain,
            delete_evidence_chain,
            get_evidence_chain_statistics,
            // FR-LANG
            detect_text_language,
            // FR-STRUCT
            extract_pdf_form_fields,
            extract_key_value_pairs,
            // FR-META
            extract_metadata,
            cache_metadata,
            get_cached_metadata,
            get_registry_files,
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
