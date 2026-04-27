use crate::config::{AppConfig, ProjectFile};
use crate::core::{Database, RegistryWorker};
use crate::utils;
use crate::AppState;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tracing::info;

#[tauri::command]
pub async fn init_project(
    app: AppHandle,
    state: State<'_, AppState>,
    config: AppConfig,
) -> Result<bool, String> {
    {
        let config_guard = state
            .config
            .lock()
            .map_err(|e| format!("Failed to lock config: {}", e))?;
        let db_guard = state
            .db
            .lock()
            .map_err(|e| format!("Failed to lock database: {}", e))?;

        if config_guard.project.name == config.project.name && db_guard.is_some() {
            info!("Project already initialized: {}", config_guard.project.name);
            return Ok(true);
        }
    }

    info!("Initializing project: {}", config.project.name);

    utils::ensure_app_dirs().map_err(|e| e.to_string())?;

    let db = Database::new(&config.project.registry_db, &config.project.intelligence_db)
        .map_err(|e| e.to_string())?;

    {
        let mut db_guard = state
            .db
            .lock()
            .map_err(|e| format!("Failed to lock database: {}", e))?;
        *db_guard = Some(Arc::new(db));
    }

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
pub fn create_project(path: String) -> Result<ProjectFile, String> {
    let project = ProjectFile::default();
    let file_path = std::path::Path::new(&path);
    project.save(file_path).map_err(|e| e.to_string())?;
    info!("Created new project: {}", path);
    Ok(project)
}

#[tauri::command]
pub fn load_project(path: String) -> Result<ProjectFile, String> {
    let file_path = std::path::Path::new(&path);
    ProjectFile::load(file_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_project(path: String, project: ProjectFile) -> Result<(), String> {
    let file_path = std::path::Path::new(&path);
    let mut proj = project;
    proj.update_modified();
    proj.save(file_path).map_err(|e| e.to_string())?;
    info!("Saved project: {}", path);
    Ok(())
}

#[tauri::command]
pub fn get_default_project() -> ProjectFile {
    ProjectFile::default()
}
