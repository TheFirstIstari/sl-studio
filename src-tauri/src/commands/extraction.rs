use crate::commands::require_db;
use crate::commands::workflow::{BusyGuard, Operation};
use crate::extractors;
use crate::get_or_create_thread_pool;
use crate::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, State};
use tracing::{error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionStats {
    pub total_files: i64,
    pub total_characters: i64,
    pub average_characters: f64,
    pub average_quality: f64,
    pub partial_count: i64,
    pub files_by_type: HashMap<String, i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub fingerprint: String,
    pub path: String,
    pub success: bool,
    pub char_count: usize,
    pub error: Option<String>,
    pub quality: Option<f64>,
    #[serde(skip)]
    pub extraction_text: Option<String>,
    #[serde(skip)]
    pub is_partial: bool,
}

impl Default for ExtractionResult {
    fn default() -> Self {
        Self {
            fingerprint: String::new(),
            path: String::new(),
            success: false,
            char_count: 0,
            error: None,
            quality: None,
            extraction_text: None,
            is_partial: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionProgress {
    pub total: usize,
    pub processed: usize,
    pub current_file: String,
    pub phase: String,
    pub success_count: usize,
    pub error_count: usize,
}

#[tauri::command]
pub fn get_extraction_statistics(state: State<AppState>) -> Result<ExtractionStats, String> {
    let Ok(db) = require_db(&state) else {
        return Ok(ExtractionStats {
            total_files: 0,
            total_characters: 0,
            average_characters: 0.0,
            average_quality: 0.0,
            partial_count: 0,
            files_by_type: HashMap::new(),
        });
    };
    let stats = db.get_extraction_statistics().map_err(|e| e.to_string())?;
    Ok(ExtractionStats {
        total_files: stats.total_files,
        total_characters: stats.total_characters,
        average_characters: stats.average_characters,
        average_quality: stats.average_quality,
        partial_count: stats.partial_count,
        files_by_type: stats.files_by_type,
    })
}

#[tauri::command]
pub fn extract_file(path: String) -> Result<extractors::ExtractionResult, String> {
    use extractors::{Deconstructor, ExtractorConfig};

    let config = ExtractorConfig::default();
    let deconstructor = Deconstructor::new(config).map_err(|e| e.to_string())?;

    let path = std::path::Path::new(&path);
    deconstructor.extract(path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn extract_batch(
    app: AppHandle,
    state: State<'_, AppState>,
    fingerprints: Vec<String>,
    cpu_workers: Option<u32>,
) -> Result<Vec<ExtractionResult>, String> {
    use extractors::{Deconstructor, ExtractorConfig};
    use rayon::prelude::*;

    // Mutual-exclusion gate: only one of scan/extract/analyze may run
    // at a time. Guard auto-clears on drop.
    let _guard = BusyGuard::acquire(&state, Operation::Extract)?;

    let workers = {
        if let Some(w) = cpu_workers {
            w as usize
        } else {
            state
                .config
                .read()
                .map_err(|e| format!("Config lock poisoned: {e}"))?
                .get_effective_workers() as usize
        }
    };

    if state.cancel_flag.load(Ordering::SeqCst) {
        return Err("Extraction cancelled by user".to_string());
    }

    info!(
        "Extracting batch of {} files with {} workers",
        fingerprints.len(),
        workers
    );

    let pool = get_or_create_thread_pool(workers);
    info!("Using thread pool with {} workers", workers);

    // Checkpoint: open a job record so the batch is resumable and visible
    // in the audit trail.  Errors here are non-fatal.
    let job_id = format!("extract_{}", Utc::now().timestamp_millis());

    let total = fingerprints.len();

    let (file_data, cached_results): (Vec<(String, String)>, Vec<ExtractionResult>) = {
        let db = require_db(&state)?;
        let mut to_extract = Vec::new();
        let mut cached = Vec::new();
        for fingerprint in &fingerprints {
            let Ok(entry) = db.get_registry_entry(fingerprint) else {
                continue;
            };
            if let Ok(Some(text)) = db.get_extracted_text(fingerprint) {
                if !text.is_empty() {
                    cached.push(ExtractionResult {
                        fingerprint: fingerprint.clone(),
                        path: entry.path.clone(),
                        success: true,
                        char_count: text.chars().count(),
                        error: None,
                        quality: Some(1.0),
                        extraction_text: Some(text),
                        is_partial: false,
                    });
                    continue;
                }
            }
            to_extract.push((fingerprint.clone(), entry.path));
        }
        (to_extract, cached)
    };
    let cache_hits = cached_results.len();
    if cache_hits > 0 {
        info!(
            "extract_batch: {} cache hit(s); extracting {} remaining file(s)",
            cache_hits,
            file_data.len()
        );
    }

    // Record checkpoint start (best-effort).
    if let Ok(db) = require_db(&state) {
        let _ = db.checkpoint_start("extract_batch", &job_id);
    }

    let deconstructor = {
        let config = ExtractorConfig::default();
        Deconstructor::new(config).map_err(|e| format!("Failed to create Deconstructor: {}", e))?
    };

    let results: Vec<ExtractionResult> = tokio::task::spawn_blocking(move || {
        pool.install(|| {
            file_data
                .par_iter()
                .filter_map(|(fingerprint, path)| {
                    let file_path = std::path::Path::new(path);
                    if !file_path.exists() {
                        return Some(ExtractionResult {
                            fingerprint: fingerprint.clone(),
                            path: path.clone(),
                            success: false,
                            char_count: 0,
                            error: Some("File not found".to_string()),
                            quality: None,
                            extraction_text: None,
                            is_partial: false,
                        });
                    }

                    match deconstructor.extract(file_path) {
                        Ok(extraction) => Some(ExtractionResult {
                            fingerprint: fingerprint.clone(),
                            path: path.clone(),
                            success: true,
                            char_count: extraction.char_count,
                            error: None,
                            quality: Some(extraction.quality_score),
                            extraction_text: Some(extraction.text),
                            is_partial: extraction.is_partial,
                        }),
                        Err(e) => {
                            error!("Extraction failed for {}: {}", path, e);
                            Some(ExtractionResult {
                                fingerprint: fingerprint.clone(),
                                path: path.clone(),
                                success: false,
                                char_count: 0,
                                error: Some(e.to_string()),
                                quality: None,
                                extraction_text: None,
                                is_partial: false,
                            })
                        }
                    }
                })
                .collect()
        })
    })
    .await
    .map_err(|e| format!("Extraction task failed: {e}"))?;

    let mut all_results: Vec<ExtractionResult> = cached_results;
    all_results.extend(results);

    {
        let db = require_db(&state)?;
        let mut saved = 0i64;
        for result in all_results.iter().skip(cache_hits) {
            if !result.success {
                // Push failed extractions to the error queue for later review.
                if let Some(ref err) = result.error {
                    let _ = db.push_error(
                        &result.fingerprint,
                        "extract_batch",
                        err,
                        Some(&result.path),
                    );
                }
                continue;
            }
            if let Some(ref text) = result.extraction_text {
                let _ = db.save_text_cache(
                    &result.fingerprint,
                    &result.path,
                    text,
                    &result.fingerprint,
                    0,
                    result.quality.unwrap_or(0.0),
                );
                let _ = db.mark_extracted(&result.fingerprint, result.is_partial);
                saved += 1;
                // Update checkpoint every 10 files so progress survives a crash.
                if saved % 10 == 0 {
                    let _ = db.checkpoint_update(&job_id, &result.fingerprint, saved);
                }
            }
        }
        // Mark job complete.
        let _ = db.checkpoint_complete(&job_id);
    }

    let mut success_count = 0;
    let mut error_count = 0;
    let processed = all_results.len();

    for result in &all_results {
        if result.success {
            success_count += 1;
        } else {
            error_count += 1;
        }
    }

    let progress = ExtractionProgress {
        total,
        processed,
        current_file: String::new(),
        phase: "Complete".to_string(),
        success_count,
        error_count,
    };
    app.emit("extraction_progress", progress).ok();

    info!(
        "Extraction complete: {}/{} successful ({} from cache)",
        success_count,
        all_results.len(),
        cache_hits
    );

    // Audit: record batch extraction summary.
    if let Ok(db) = require_db(&state) {
        let _ = db.log_audit(
            "extract_batch",
            &format!(
                "total={},success={},errors={},cache_hits={}",
                all_results.len(),
                success_count,
                error_count,
                cache_hits
            ),
            None,
        );
    }

    Ok(all_results)
}
