use crate::core::IntelligenceEntry;
use crate::inference::{self, Reasoner, ReasonerConfig};
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tracing::{error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisProgress {
    pub total: usize,
    pub processed: usize,
    pub current_file: String,
    pub phase: String,
}

#[tauri::command]
pub fn init_reasoner(
    state: State<AppState>,
    model_path: String,
    context_size: u32,
    gpu_layers: Option<i32>,
) -> Result<bool, String> {
    #[allow(clippy::unnecessary_lazy_evaluations)]
    let actual_gpu_layers = gpu_layers.unwrap_or({
        #[cfg(target_os = "macos")]
        {
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

    let mut cached = state
        .reasoner
        .lock()
        .map_err(|e| format!("Reasoner mutex poisoned: {e}"))?;
    *cached = Some(Arc::new(reasoner));

    info!("Reasoner initialized and cached");
    Ok(true)
}

#[tauri::command]
pub fn analyze_file(
    state: State<AppState>,
    path: String,
) -> Result<inference::AnalysisResult, String> {
    let reasoner_arc = {
        let cached = state
            .reasoner
            .lock()
            .map_err(|e| format!("Reasoner mutex poisoned: {e}"))?;
        cached.clone()
    };

    let reasoner = reasoner_arc.ok_or("Reasoner not initialized. Call init_reasoner first.")?;

    let file_path = std::path::Path::new(&path);
    reasoner.analyze_file(file_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn validate_model(_state: State<AppState>, model_path: String) -> Result<bool, String> {
    let config = ReasonerConfig {
        model_path: model_path.clone(),
        context_size: 2048,
        gpu_layers: 0,
        temperature: 0.1,
        ..Default::default()
    };

    match Reasoner::new(config) {
        Ok(_reasoner) => {
            info!("Model validation successful: {}", model_path);
            Ok(true)
        }
        Err(e) => {
            error!("Model validation failed for {}: {}", model_path, e);
            Err(format!("Model not supported: {}", e))
        }
    }
}

#[tauri::command]
pub fn is_model_loaded(state: State<AppState>) -> bool {
    let Ok(cached) = state.reasoner.lock() else {
        return false;
    };
    cached
        .as_ref()
        .map(|r| r.is_model_loaded())
        .unwrap_or(false)
}

#[tauri::command]
pub fn get_reasoner_config(state: State<AppState>) -> Option<ReasonerConfig> {
    let cached = state.reasoner.lock().ok()?;
    cached.as_ref().map(|r| r.get_config())
}

#[tauri::command]
pub fn set_cancel_flag(state: State<AppState>, cancel: bool) -> bool {
    state.cancel_flag.store(cancel, Ordering::SeqCst);
    info!("Cancel flag set to: {}", cancel);
    cancel
}

#[tauri::command]
pub fn get_cancel_flag(state: State<AppState>) -> bool {
    state.cancel_flag.load(Ordering::SeqCst)
}

#[tauri::command]
pub async fn analyze_batch(
    app: AppHandle,
    state: State<'_, AppState>,
    fingerprints: Vec<String>,
) -> Result<Vec<inference::AnalysisResult>, String> {
    let reasoner_arc = {
        let cached = state
            .reasoner
            .lock()
            .map_err(|e| format!("Reasoner mutex poisoned: {e}"))?;
        cached.clone()
    };

    let reasoner = reasoner_arc.ok_or("Reasoner not initialized. Call init_reasoner first.")?;

    let total = fingerprints.len();
    app.emit(
        "analysis_progress",
        AnalysisProgress {
            total,
            processed: 0,
            current_file: "Starting analysis...".to_string(),
            phase: "Initializing".to_string(),
        },
    )
    .ok();

    let db = {
        let guard = state
            .db
            .lock()
            .map_err(|e| format!("Database mutex poisoned: {e}"))?;
        guard.as_ref().ok_or("Database not initialized")?.clone()
    };
    let mut results = Vec::new();
    let mut processed = 0;

    for fingerprint in &fingerprints {
        if state.cancel_flag.load(Ordering::SeqCst) {
            info!("Analysis cancelled by user");
            break;
        }

        let entry = match db.get_registry_entry(fingerprint) {
            Ok(e) => e,
            Err(e) => {
                error!("Registry lookup failed for {}: {}", fingerprint, e);
                continue;
            }
        };

        app.emit(
            "analysis_progress",
            AnalysisProgress {
                total,
                processed,
                current_file: entry.file_name.clone(),
                phase: "Analyzing".to_string(),
            },
        )
        .ok();

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

        match reasoner.analyze_text(fingerprint, &entry.file_name, &text) {
            Ok(result) => {
                for fact in &result.facts {
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

                let _ = db.mark_processed(fingerprint);
                info!(
                    "Saved {} facts from {}",
                    result.facts.len(),
                    entry.file_name
                );
                results.push(result);
            }
            Err(e) => {
                error!("Analysis failed for {}: {}", fingerprint, e);
            }
        }

        processed += 1;
    }

    app.emit(
        "analysis_progress",
        AnalysisProgress {
            total,
            processed,
            current_file: String::new(),
            phase: "Complete".to_string(),
        },
    )
    .ok();

    info!("Analysis complete: {} files processed", results.len());
    Ok(results)
}
