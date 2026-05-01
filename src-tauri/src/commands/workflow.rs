use crate::commands::require_db;
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

/// One-at-a-time gate for the three long-running operations (scan,
/// extract, analyze). The user workflow is confusing if two of these
/// run concurrently — and they also fight for CPU, database writers,
/// and the cancel flag. The guard:
///
///   1. Refuses to acquire if any of the flags is already set.
///   2. Sets the caller's flag on construction.
///   3. Clears it (and progress state) on drop, even on panic or early
///      return. This is strictly more robust than asking the frontend
///      to call `update_processing_state` before and after.
///
/// Usage inside a command:
///
///     let _guard = BusyGuard::acquire(&state, Operation::Scan)?;
///     // ... long-running work ...
///     // guard drops here, clearing state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Scan,
    Extract,
    Analyze,
}

impl Operation {
    fn label(self) -> &'static str {
        match self {
            Operation::Scan => "Scanning",
            Operation::Extract => "Extracting",
            Operation::Analyze => "Analyzing",
        }
    }
}

pub struct BusyGuard {
    state: Arc<std::sync::Mutex<crate::ProcessingState>>,
    op: Operation,
}

impl BusyGuard {
    /// Acquire the exclusive slot. Returns an error whose message
    /// names the active operation so the frontend can surface it.
    pub fn acquire(state: &State<AppState>, op: Operation) -> Result<Self, String> {
        let mut proc = state.processing.lock().map_err(|e| e.to_string())?;
        if proc.is_scanning || proc.is_extracting || proc.is_analyzing {
            let active = if proc.is_scanning {
                Operation::Scan
            } else if proc.is_extracting {
                Operation::Extract
            } else {
                Operation::Analyze
            };
            return Err(format!(
                "Cannot start {}: {} is already in progress. Wait for it to finish or cancel it first.",
                op.label().to_lowercase(),
                active.label().to_lowercase()
            ));
        }
        match op {
            Operation::Scan => proc.is_scanning = true,
            Operation::Extract => proc.is_extracting = true,
            Operation::Analyze => proc.is_analyzing = true,
        }
        // Reset progress counters for the fresh run.
        proc.scan_progress = 0.0;
        proc.extract_progress = 0.0;
        proc.analyze_progress = 0.0;
        proc.processed_count = 0;
        proc.total_count = 0;
        proc.current_file.clear();
        drop(proc);

        Ok(Self {
            state: state.processing_arc(),
            op,
        })
    }
}

impl Drop for BusyGuard {
    fn drop(&mut self) {
        if let Ok(mut proc) = self.state.lock() {
            match self.op {
                Operation::Scan => proc.is_scanning = false,
                Operation::Extract => proc.is_extracting = false,
                Operation::Analyze => proc.is_analyzing = false,
            }
            proc.current_file.clear();
        }
    }
}

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
    let Ok(db) = require_db(&state) else {
        return Ok(Stats {
            registry_count: 0,
            intelligence_count: 0,
        });
    };
    Ok(Stats {
        registry_count: db.get_registry_count().unwrap_or(0),
        intelligence_count: db.get_intelligence_count().unwrap_or(0),
    })
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
    let db_opt = require_db(&state).ok();
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
