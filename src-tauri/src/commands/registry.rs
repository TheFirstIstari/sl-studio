use crate::core::{self, RegistryProgress, RegistryWorker};
use crate::extractors;
use crate::AppState;
use tauri::{AppHandle, Emitter, State};
use tracing::info;

#[tauri::command]
pub async fn start_registry(app: AppHandle, state: State<'_, AppState>) -> Result<usize, String> {
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
pub fn get_unprocessed_files(
    state: State<AppState>,
    limit: i64,
) -> Result<Vec<core::RegistryEntry>, String> {
    let db = state
        .db
        .lock()
        .map_err(|e| format!("Database mutex poisoned: {e}"))?;
    if let Some(db) = db.as_ref() {
        db.get_unprocessed_files(limit).map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
pub fn mark_processed(state: State<AppState>, fingerprint: String) -> Result<(), String> {
    let db = state
        .db
        .lock()
        .map_err(|e| format!("Database mutex poisoned: {e}"))?;
    if let Some(db) = db.as_ref() {
        db.mark_processed(&fingerprint).map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
pub fn get_extraction_queue(
    state: State<AppState>,
    limit: i64,
) -> Result<Vec<core::RegistryEntry>, String> {
    let db = {
        let guard = state
            .db
            .lock()
            .map_err(|e| format!("Database mutex poisoned: {e}"))?;
        guard.as_ref().ok_or("Database not initialized")?.clone()
    };
    db.get_extraction_queue(limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_analysis_queue(
    state: State<AppState>,
    limit: i64,
) -> Result<Vec<core::RegistryEntry>, String> {
    let db = {
        let guard = state
            .db
            .lock()
            .map_err(|e| format!("Database mutex poisoned: {e}"))?;
        guard.as_ref().ok_or("Database not initialized")?.clone()
    };
    db.get_analysis_queue(limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_supported_extensions() -> Vec<String> {
    extractors::Deconstructor::supported_extensions()
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}
