use crate::commands::require_db;
use crate::commands::workflow::{BusyGuard, Operation};
use crate::core::IntelligenceEntry;
use crate::inference::{self, Reasoner, ReasonerConfig};
use crate::AppState;
use chrono::Utc;
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
        .write()
        .map_err(|e| format!("Reasoner lock poisoned: {e}"))?;
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
            .read()
            .map_err(|e| format!("Reasoner lock poisoned: {e}"))?;
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
    let Ok(cached) = state.reasoner.read() else {
        return false;
    };
    cached
        .as_ref()
        .map(|r| r.is_model_loaded())
        .unwrap_or(false)
}

#[tauri::command]
pub fn get_reasoner_config(state: State<AppState>) -> Option<ReasonerConfig> {
    let cached = state.reasoner.read().ok()?;
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
    // Mutual-exclusion gate: only one of scan/extract/analyze may run
    // at a time. Guard auto-clears on drop.
    let _guard = BusyGuard::acquire(&state, Operation::Analyze)?;

    let reasoner_arc = {
        let cached = state
            .reasoner
            .read()
            .map_err(|e| format!("Reasoner lock poisoned: {e}"))?;
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

    let db = require_db(&state)?;
    let mut results = Vec::new();
    let mut processed = 0;

    // Checkpoint: open a job record for this analysis batch.
    let job_id = format!("analyze_{}", Utc::now().timestamp_millis());
    let _ = db.checkpoint_start("analyze_batch", &job_id);

    for fingerprint in &fingerprints {
        if state.cancel_flag.load(Ordering::SeqCst) {
            info!("Analysis cancelled by user");
            break;
        }

        let entry = match db.get_registry_entry(fingerprint) {
            Ok(e) => e,
            Err(e) => {
                error!("Registry lookup failed for {}: {}", fingerprint, e);
                let _ = db.push_error(fingerprint, "analyze_batch", &e.to_string(), None);
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

        // FR-LANG: detect language once per file; tag every fact below.
        let detected_language: Option<String> =
            crate::extractors::language::detect_language(&text).map(String::from);

        // Run the LLM analysis on a blocking thread so the tokio runtime
        // stays free to service IPC traffic (cancel, progress polling).
        // Reasoner is already an Arc; cheap to clone into the closure.
        let reasoner_clone = reasoner.clone();
        let fp = fingerprint.clone();
        let fname = entry.file_name.clone();
        let analyze_result =
            tokio::task::spawn_blocking(move || reasoner_clone.analyze_text(&fp, &fname, &text))
                .await
                .map_err(|e| format!("Analysis task failed: {e}"))?;

        match analyze_result {
            Ok(result) => {
                // Build all IntelligenceEntry rows up front, then insert
                // them in a single transaction. Saves N pool checkouts +
                // N autocommits for a typical 10–50 facts per file.
                let entries: Vec<IntelligenceEntry> = result
                    .facts
                    .iter()
                    .map(|fact| IntelligenceEntry {
                        id: 0,
                        registry_id: entry.id,
                        fingerprint: fingerprint.clone(),
                        filename: entry.file_name.clone(),
                        source_quote: fact.source_quote.clone(),
                        page_number: None,
                        evidence_full: None,
                        evidence_hash: None,
                        associated_date: fact.date.clone(),
                        location: fact.location.clone(),
                        people: if fact.people.is_empty() {
                            None
                        } else {
                            Some(fact.people.join(", "))
                        },
                        fact_summary: fact.summary.clone(),
                        category: Some(fact.category.clone()),
                        identified_crime: fact.identified_crime.clone(),
                        severity_score: fact.severity,
                        confidence: Some(fact.confidence as f64),
                        quality_score: Some(result.quality_score as f64),
                        source_language: detected_language.clone(),
                        translated_quote: None,
                        pipeline_id: None,
                        pass_name: None,
                        is_deleted: false,
                        deleted_at: None,
                        processing_time_ms: None,
                        created_at: None,
                    })
                    .collect();

                if let Err(e) = db.insert_intelligence_batch(&entries) {
                    error!("Failed to save facts for {}: {}", fingerprint, e);
                }

                let _ = db.mark_processed(fingerprint);
                info!(
                    "Saved {} facts from {}",
                    result.facts.len(),
                    entry.file_name
                );
                // Periodic checkpoint update every 5 files.
                if (processed + 1) % 5 == 0 {
                    let _ = db.checkpoint_update(&job_id, fingerprint, (processed + 1) as i64);
                }
                results.push(result);
            }
            Err(e) => {
                error!("Analysis failed for {}: {}", fingerprint, e);
                let _ = db.push_error(fingerprint, "analyze_batch", &e.to_string(), None);
            }
        }

        processed += 1;
    }

    // Complete the checkpoint.
    let _ = db.checkpoint_complete(&job_id);

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

    let facts_saved: usize = results.iter().map(|r| r.facts.len()).sum();
    info!(
        "Analysis complete: {} files processed, {} facts saved",
        results.len(),
        facts_saved
    );

    // Audit: record batch analysis summary.
    let _ = db.log_audit(
        "analyze_batch",
        &format!("files={},facts_saved={}", results.len(), facts_saved),
        None,
    );

    Ok(results)
}
