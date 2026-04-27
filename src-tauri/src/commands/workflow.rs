use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub registry_count: i64,
    pub intelligence_count: i64,
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

#[tauri::command]
pub fn get_stats(state: State<AppState>) -> Result<Stats, String> {
    let db_opt = {
        let guard = state
            .db
            .lock()
            .map_err(|e| format!("Failed to lock database: {}", e))?;
        guard.as_ref().cloned()
    };
    if let Some(db) = db_opt {
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
pub fn update_processing_state(
    state: State<AppState>,
    is_scanning: Option<bool>,
    is_extracting: Option<bool>,
    is_analyzing: Option<bool>,
    progress: Option<f32>,
    current_file: Option<String>,
    processed: Option<i64>,
    total: Option<i64>,
) -> Result<(), String> {
    let mut proc = state.processing.lock().map_err(|e| e.to_string())?;
    if let Some(v) = is_scanning {
        proc.is_scanning = v;
    }
    if let Some(v) = is_extracting {
        proc.is_extracting = v;
    }
    if let Some(v) = is_analyzing {
        proc.is_analyzing = v;
    }
    if let Some(v) = progress {
        if proc.is_scanning {
            proc.scan_progress = v;
        }
        if proc.is_extracting {
            proc.extract_progress = v;
        }
        if proc.is_analyzing {
            proc.analyze_progress = v;
        }
    }
    if let Some(v) = current_file {
        proc.current_file = v;
    }
    if let Some(v) = processed {
        proc.processed_count = v;
    }
    if let Some(v) = total {
        proc.total_count = v;
    }
    Ok(())
}

#[tauri::command]
pub fn get_workflow_state(state: State<AppState>) -> Result<WorkflowState, String> {
    let db_opt = {
        let guard = state
            .db
            .lock()
            .map_err(|e| format!("Failed to lock database: {}", e))?;
        guard.as_ref().cloned()
    };
    let (db_state, processing) = {
        let proc_guard = state
            .processing
            .lock()
            .map_err(|e| format!("Failed to lock processing: {}", e))?;
        (
            db_opt.as_ref().map(|db| db.get_workflow_state()),
            proc_guard.clone(),
        )
    };

    let mut workflow = match db_state {
        Some(Ok(w)) => w,
        _ => WorkflowState {
            files_scanned: 0,
            files_extracted: 0,
            files_analyzed: 0,
            last_scan_time: None,
            last_extraction_time: None,
            last_analysis_time: None,
            current_stage: "none".to_string(),
            is_scanning: false,
            is_extracting: false,
            is_analyzing: false,
            scan_progress: 0.0,
            extract_progress: 0.0,
            analyze_progress: 0.0,
            current_file: String::new(),
            processed_count: 0,
            total_count: 0,
        },
    };

    workflow.is_scanning = processing.is_scanning;
    workflow.is_extracting = processing.is_extracting;
    workflow.is_analyzing = processing.is_analyzing;
    workflow.scan_progress = processing.scan_progress;
    workflow.extract_progress = processing.extract_progress;
    workflow.analyze_progress = processing.analyze_progress;
    workflow.current_file = processing.current_file;
    workflow.processed_count = processing.processed_count;
    workflow.total_count = processing.total_count;

    Ok(workflow)
}
