use crate::commands::require_db;
use crate::commands::workflow::{BusyGuard, Operation};
use crate::core::{self, RegistryProgress, RegistryWorker};
use crate::extractors;
use crate::AppState;
use tauri::{AppHandle, Emitter, State};
use tracing::info;

#[tauri::command]
pub async fn start_registry(app: AppHandle, state: State<'_, AppState>) -> Result<usize, String> {
    // Mutual-exclusion gate: scan/extract/analyze cannot run
    // concurrently. The guard auto-clears the flag on drop.
    let _guard = BusyGuard::acquire(&state, Operation::Scan)?;

    let (evidence_root, registry_db, intelligence_db) = {
        let config_guard = state
            .config
            .read()
            .map_err(|e| format!("Failed to read config: {}", e))?;
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

    // Audit: record the scan outcome.
    if let Ok(db) = require_db(&state) {
        let _ = db.log_audit("registry_scan", &format!("files_found={result}"), None);
    }

    Ok(result)
}

#[tauri::command]
pub fn get_unprocessed_files(
    state: State<AppState>,
    limit: i64,
) -> Result<Vec<core::RegistryEntry>, String> {
    require_db(&state)?
        .get_unprocessed_files(limit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn mark_processed(state: State<AppState>, fingerprint: String) -> Result<(), String> {
    require_db(&state)?
        .mark_processed(&fingerprint)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_extraction_queue(
    state: State<AppState>,
    limit: i64,
) -> Result<Vec<core::RegistryEntry>, String> {
    require_db(&state)?
        .get_extraction_queue(limit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_analysis_queue(
    state: State<AppState>,
    limit: i64,
) -> Result<Vec<core::RegistryEntry>, String> {
    require_db(&state)?
        .get_analysis_queue(limit)
        .map_err(|e| e.to_string())
}

/// FR-META: Return all registry files ordered by name.
/// Used by the metadata viewer page to populate the file list.
#[tauri::command]
pub fn get_registry_files(
    state: State<AppState>,
    limit: i64,
) -> Result<Vec<core::RegistryEntry>, String> {
    require_db(&state)?
        .get_all_registry_files(limit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_supported_extensions() -> Vec<String> {
    extractors::Deconstructor::supported_extensions()
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}
