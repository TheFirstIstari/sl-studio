use crate::commands::require_db;
use crate::config::{AppConfig, ValidationResult};
use crate::utils;
use crate::AppState;
use tauri::State;
use tracing::info;

#[tauri::command]
pub fn load_config(state: State<AppState>) -> Result<AppConfig, String> {
    let guard = state
        .config
        .read()
        .map_err(|e| format!("Failed to read config: {}", e))?;
    let config = guard.clone();
    info!("Config loaded");
    Ok(config)
}

#[tauri::command]
pub fn save_config(state: State<AppState>, config: AppConfig) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())?;
    let mut guard = state
        .config
        .write()
        .map_err(|e| format!("Failed to write config: {}", e))?;
    *guard = config;
    info!("Config saved");
    // Audit: best-effort — DB may not be open yet during first-run setup.
    if let Ok(db) = require_db(&state) {
        let _ = db.log_audit("config_saved", "", None);
    }
    Ok(())
}

#[tauri::command]
pub fn validate_config(config: AppConfig) -> ValidationResult {
    config.validate()
}

#[tauri::command]
pub fn get_app_data_dir() -> String {
    utils::app_data_dir().to_string_lossy().to_string()
}

#[tauri::command]
pub fn get_models_dir() -> String {
    if crate::IS_DEV {
        utils::dev_models_dir().to_string_lossy().to_string()
    } else {
        utils::models_dir().to_string_lossy().to_string()
    }
}
