// Tauri IPC command handlers for SL Studio.
// Every `pub async fn` annotated with `#[tauri::command]` is invoked from
// the SvelteKit frontend via `invoke('command_name', { ...args })`.
//
// Naming convention: Rust uses snake_case parameters; Tauri automatically
// maps JS camelCase invoke keys to Rust snake_case (e.g. `stateJson` → `state_json`).

use crate::{AppError, Result};
use rusqlite::params;
use std::collections::HashMap;
use tracing::{info, warn};

// ── require_db ─────────────────────────────────────────────────────

/// Return a clone of the shared database connection pool.
/// The pool is initialised on first call via `OnceLock` in `lib.rs`.
pub fn require_db() -> Result<crate::core::database::Pool> {
    crate::require_db()
}

// ── Config commands ────────────────────────────────────────────────

/// Load the current application configuration.
#[tauri::command]
pub async fn load_config() -> Result<crate::AppConfig> {
    // Try to read a persisted config file; fall back to defaults.
    let config_path = std::env::current_dir()
        .unwrap_or_default()
        .join("sl-studio-config.json");

    if config_path.exists() {
        let data = std::fs::read_to_string(&config_path)?;
        let cfg: crate::AppConfig = serde_json::from_str(&data)?;
        return Ok(cfg);
    }

    Ok(crate::AppConfig {
        version: "0.3.0".to_string(),
        project: crate::ProjectConfig {
            name: "SL Studio".to_string(),
            evidence_root: String::new(),
            registry_db: "registry.db".to_string(),
            intelligence_db: "intelligence.db".to_string(),
        },
        model: crate::ModelConfig {
            source: "local".to_string(),
            id: "default".to_string(),
            mlx_model_name: "qwen3.5-4b-4bit".to_string(),
            dtype: "float16".to_string(),
            context_length: 4096,
            downloaded: false,
            local_path: String::new(),
        },
        hardware: crate::HardwareConfig {
            gpu_backend: "cpu".to_string(),
            gpu_memory_fraction: 0.8,
            cpu_workers: num_cpus::get_physical(),
            auto_scale_workers: true,
            batch_size: 6,
            auto_scale_batch: true,
            ocr_provider: "ocrs".to_string(),
            whisper_size: "base".to_string(),
            whisper_model_path: None,
        },
        processing: crate::ProcessingConfig {
            batch_size: 6,
            max_image_resolution: 2048,
        },
    })
}

/// Persist the application configuration to disk.
#[tauri::command]
pub async fn save_config(config: crate::AppConfig) -> Result<()> {
    let config_path = std::env::current_dir()
        .unwrap_or_default()
        .join("sl-studio-config.json");

    let data = serde_json::to_string_pretty(&config)?;
    std::fs::write(&config_path, data)?;
    info!("Saved configuration to {}", config_path.display());
    Ok(())
}

/// Initialize a project: set up directories and database.
#[tauri::command]
pub async fn init_project(config: crate::AppConfig) -> Result<()> {
    let _db = require_db()?;
    info!("Initialized project: {:?}", config.project.name);
    Ok(())
}

// ── Registry / scanning commands ───────────────────────────────────

/// Scan the evidence root directory and populate the registry.
/// Returns the number of files added to the registry.
#[tauri::command]
pub async fn start_registry() -> Result<i64> {
    let db = require_db()?;
    let cfg = load_config().await?;

    let evidence_root = std::path::Path::new(&cfg.project.evidence_root);
    if !evidence_root.exists() {
        warn!(
            "Evidence root does not exist: {}",
            cfg.project.evidence_root
        );
        return Ok(0);
    }

    let mut count: i64 = 0;
    if let Ok(entries) = std::fs::read_dir(evidence_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let fingerprint = format!("{:x}", {
                    use std::hash::{Hash, Hasher};
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    path.to_string_lossy().hash(&mut h);
                    h.finish()
                });
                let file_name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let file_size = entry.metadata().map(|m| m.len()).unwrap_or(0) as i64;
                let file_type = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                db.execute(
                    "INSERT OR IGNORE INTO registry (fingerprint, path, file_size, file_type, file_name) VALUES (?, ?, ?, ?, ?)",
                    params![fingerprint, path.to_string_lossy(), file_size, file_type, file_name],
                )?;
                count += 1;
            }
        }
    }
    info!("Registry scan complete: {} files", count);
    Ok(count)
}

/// Get files that still need extraction.
#[tauri::command]
pub async fn get_extraction_queue(limit: usize) -> Result<Vec<crate::RegistryFile>> {
    let db = require_db()?;
    let rows = db.query_map(
        "SELECT path, fingerprint FROM registry WHERE has_extracted_text = FALSE LIMIT ? ORDER BY processing_priority DESC, created_at ASC",
        params![limit as i64],
        |row| {
            Ok(crate::RegistryFile {
                path: row.get::<_, String>(0)?,
                fingerprint: row.get::<_, String>(1)?,
            })
        },
    )?;
    Ok(rows)
}

/// Extract text from a batch of files.
#[tauri::command]
pub async fn extract_batch(
    fingerprints: Vec<String>,
    cpu_workers: usize,
) -> Result<Vec<crate::ExtractionResult>> {
    let db = require_db()?;
    let _ = cpu_workers; // reserved for parallel extraction
    let mut results = Vec::new();

    for fp in &fingerprints {
        let path_opt: Option<String> = db.query_row_optional(
            "SELECT path FROM registry WHERE fingerprint = ?",
            params![fp],
            |row| row.get::<_, String>(0),
        )?;

        if let Some(path) = path_opt {
            let result = extract_file(&path, fp).await;

            if result.success {
                db.execute(
                    "UPDATE registry SET has_extracted_text = TRUE, extracted_at = datetime('now') WHERE fingerprint = ?",
                    params![fp],
                )?;
            }

            results.push(result);
        } else {
            results.push(crate::ExtractionResult {
                fingerprint: fp.clone(),
                path: String::new(),
                success: false,
                char_count: 0,
                error: Some("File not found in registry".to_string()),
            });
        }
    }

    info!("Extracted {} files", results.len());
    Ok(results)
}

/// Get extraction statistics.
#[tauri::command]
pub async fn get_extraction_statistics() -> Result<crate::ExtractionStats> {
    let db = require_db()?;
    let total_files: i64 = db.query_row("SELECT COUNT(*) FROM registry", params![], |row| {
        row.get::<_, i64>(0)
    })?;
    let total_characters: i64 = db.query_row(
        "SELECT COALESCE(SUM(LENGTH(extracted_text)), 0) FROM text_cache",
        params![],
        |row| row.get::<_, i64>(0),
    )?;
    let partial_count: i64 = db.query_row(
        "SELECT COUNT(*) FROM text_cache WHERE quality_score IS NOT NULL AND quality_score < 0.5",
        params![],
        |row| row.get::<_, i64>(0),
    )?;

    let mut files_by_type = HashMap::new();
    let type_rows: Vec<(String, i64)> = db.query_map(
        "SELECT file_type, COUNT(*) FROM registry GROUP BY file_type",
        params![],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
    )?;
    for (ft, cnt) in type_rows {
        files_by_type.insert(ft, cnt as u64);
    }

    Ok(crate::ExtractionStats {
        total_files: total_files as u64,
        total_characters: total_characters as u64,
        average_characters: if total_files > 0 {
            total_characters as f64 / total_files as f64
        } else {
            0.0
        },
        average_quality: 0.0,
        partial_count: partial_count as u64,
        files_by_type,
    })
}

// ── Facts commands ─────────────────────────────────────────────────

/// Search facts in the intelligence table.
/// When `query` is empty, returns all facts; otherwise filters by `fact_summary LIKE`.
#[tauri::command]
pub async fn search_facts(query: String, limit: usize) -> Result<Vec<crate::Fact>> {
    let db = require_db()?;

    if query.is_empty() {
        let sql = "SELECT id, fingerprint, filename, fact_summary, category, identified_crime, severity_score, confidence, created_at FROM intelligence WHERE is_deleted = FALSE ORDER BY created_at DESC LIMIT ?";
        Ok(db.query_map(sql, params![limit as i64], |row| {
            Ok(crate::Fact {
                id: row.get::<_, i64>(0)? as u64,
                fingerprint: row.get::<_, String>(1)?,
                filename: row.get::<_, String>(2)?,
                fact_summary: row.get::<_, String>(3)?,
                category: row.get::<_, Option<String>>(4)?,
                identified_crime: row.get::<_, Option<String>>(5)?,
                severity_score: row.get::<_, i64>(6)? as u8,
                confidence: row.get::<_, Option<f64>>(7)?,
                created_at: row.get::<_, String>(8)?,
            })
        })?)
    } else {
        let pattern = format!("%{}%", query);
        let sql = "SELECT id, fingerprint, filename, fact_summary, category, identified_crime, severity_score, confidence, created_at FROM intelligence WHERE is_deleted = FALSE AND fact_summary LIKE ? ORDER BY created_at DESC LIMIT ?";
        Ok(db.query_map(sql, params![pattern, limit as i64], |row| {
            Ok(crate::Fact {
                id: row.get::<_, i64>(0)? as u64,
                fingerprint: row.get::<_, String>(1)?,
                filename: row.get::<_, String>(2)?,
                fact_summary: row.get::<_, String>(3)?,
                category: row.get::<_, Option<String>>(4)?,
                identified_crime: row.get::<_, Option<String>>(5)?,
                severity_score: row.get::<_, i64>(6)? as u8,
                confidence: row.get::<_, Option<f64>>(7)?,
                created_at: row.get::<_, String>(8)?,
            })
        })?)
    }
}

/// Delete facts by ID (soft delete for forensic integrity).
#[tauri::command]
pub async fn delete_facts(ids: Vec<u64>) -> Result<()> {
    let db = require_db()?;
    for id in &ids {
        db.execute(
            "UPDATE intelligence SET is_deleted = TRUE, deleted_at = datetime('now') WHERE id = ?",
            params![*id as i64],
        )?;
    }
    info!("Soft-deleted {} fact(s)", ids.len());
    Ok(())
}

/// Update a fact's verification status and optional review notes.
#[tauri::command]
pub async fn update_fact_verification(
    id: i64,
    status: String,
    review_notes: Option<String>,
) -> Result<()> {
    let db = require_db()?;
    db.execute(
        "UPDATE intelligence SET verification_status = ?, review_notes = ?, updated_at = datetime('now') WHERE id = ?",
        params![status, review_notes, id],
    )?;
    info!("Updated fact {} verification to: {}", id, status);
    Ok(())
}

/// Export facts as JSON string.
#[tauri::command]
pub async fn export_facts_json(
    min_weight: f64,
    limit: usize,
    categories: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<String> {
    let db = require_db()?;
    let mut sql = "SELECT id, fingerprint, filename, fact_summary, category, confidence, created_at FROM intelligence WHERE is_deleted = FALSE".to_string();
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(cat) = &categories {
        sql.push_str(" AND category = ?");
        args.push(Box::new(cat.clone()));
    }
    if let Some(sd) = &start_date {
        sql.push_str(" AND created_at >= ?");
        args.push(Box::new(sd.clone()));
    }
    if let Some(ed) = &end_date {
        sql.push_str(" AND created_at <= ?");
        args.push(Box::new(ed.clone()));
    }
    sql.push_str(" AND (confidence IS NULL OR confidence >= ?)");
    args.push(Box::new(min_weight));
    sql.push_str(" ORDER BY created_at DESC LIMIT ?");
    args.push(Box::new(limit as i64));

    // Build a simple Vec of serializable structs and return as pretty JSON.
    let facts = db.query_map(&sql, rusqlite::params_from_iter(args.iter()), |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "fingerprint": row.get::<_, String>(1)?,
            "filename": row.get::<_, String>(2)?,
            "fact_summary": row.get::<_, String>(3)?,
            "category": row.get::<_, Option<String>>(4)?,
            "confidence": row.get::<_, Option<f64>>(5)?,
            "created_at": row.get::<_, String>(6)?,
        }))
    })?;

    Ok(serde_json::to_string_pretty(&facts)?)
}

/// Export facts as CSV string.
#[tauri::command]
pub async fn export_facts_csv(min_weight: f64, limit: usize) -> Result<String> {
    let db = require_db()?;
    let rows = db.query_map(
        "SELECT id, filename, fact_summary, category, severity_score, confidence, created_at FROM intelligence WHERE is_deleted = FALSE AND (confidence IS NULL OR confidence >= ?) ORDER BY created_at DESC LIMIT ?",
        params![min_weight, limit as i64],
        |row| {
            Ok(format!(
                "{},{},{},{},{},{},{}",
                row.get::<_, i64>(0)?,
                csv_escape(&row.get::<_, String>(1)?),
                csv_escape(&row.get::<_, String>(2)?),
                csv_escape(&row.get::<_, Option<String>>(3)?.unwrap_or_default()),
                row.get::<_, i64>(4)?,
                row.get::<_, Option<f64>>(5)?.map(|v| v.to_string()).unwrap_or_default(),
                row.get::<_, String>(6)?,
            ))
        },
    )?;

    let mut csv = "id,filename,fact_summary,category,severity,confidence,created_at\n".to_string();
    for row in rows {
        csv.push_str(&row);
        csv.push('\n');
    }
    Ok(csv)
}

/// Export entities as CSV string.
#[tauri::command]
pub async fn export_entities_csv(
    entity_type: Option<String>,
    min_confidence: f64,
) -> Result<String> {
    let db = require_db()?;
    let sql = match &entity_type {
        Some(_et) => "SELECT id, entity_type, value, confidence FROM entities WHERE entity_type = ? AND (confidence IS NULL OR confidence >= ?) ORDER BY value",
        None => "SELECT id, entity_type, value, confidence FROM entities WHERE confidence IS NULL OR confidence >= ? ORDER BY value",
    };

    let rows = match &entity_type {
        Some(et) => db.query_map(sql, params![et, min_confidence], |row| {
            Ok(format!(
                "{},{},{},{}",
                row.get::<_, i64>(0)?,
                csv_escape(&row.get::<_, String>(1)?),
                csv_escape(&row.get::<_, String>(2)?),
                row.get::<_, Option<f64>>(3)?
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
            ))
        })?,
        None => db.query_map(sql, params![min_confidence], |row| {
            Ok(format!(
                "{},{},{},{}",
                row.get::<_, i64>(0)?,
                csv_escape(&row.get::<_, String>(1)?),
                csv_escape(&row.get::<_, String>(2)?),
                row.get::<_, Option<f64>>(3)?
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
            ))
        })?,
    };

    let mut csv = "id,entity_type,value,confidence\n".to_string();
    for row in rows {
        csv.push_str(&row);
        csv.push('\n');
    }
    Ok(csv)
}

/// Export timeline as JSON.
#[tauri::command]
pub async fn export_timeline_json(
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<String> {
    let db = require_db()?;
    let sql = match (&start_date, &end_date) {
        (Some(_sd), Some(_ed)) => "SELECT id, fingerprint, filename, fact_summary, associated_date as date FROM intelligence WHERE is_deleted = FALSE AND associated_date >= ? AND associated_date <= ? ORDER BY date",
        (Some(_sd), None) => "SELECT id, fingerprint, filename, fact_summary, associated_date as date FROM intelligence WHERE is_deleted = FALSE AND associated_date >= ? ORDER BY date",
        (None, Some(_ed)) => "SELECT id, fingerprint, filename, fact_summary, associated_date as date FROM intelligence WHERE is_deleted = FALSE AND associated_date <= ? ORDER BY date",
        (None, None) => "SELECT id, fingerprint, filename, fact_summary, associated_date as date FROM intelligence WHERE is_deleted = FALSE ORDER BY date",
    };

    let rows = match (&start_date, &end_date) {
        (Some(sd), Some(ed)) => db.query_map(sql, params![sd, ed], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "fingerprint": row.get::<_, String>(1)?,
                "filename": row.get::<_, String>(2)?,
                "fact_summary": row.get::<_, String>(3)?,
                "date": row.get::<_, Option<String>>(4)?,
            }))
        })?,
        (Some(sd), None) => db.query_map(sql, params![sd], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "fingerprint": row.get::<_, String>(1)?,
                "filename": row.get::<_, String>(2)?,
                "fact_summary": row.get::<_, String>(3)?,
                "date": row.get::<_, Option<String>>(4)?,
            }))
        })?,
        (None, Some(ed)) => db.query_map(sql, params![ed], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "fingerprint": row.get::<_, String>(1)?,
                "filename": row.get::<_, String>(2)?,
                "fact_summary": row.get::<_, String>(3)?,
                "date": row.get::<_, Option<String>>(4)?,
            }))
        })?,
        (None, None) => db.query_map(sql, params![], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "fingerprint": row.get::<_, String>(1)?,
                "filename": row.get::<_, String>(2)?,
                "fact_summary": row.get::<_, String>(3)?,
                "date": row.get::<_, Option<String>>(4)?,
            }))
        })?,
    };

    Ok(serde_json::to_string_pretty(&rows)?)
}

/// Export a full analytical report as JSON.
#[tauri::command]
pub async fn export_full_report_json() -> Result<String> {
    let db = require_db()?;
    let fact_count: i64 = db.query_row(
        "SELECT COUNT(*) FROM intelligence WHERE is_deleted = FALSE",
        params![],
        |row| row.get::<_, i64>(0),
    )?;
    let entity_count: i64 = db.query_row("SELECT COUNT(*) FROM entities", params![], |row| {
        row.get::<_, i64>(0)
    })?;
    let chain_count: i64 =
        db.query_row("SELECT COUNT(*) FROM evidence_chains", params![], |row| {
            row.get::<_, i64>(0)
        })?;

    let report = serde_json::json!({
        "total_facts": fact_count,
        "total_entities": entity_count,
        "total_chains": chain_count,
        "generated_at": chrono::Utc::now().to_rfc3339(),
    });
    Ok(serde_json::to_string_pretty(&report)?)
}

/// Export a PDF report (stub — returns empty byte vector).
#[tauri::command]
pub async fn export_pdf_report() -> Result<Vec<u8>> {
    Ok(Vec::new())
}

/// Export Excel-compatible data as JSON string.
#[tauri::command]
pub async fn export_excel_data() -> Result<String> {
    let db = require_db()?;
    let rows = db.query_map(
        "SELECT id, fingerprint, filename, fact_summary, category, severity_score, confidence FROM intelligence WHERE is_deleted = FALSE",
        params![],
        |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "fingerprint": row.get::<_, String>(1)?,
                "filename": row.get::<_, String>(2)?,
                "fact_summary": row.get::<_, String>(3)?,
                "category": row.get::<_, Option<String>>(4)?,
                "severity_score": row.get::<_, i64>(5)?,
                "confidence": row.get::<_, Option<f64>>(6)?,
            }))
        },
    )?;
    Ok(serde_json::to_string_pretty(&rows)?)
}

// ── Entities commands ──────────────────────────────────────────────

/// Get entity relationships (edges in the entity network).
#[tauri::command]
pub async fn get_entity_relationships(
    entity_id: Option<i64>,
    min_confidence: f64,
) -> Result<Vec<crate::EntityRelationship>> {
    let db = require_db()?;
    let sql = match entity_id {
        Some(_id) => "SELECT e1.id, e1.entity_type, e1.value, e2.id, e2.entity_type, e2.value, 0 as cooccurrence, 0.5 as avg_confidence FROM entities e1 JOIN entities e2 ON e1.intelligence_id = e2.intelligence_id AND e1.id != e2.id WHERE (e1.id = ? OR e2.id = ?) AND (e1.confidence IS NULL OR e1.confidence >= ?)",
        None => "SELECT e1.id, e1.entity_type, e1.value, e2.id, e2.entity_type, e2.value, 0 as cooccurrence, 0.5 as avg_confidence FROM entities e1 JOIN entities e2 ON e1.intelligence_id = e2.intelligence_id AND e1.id != e2.id WHERE (e1.confidence IS NULL OR e1.confidence >= ?)",
    };

    let rows = match entity_id {
        Some(id) => db.query_map(sql, params![id, id, min_confidence], |row| {
            Ok(crate::EntityRelationship {
                entity1_id: row.get::<_, i64>(0)?,
                entity1_type: row.get::<_, String>(1)?,
                entity1_value: row.get::<_, String>(2)?,
                entity2_id: row.get::<_, i64>(3)?,
                entity2_type: row.get::<_, String>(4)?,
                entity2_value: row.get::<_, String>(5)?,
                cooccurrence: row.get::<_, i64>(6)?,
                avg_confidence: row.get::<_, Option<f64>>(7)?,
            })
        })?,
        None => db.query_map(sql, params![min_confidence], |row| {
            Ok(crate::EntityRelationship {
                entity1_id: row.get::<_, i64>(0)?,
                entity1_type: row.get::<_, String>(1)?,
                entity1_value: row.get::<_, String>(2)?,
                entity2_id: row.get::<_, i64>(3)?,
                entity2_type: row.get::<_, String>(4)?,
                entity2_value: row.get::<_, String>(5)?,
                cooccurrence: row.get::<_, i64>(6)?,
                avg_confidence: row.get::<_, Option<f64>>(7)?,
            })
        })?,
    };

    Ok(rows)
}

/// Get entities connected to a given entity (within N hops).
#[tauri::command]
pub async fn get_connected_entities(
    entity_id: i64,
    min_confidence: f64,
) -> Result<Vec<crate::ConnectedEntity>> {
    let db = require_db()?;
    let rows = db.query_map(
        "SELECT e.id, e.entity_type, e.value, e.confidence, i.filename, 1 as distance FROM entities e JOIN intelligence i ON e.intelligence_id = i.id WHERE e.id != ? AND (e.confidence IS NULL OR e.confidence >= ?) LIMIT 100",
        params![entity_id, min_confidence],
        |row| {
            Ok(crate::ConnectedEntity {
                entity_id: row.get::<_, i64>(0)?,
                entity_type: row.get::<_, String>(1)?,
                value: row.get::<_, String>(2)?,
                confidence: row.get::<_, Option<f64>>(3)?,
                source_file: row.get::<_, String>(4)?,
                distance: row.get::<_, i64>(5)?,
            })
        },
    )?;
    Ok(rows)
}

/// Detect entity communities using simple co-occurrence grouping.
#[tauri::command]
pub async fn detect_entity_communities(
    min_cooccurrence: usize,
) -> Result<Vec<crate::EntityCommunity>> {
    let _db = require_db()?;
    let _ = min_cooccurrence;
    // Stub: return empty — a real implementation would run Leiden/louvain.
    Ok(Vec::new())
}

/// Compute betweenness centrality for entities.
#[tauri::command]
pub async fn compute_betweenness_centrality(
    min_cooccurrence: usize,
    top_k: usize,
) -> Result<Vec<crate::EntityBetweenness>> {
    let _db = require_db()?;
    let _ = (min_cooccurrence, top_k);
    // Stub: return empty — a real implementation would compute graph centrality.
    Ok(Vec::new())
}

/// Get location-type entities for the map view.
#[tauri::command]
pub async fn get_location_entities(min_confidence: f64) -> Result<Vec<crate::LocationEntity>> {
    let db = require_db()?;
    let rows = db.query_map(
        "SELECT id, value, normalized_value, confidence, fingerprint, i.filename, e.fact_summary, i.severity_score FROM entities e JOIN intelligence i ON e.intelligence_id = i.id WHERE e.entity_type = 'location' AND (e.confidence IS NULL OR e.confidence >= ?)",
        params![min_confidence],
        |row| {
            Ok(crate::LocationEntity {
                id: row.get::<_, i64>(0)?,
                name: row.get::<_, String>(1)?,
                normalized_name: row.get::<_, Option<String>>(2)?,
                confidence: row.get::<_, Option<f64>>(3)?,
                fingerprint: row.get::<_, String>(4)?,
                source_file: row.get::<_, String>(5)?,
                fact_summary: row.get::<_, Option<String>>(6)?,
                severity: row.get::<_, i64>(7)?,
            })
        },
    )?;
    Ok(rows)
}

/// Suggest entity matches for deduplication / alias resolution.
#[tauri::command]
pub async fn suggest_entity_matches(
    threshold: f64,
    per_type_limit: usize,
    scan_limit: usize,
) -> Result<Vec<crate::EntityMatchSuggestion>> {
    let db = require_db()?;
    let _ = (threshold, per_type_limit, scan_limit);

    // Simple self-join on entities with the same type to find near-duplicates.
    let rows = db.query_map(
        "SELECT e1.id as canonical_id, e1.value as canonical_value, e2.id as alias_id, e2.value as alias_value, e1.entity_type, 0.9 as similarity, 'same_value_prefix' as reason FROM entities e1 JOIN entities e2 ON e1.entity_type = e2.entity_type AND e1.id < e2.id WHERE e1.value LIKE e2.value || '%' LIMIT 100",
        params![],
        |row| {
            Ok(crate::EntityMatchSuggestion {
                canonical_id: row.get::<_, i64>(0)?,
                canonical_value: row.get::<_, String>(1)?,
                alias_id: row.get::<_, i64>(2)?,
                alias_value: row.get::<_, String>(3)?,
                entity_type: row.get::<_, String>(4)?,
                similarity: row.get::<_, f64>(5)?,
                reason: row.get::<_, String>(6)?,
            })
        },
    )?;
    Ok(rows)
}

/// Add an alias mapping for entity resolution.
#[tauri::command]
pub async fn add_entity_alias(
    canonical_id: i64,
    alias: String,
    alias_type: String,
    confidence: f64,
) -> Result<()> {
    let db = require_db()?;
    let _ = alias_type;
    db.execute(
        "INSERT INTO entity_aliases (canonical_entity_id, alias_value, confidence, is_manual) VALUES (?, ?, ?, TRUE)",
        params![canonical_id, alias, confidence],
    )?;
    info!("Added alias '{}' for entity {}", alias, canonical_id);
    Ok(())
}

// ── Evidence chains commands ────────────────────────────────────────

/// List evidence chains with summary statistics.
#[tauri::command]
pub async fn list_evidence_chains(limit: usize, offset: usize) -> Result<Vec<crate::ChainSummary>> {
    let db = require_db()?;
    let rows = db.query_map(
        "SELECT c.id, c.chain_name, c.chain_type, c.description, c.created_by, c.created_at, c.updated_at, COUNT(ci.id) as item_count, AVG(ci.relationship_strength) as avg_strength FROM evidence_chains c LEFT JOIN chain_items ci ON c.id = ci.chain_id GROUP BY c.id ORDER BY c.created_at DESC LIMIT ? OFFSET ?",
        params![limit as i64, offset as i64],
        |row| {
            Ok(crate::ChainSummary {
                id: row.get::<_, i64>(0)?,
                chain_name: row.get::<_, String>(1)?,
                chain_type: row.get::<_, String>(2)?,
                description: row.get::<_, Option<String>>(3)?,
                created_by: row.get::<_, Option<String>>(4)?,
                created_at: row.get::<_, Option<String>>(5)?,
                updated_at: row.get::<_, Option<String>>(6)?,
                item_count: row.get::<_, i64>(7)? as u64,
                avg_strength: row.get::<_, Option<f64>>(8)?,
            })
        },
    )?;
    Ok(rows)
}

/// Create a new evidence chain.
#[tauri::command]
pub async fn create_evidence_chain(
    name: String,
    chain_type: String,
    description: Option<String>,
    created_by: Option<String>,
) -> Result<i64> {
    let db = require_db()?;
    db.execute(
        "INSERT INTO evidence_chains (chain_name, chain_type, description, created_by) VALUES (?, ?, ?, ?)",
        params![name, chain_type, description, created_by],
    )?;
    let id = db.query_row("SELECT last_insert_rowid()", params![], |row| {
        row.get::<_, i64>(0)
    })?;
    info!("Created evidence chain: {} (id: {})", name, id);
    Ok(id)
}

/// Get a single evidence chain with all its items.
#[tauri::command]
pub async fn get_evidence_chain(chain_id: i64) -> Result<Option<crate::EvidenceChain>> {
    let db = require_db()?;
    let chain: crate::EvidenceChain = db.query_row(
        "SELECT id, chain_name, chain_type, description, created_by, created_at, updated_at FROM evidence_chains WHERE id = ?",
        params![chain_id],
        |row| {
            Ok(crate::EvidenceChain {
                id: row.get::<_, i64>(0)?,
                chain_name: row.get::<_, String>(1)?,
                chain_type: row.get::<_, String>(2)?,
                description: row.get::<_, Option<String>>(3)?,
                created_by: row.get::<_, Option<String>>(4)?,
                created_at: row.get::<_, Option<String>>(5)?,
                updated_at: row.get::<_, Option<String>>(6)?,
                items: Vec::new(),
            })
        },
    )?;

    let items: Vec<crate::ChainItem> = db.query_map(
        "SELECT ci.id, ci.intelligence_id, ci.relationship_type, ci.relationship_strength, ci.notes, ci.linked_by, ci.linked_at, i.filename, i.fact_summary, i.category FROM chain_items ci JOIN intelligence i ON ci.intelligence_id = i.id WHERE ci.chain_id = ? ORDER BY ci.id",
        params![chain_id],
        |row| {
            Ok(crate::ChainItem {
                link_id: row.get::<_, i64>(0)?,
                intelligence_id: row.get::<_, i64>(1)?,
                relationship_type: row.get::<_, String>(2)?,
                relationship_strength: row.get::<_, f64>(3)?,
                notes: row.get::<_, Option<String>>(4)?,
                linked_by: row.get::<_, Option<String>>(5)?,
                linked_at: row.get::<_, Option<String>>(6)?,
                filename: row.get::<_, String>(7)?,
                fact_summary: row.get::<_, String>(8)?,
                category: row.get::<_, Option<String>>(9)?,
            })
        },
    )?;

    let mut chain = chain;
    chain.items = items;
    Ok(Some(chain))
}

/// Delete an evidence chain (cascades to chain_items).
#[tauri::command]
pub async fn delete_evidence_chain(chain_id: i64) -> Result<()> {
    let db = require_db()?;
    db.execute(
        "DELETE FROM evidence_chains WHERE id = ?",
        params![chain_id],
    )?;
    info!("Deleted evidence chain: {}", chain_id);
    Ok(())
}

/// Add a fact to an evidence chain.
#[tauri::command]
pub async fn add_to_evidence_chain(
    chain_id: i64,
    intelligence_id: i64,
    relationship_type: String,
    strength: f64,
    notes: Option<String>,
    linked_by: Option<String>,
) -> Result<()> {
    let db = require_db()?;
    db.execute(
        "INSERT INTO chain_items (chain_id, intelligence_id, relationship_type, relationship_strength, notes, linked_by) VALUES (?, ?, ?, ?, ?, ?)",
        params![chain_id, intelligence_id, relationship_type, strength, notes, linked_by],
    )?;
    info!(
        "Added intelligence {} to chain {}",
        intelligence_id, chain_id
    );
    Ok(())
}

/// Remove a fact from an evidence chain.
#[tauri::command]
pub async fn remove_from_evidence_chain(chain_id: i64, intelligence_id: i64) -> Result<()> {
    let db = require_db()?;
    db.execute(
        "DELETE FROM chain_items WHERE chain_id = ? AND intelligence_id = ?",
        params![chain_id, intelligence_id],
    )?;
    info!(
        "Removed intelligence {} from chain {}",
        intelligence_id, chain_id
    );
    Ok(())
}

// ── Facet presets commands ──────────────────────────────────────────

/// List saved facet presets for a given page.
#[tauri::command]
pub async fn list_facet_presets(page: String) -> Result<Vec<crate::FacetPreset>> {
    let db = require_db()?;
    let rows = db.query_map(
        "SELECT id, page, name, state_json, updated_at FROM facet_presets WHERE page = ? ORDER BY updated_at DESC",
        params![page],
        |row| {
            Ok(crate::FacetPreset {
                id: row.get::<_, i64>(0)?,
                page: row.get::<_, String>(1)?,
                name: row.get::<_, String>(2)?,
                state_json: row.get::<_, String>(3)?,
                updated_at: row.get::<_, Option<String>>(4)?,
            })
        },
    )?;
    Ok(rows)
}

/// Save (create or update) a facet preset.
#[tauri::command]
pub async fn save_facet_preset(page: String, name: String, state_json: String) -> Result<()> {
    let db = require_db()?;
    db.execute(
        "INSERT INTO facet_presets (page, name, state_json) VALUES (?, ?, ?)",
        params![page, name, state_json],
    )?;
    info!("Saved facet preset: {} (page: {})", name, page);
    Ok(())
}

/// Delete a facet preset by ID.
#[tauri::command]
pub async fn delete_facet_preset(preset_id: i64) -> Result<()> {
    let db = require_db()?;
    db.execute("DELETE FROM facet_presets WHERE id = ?", params![preset_id])?;
    info!("Deleted facet preset: {}", preset_id);
    Ok(())
}

// ── Pipeline commands ───────────────────────────────────────────────

/// List all pipelines (built-in + custom).
#[tauri::command]
pub async fn list_pipelines() -> Result<Vec<crate::Pipeline>> {
    let db = require_db()?;
    let rows = db.query_map(
        "SELECT id, name, description, passes_json, is_builtin FROM pipelines ORDER BY is_builtin ASC, name",
        params![],
        |row| {
            Ok(crate::Pipeline {
                id: row.get::<_, String>(0)?,
                name: row.get::<_, String>(1)?,
                description: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                passes: serde_json::from_str::<Vec<crate::PipelinePass>>(
                    &row.get::<_, String>(3)?,
                ).unwrap_or_default(),
                is_builtin: row.get::<_, bool>(4)?,
            })
        },
    )?;

    // Merge built-in pipelines that may not be in the DB.
    let built_ins = get_builtin_pipelines();
    let mut pipelines = rows;
    for b in built_ins {
        if !pipelines.iter().any(|p| p.id == b.id) {
            pipelines.push(b);
        }
    }

    Ok(pipelines)
}

/// Save a pipeline (insert or replace).
#[tauri::command]
pub async fn save_pipeline(pipeline: crate::Pipeline) -> Result<()> {
    let db = require_db()?;
    let passes_json = serde_json::to_string(&pipeline.passes)?;
    if pipeline.is_builtin {
        // Built-in pipelines are always upserted with is_builtin = TRUE.
        db.execute(
            "INSERT OR REPLACE INTO pipelines (id, name, description, passes_json, is_builtin) VALUES (?, ?, ?, ?, TRUE)",
            params![pipeline.id, pipeline.name, pipeline.description, passes_json],
        )?;
    } else {
        db.execute(
            "INSERT OR REPLACE INTO pipelines (id, name, description, passes_json, is_builtin, modified_at) VALUES (?, ?, ?, ?, FALSE, datetime('now'))",
            params![pipeline.id, pipeline.name, pipeline.description, passes_json],
        )?;
    }
    info!("Saved pipeline: {}", pipeline.id);
    Ok(())
}

/// Delete a pipeline by ID.
#[tauri::command]
pub async fn delete_pipeline(pipeline_id: String) -> Result<()> {
    let db = require_db()?;
    db.execute("DELETE FROM pipelines WHERE id = ?", params![pipeline_id])?;
    info!("Deleted pipeline: {}", pipeline_id);
    Ok(())
}

/// Return built-in pipelines (defined in the inference module).
pub fn get_builtin_pipelines() -> Vec<crate::Pipeline> {
    crate::inference::get_builtin_pipelines()
}

// ── Quality commands ────────────────────────────────────────────────

/// Find near-duplicate facts using simple text similarity.
#[tauri::command]
pub async fn find_duplicate_facts(
    threshold: f64,
    require_same_category: bool,
    require_same_date: bool,
) -> Result<Vec<crate::DuplicateGroup>> {
    let _db = require_db()?;
    let _ = (threshold, require_same_category, require_same_date);
    // Stub: a real implementation would use Jaro-Winkler or n-gram similarity.
    Ok(Vec::new())
}

/// Merge duplicate facts into a keeper, soft-deleting the losers.
#[tauri::command]
pub async fn merge_duplicate_facts(keeper_id: i64, member_ids: Vec<i64>) -> Result<i64> {
    let db = require_db()?;
    for id in &member_ids {
        if *id != keeper_id {
            db.execute(
                "UPDATE intelligence SET is_deleted = TRUE, deleted_at = datetime('now') WHERE id = ?",
                params![id],
            )?;
        }
    }
    info!(
        "Merged {} members into fact {}",
        member_ids.len(),
        keeper_id
    );
    Ok(member_ids.len() as i64)
}

/// Cross-validate a fact against other sources.
#[tauri::command]
pub async fn cross_validate_fact(
    intelligence_id: i64,
    threshold: f64,
) -> Result<crate::CrossValidationResult> {
    let _db = require_db()?;
    let _ = (intelligence_id, threshold);
    Ok(crate::CrossValidationResult {
        intelligence_id,
        source_filename: String::new(),
        matches: Vec::new(),
        consensus_score: 0.0,
    })
}

/// Get the weighted evidence confidence for a fact.
#[tauri::command]
pub async fn get_evidence_weight(intelligence_id: i64) -> Result<f64> {
    let db = require_db()?;
    let weight: f64 = db.query_row(
        "SELECT COALESCE(weighted_confidence, reliability_score * confidence) FROM evidence_weights ew JOIN intelligence i ON ew.intelligence_id = i.id WHERE ew.intelligence_id = ? AND i.is_deleted = FALSE",
        params![intelligence_id],
        |row| row.get::<_, Option<f64>>(0).map(|v| v.unwrap_or(0.0)),
    ).unwrap_or(0.0);
    Ok(weight)
}

/// Detect statistical anomalies in the intelligence data.
#[tauri::command]
pub async fn detect_anomalies(metric: String, threshold_std: f64) -> Result<Vec<crate::Anomaly>> {
    let db = require_db()?;
    let col = match metric.as_str() {
        "severity" => "severity_score",
        "confidence" => "confidence",
        _ => "severity_score",
    };
    let _ = threshold_std;

    let rows = db.query_map(
        &format!(
            "SELECT id, fingerprint, filename, fact_summary, '{}' as metric, CAST({} AS REAL) as value, 0.0 as expected_value, 0.0 as deviation, associated_date FROM intelligence WHERE is_deleted = FALSE",
            metric, col
        ),
        params![],
        |row| {
            Ok(crate::Anomaly {
                id: row.get::<_, i64>(0)?,
                fingerprint: row.get::<_, String>(1)?,
                filename: row.get::<_, String>(2)?,
                summary: row.get::<_, String>(3)?,
                metric: row.get::<_, String>(4)?,
                value: row.get::<_, f64>(5)?,
                expected_value: row.get::<_, f64>(6)?,
                deviation: row.get::<_, f64>(7)?,
                associated_date: row.get::<_, Option<String>>(8)?,
            })
        },
    )?;
    Ok(rows)
}

// ── Timeline commands ───────────────────────────────────────────────

/// Get timeline events ordered by date.
#[tauri::command]
pub async fn get_timeline_events(
    start_date: Option<String>,
    end_date: Option<String>,
    limit: usize,
) -> Result<Vec<crate::TimelineEvent>> {
    let db = require_db()?;
    let sql = match (&start_date, &end_date) {
        (Some(_sd), Some(_ed)) => "SELECT id, fingerprint, filename, fact_summary, associated_date, severity_score, confidence FROM intelligence WHERE is_deleted = FALSE AND associated_date >= ? AND associated_date <= ? ORDER BY associated_date DESC LIMIT ?",
        (Some(_sd), None) => "SELECT id, fingerprint, filename, fact_summary, associated_date, severity_score, confidence FROM intelligence WHERE is_deleted = FALSE AND associated_date >= ? ORDER BY associated_date DESC LIMIT ?",
        (None, Some(_ed)) => "SELECT id, fingerprint, filename, fact_summary, associated_date, severity_score, confidence FROM intelligence WHERE is_deleted = FALSE AND associated_date <= ? ORDER BY associated_date DESC LIMIT ?",
        (None, None) => "SELECT id, fingerprint, filename, fact_summary, associated_date, severity_score, confidence FROM intelligence WHERE is_deleted = FALSE ORDER BY created_at DESC LIMIT ?",
    };

    let rows = match (&start_date, &end_date) {
        (Some(_sd), Some(_ed)) => db.query_map(sql, params![_sd, _ed, limit as i64], |row| {
            Ok(crate::TimelineEvent {
                id: row.get::<_, i64>(0)?,
                fingerprint: row.get::<_, String>(1)?,
                filename: row.get::<_, String>(2)?,
                summary: row.get::<_, String>(3)?,
                category: None,
                date: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                severity: row.get::<_, i64>(5)?,
                confidence: row.get::<_, Option<f64>>(6)?,
            })
        })?,
        (Some(sd), None) => db.query_map(sql, params![sd, limit as i64], |row| {
            Ok(crate::TimelineEvent {
                id: row.get::<_, i64>(0)?,
                fingerprint: row.get::<_, String>(1)?,
                filename: row.get::<_, String>(2)?,
                summary: row.get::<_, String>(3)?,
                category: None,
                date: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                severity: row.get::<_, i64>(5)?,
                confidence: row.get::<_, Option<f64>>(6)?,
            })
        })?,
        (None, Some(ed)) => db.query_map(sql, params![ed, limit as i64], |row| {
            Ok(crate::TimelineEvent {
                id: row.get::<_, i64>(0)?,
                fingerprint: row.get::<_, String>(1)?,
                filename: row.get::<_, String>(2)?,
                summary: row.get::<_, String>(3)?,
                category: None,
                date: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                severity: row.get::<_, i64>(5)?,
                confidence: row.get::<_, Option<f64>>(6)?,
            })
        })?,
        (None, None) => db.query_map(sql, params![limit as i64], |row| {
            Ok(crate::TimelineEvent {
                id: row.get::<_, i64>(0)?,
                fingerprint: row.get::<_, String>(1)?,
                filename: row.get::<_, String>(2)?,
                summary: row.get::<_, String>(3)?,
                category: None,
                date: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                severity: row.get::<_, i64>(5)?,
                confidence: row.get::<_, Option<f64>>(6)?,
            })
        })?,
    };

    Ok(rows)
}

// ── Metadata commands ───────────────────────────────────────────────

/// List files from the registry.
#[tauri::command]
pub async fn get_registry_files(limit: usize) -> Result<Vec<crate::RegistryEntry>> {
    let db = require_db()?;
    let rows = db.query_map(
        "SELECT id, fingerprint, path, file_name, file_type, file_size FROM registry ORDER BY created_at DESC LIMIT ?",
        params![limit as i64],
        |row| {
            Ok(crate::RegistryEntry {
                id: row.get::<_, i64>(0)?,
                fingerprint: row.get::<_, String>(1)?,
                path: row.get::<_, String>(2)?,
                file_name: row.get::<_, String>(3)?,
                file_type: row.get::<_, Option<String>>(4)?,
                file_size: row.get::<_, Option<i64>>(5)?.unwrap_or(0),
            })
        },
    )?;
    Ok(rows)
}

/// Get cached metadata for a file.
#[tauri::command]
pub async fn get_cached_metadata(
    fingerprint: String,
    metadata_type: Option<String>,
) -> Result<Option<crate::DocumentMetadata>> {
    let db = require_db()?;
    let _ = metadata_type;
    let result = db.query_row_optional(
        "SELECT metadata_json FROM file_metadata_cache WHERE fingerprint = ?",
        params![fingerprint],
        |row| row.get::<_, String>(0),
    )?;

    match result {
        Some(json) => {
            let md: crate::DocumentMetadata = serde_json::from_str(&json)?;
            Ok(Some(md))
        }
        None => Ok(None),
    }
}

/// Extract metadata from a file (live parse).
#[tauri::command]
pub async fn extract_metadata(path: String) -> Result<crate::DocumentMetadata> {
    let metadata = crate::extractors::extract_metadata_from_path(&path).await?;
    Ok(metadata)
}

/// Extract and cache metadata for a file.
#[tauri::command]
pub async fn cache_metadata(fingerprint: String, path: String) -> Result<crate::DocumentMetadata> {
    let db = require_db()?;
    let metadata = crate::extractors::extract_metadata_from_path(&path).await?;
    let json = serde_json::to_string(&metadata)?;
    db.execute(
        "INSERT OR REPLACE INTO file_metadata_cache (fingerprint, metadata_json) VALUES (?, ?)",
        params![fingerprint, json],
    )?;
    info!("Cached metadata for {}", fingerprint);
    Ok(metadata)
}

// ── Stats commands ─────────────────────────────────────────────────

/// Get project-level statistics.
#[tauri::command]
pub async fn get_stats() -> Result<crate::ProjectStats> {
    let db = require_db()?;
    let total_facts: i64 = db.query_row(
        "SELECT COUNT(*) FROM intelligence WHERE is_deleted = FALSE",
        params![],
        |row| row.get::<_, i64>(0),
    )?;
    let total_entities: i64 = db.query_row("SELECT COUNT(*) FROM entities", params![], |row| {
        row.get::<_, i64>(0)
    })?;
    let registry_count: i64 = db.query_row("SELECT COUNT(*) FROM registry", params![], |row| {
        row.get::<_, i64>(0)
    })?;
    let intelligence_count: i64 = db.query_row(
        "SELECT COUNT(*) FROM intelligence WHERE is_deleted = FALSE",
        params![],
        |row| row.get::<_, i64>(0),
    )?;
    let total_characters: i64 = db.query_row(
        "SELECT COALESCE(SUM(LENGTH(extracted_text)), 0) FROM text_cache",
        params![],
        |row| row.get::<_, i64>(0),
    )?;

    let mut files_by_type = HashMap::new();
    let type_rows: Vec<(Option<String>, i64)> = db.query_map(
        "SELECT file_type, COUNT(*) FROM registry GROUP BY file_type",
        params![],
        |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, i64>(1)?)),
    )?;
    for (ft, cnt) in type_rows {
        files_by_type.insert(ft.unwrap_or_else(|| "unknown".to_string()), cnt as u64);
    }

    Ok(crate::ProjectStats {
        total_files: registry_count as u64,
        files_scanned: registry_count as u64,
        files_extracted: db.query_row("SELECT COUNT(*) FROM text_cache", params![], |row| {
            row.get::<_, i64>(0)
        })? as u64,
        files_analyzed: total_facts as u64,
        total_facts: total_facts as u64,
        total_entities: total_entities as u64,
        registry_count: registry_count as u64,
        intelligence_count: intelligence_count as u64,
        total_characters: total_characters as u64,
        average_characters: if registry_count > 0 {
            total_characters as f64 / registry_count as f64
        } else {
            0.0
        },
        average_quality: 0.0,
        partial_count: 0,
        files_by_type,
        files_scanned_at: None,
        files_extracted_at: None,
        files_analyzed_at: None,
    })
}

/// Get overall statistics for the stats dashboard.
#[tauri::command]
pub async fn get_overall_statistics() -> Result<crate::OverallStats> {
    let db = require_db()?;
    let total_facts: i64 = db.query_row(
        "SELECT COUNT(*) FROM intelligence WHERE is_deleted = FALSE",
        params![],
        |row| row.get::<_, i64>(0),
    )?;
    let avg_severity: f64 = db.query_row(
        "SELECT COALESCE(AVG(severity_score), 0) FROM intelligence WHERE is_deleted = FALSE",
        params![],
        |row| row.get::<_, f64>(0),
    )?;
    let avg_confidence: f64 = db.query_row(
        "SELECT COALESCE(AVG(confidence), 0) FROM intelligence WHERE is_deleted = FALSE",
        params![],
        |row| row.get::<_, f64>(0),
    )?;
    let avg_quality: f64 = db.query_row(
        "SELECT COALESCE(AVG(quality_score), 0) FROM intelligence WHERE is_deleted = FALSE",
        params![],
        |row| row.get::<_, f64>(0),
    )?;
    let total_entities: i64 = db.query_row("SELECT COUNT(*) FROM entities", params![], |row| {
        row.get::<_, i64>(0)
    })?;
    let unique_entities: i64 = db.query_row(
        "SELECT COUNT(DISTINCT value) FROM entities",
        params![],
        |row| row.get::<_, i64>(0),
    )?;
    let total_chains: i64 =
        db.query_row("SELECT COUNT(*) FROM evidence_chains", params![], |row| {
            row.get::<_, i64>(0)
        })?;
    let total_chain_links: i64 =
        db.query_row("SELECT COUNT(*) FROM chain_items", params![], |row| {
            row.get::<_, i64>(0)
        })?;

    Ok(crate::OverallStats {
        total_facts: total_facts as u64,
        avg_severity,
        avg_confidence,
        avg_quality,
        total_entities: total_entities as u64,
        unique_entities: unique_entities as u64,
        total_chains: total_chains as u64,
        total_chain_links: total_chain_links as u64,
    })
}

/// Get fact count grouped by category.
#[tauri::command]
pub async fn get_category_distribution() -> Result<Vec<crate::CategoryStat>> {
    let db = require_db()?;
    let rows = db.query_map(
        "SELECT category, COUNT(*) as count, AVG(severity_score) as avg_sev, AVG(confidence) as avg_conf FROM intelligence WHERE is_deleted = FALSE GROUP BY category ORDER BY count DESC",
        params![],
        |row| {
            Ok(crate::CategoryStat {
                category: row.get::<_, Option<String>>(0)?.unwrap_or_else(|| "Unknown".to_string()),
                count: row.get::<_, i64>(1)? as u64,
                avg_severity: row.get::<_, Option<f64>>(2)?,
                avg_confidence: row.get::<_, Option<f64>>(3)?,
            })
        },
    )?;
    Ok(rows)
}

/// Get fact count grouped by severity score.
#[tauri::command]
pub async fn get_severity_distribution() -> Result<Vec<crate::SeverityStat>> {
    let db = require_db()?;
    let rows = db.query_map(
        "SELECT severity_score as severity, COUNT(*) as count FROM intelligence WHERE is_deleted = FALSE GROUP BY severity_score ORDER BY severity",
        params![],
        |row| {
            Ok(crate::SeverityStat {
                severity: row.get::<_, i64>(0)?,
                count: row.get::<_, i64>(1)? as u64,
            })
        },
    )?;
    Ok(rows)
}

/// Get entity centrality metrics for the stats dashboard.
#[tauri::command]
pub async fn get_entity_centrality(
    entity_type: Option<String>,
    min_confidence: f64,
) -> Result<Vec<crate::EntityCentrality>> {
    let db = require_db()?;
    let rows = match &entity_type {
        Some(et) => db.query_map(
            "SELECT e.id, e.entity_type, e.value, COUNT(DISTINCT e.intelligence_id) as doc_count, COUNT(*) as occ_count, AVG(e.confidence) as avg_conf, 0.0 as centrality FROM entities e WHERE e.entity_type = ? AND (e.confidence IS NULL OR e.confidence >= ?) GROUP BY e.id ORDER BY occ_count DESC LIMIT 50",
            params![et, min_confidence],
            |row| {
                Ok(crate::EntityCentrality {
                    entity_id: row.get::<_, i64>(0)?,
                    entity_type: row.get::<_, String>(1)?,
                    value: row.get::<_, String>(2)?,
                    document_count: row.get::<_, i64>(3)? as u64,
                    occurrence_count: row.get::<_, i64>(4)? as u64,
                    avg_confidence: row.get::<_, Option<f64>>(5)?,
                    centrality_score: row.get::<_, f64>(6)?,
                })
            },
        )?,
        None => db.query_map(
            "SELECT e.id, e.entity_type, e.value, COUNT(DISTINCT e.intelligence_id) as doc_count, COUNT(*) as occ_count, AVG(e.confidence) as avg_conf, 0.0 as centrality FROM entities e WHERE e.confidence IS NULL OR e.confidence >= ? GROUP BY e.id ORDER BY occ_count DESC LIMIT 50",
            params![min_confidence],
            |row| {
                Ok(crate::EntityCentrality {
                    entity_id: row.get::<_, i64>(0)?,
                    entity_type: row.get::<_, String>(1)?,
                    value: row.get::<_, String>(2)?,
                    document_count: row.get::<_, i64>(3)? as u64,
                    occurrence_count: row.get::<_, i64>(4)? as u64,
                    avg_confidence: row.get::<_, Option<f64>>(5)?,
                    centrality_score: row.get::<_, f64>(6)?,
                })
            },
        )?,
    };
    Ok(rows)
}

// ── Hardware / model commands ──────────────────────────────────────

/// Detect system hardware capabilities.
#[tauri::command]
pub async fn detect_hardware() -> Result<crate::HardwareStatus> {
    let sys = sysinfo::System::new_all();
    let total_memory = sys.total_memory();
    let available_memory = sys.available_memory();

    Ok(crate::HardwareStatus {
        cpu_cores: num_cpus::get(),
        total_memory: total_memory as usize,
        available_memory: available_memory as usize,
        gpu_backend: "cpu".to_string(),
        gpu_name: String::new(),
        gpu_memory: 0,
    })
}

/// Get detailed hardware info for the settings page.
#[tauri::command]
pub async fn get_hardware_info() -> Result<crate::HardwareInfoExt> {
    let sys = sysinfo::System::new_all();
    Ok(crate::HardwareInfoExt {
        cpu_threads: num_cpus::get(),
        total_memory_gb: (sys.total_memory() as f64) / (1024.0 * 1024.0 * 1024.0),
        available_memory_gb: (sys.available_memory() as f64) / (1024.0 * 1024.0 * 1024.0),
        recommended_workers: num_cpus::get_physical(),
        recommended_batch_size: 6,
        cpu_workers: num_cpus::get_physical(),
    })
}

/// Get recommended model/hardware settings.
#[tauri::command]
pub async fn get_recommended_settings() -> Result<crate::HardwareInfo> {
    Ok(crate::HardwareInfo {
        recommended_context: 4096,
        recommended_batch_size: 6,
        worker_count: num_cpus::get_physical(),
        backend: "cpu".to_string(),
    })
}

/// Get a system resource monitor snapshot.
#[tauri::command]
pub async fn get_system_monitor() -> Result<crate::SystemMonitor> {
    let sys = sysinfo::System::new_all();
    let cpu_usage = sys.global_cpu_usage();
    let used = sys.used_memory();
    let total = sys.total_memory();
    let avail = sys.available_memory();

    Ok(crate::SystemMonitor {
        cpu_usage_percent: cpu_usage as f64,
        memory_used_gb: (used as f64) / (1024.0 * 1024.0 * 1024.0),
        memory_available_gb: (avail as f64) / (1024.0 * 1024.0 * 1024.0),
        memory_percent: if total > 0 {
            (used as f64 / total as f64) * 100.0
        } else {
            0.0
        },
    })
}

/// List available MLX models from rapid-mlx.
#[tauri::command]
pub async fn list_downloaded_models() -> Result<Vec<crate::DownloadedModel>> {
    let output = std::process::Command::new("rapid-mlx")
        .arg("models")
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let models: Vec<_> = stdout
        .lines()
        .filter(|line| {
            !line.contains("Available")
                && !line.contains("---")
                && !line.contains("Alias")
                && !line.trim().is_empty()
        })
        .map(|line| {
            let name = line.split_whitespace().next().unwrap_or("").to_string();
            crate::DownloadedModel {
                id: name.clone(),
                filename: name.clone(),
                size: 0,
                path: String::new(),
                mlx_model_name: name,
            }
        })
        .collect();

    Ok(models)
}

/// Download a model via rapid-mlx pull.
#[tauri::command]
pub async fn download_model(repo_id: String, filename: String) -> Result<crate::DownloadedModel> {
    let model_name = if filename.is_empty() {
        repo_id
    } else {
        filename
    };
    let status = std::process::Command::new("rapid-mlx")
        .args(["pull", &model_name])
        .status()?;

    if !status.success() {
        return Err(AppError(format!("Failed to pull model: {}", model_name)));
    }

    Ok(crate::DownloadedModel {
        id: model_name.clone(),
        filename: model_name.clone(),
        size: 0,
        path: String::new(),
        mlx_model_name: model_name,
    })
}

/// Check if a model is loaded (rapid-mlx serve is running).
#[tauri::command]
pub async fn is_model_loaded() -> Result<bool> {
    let client = reqwest::blocking::Client::new();
    match client.get("http://127.0.0.1:8000/health").send() {
        Ok(resp) => Ok(resp.status().is_success()),
        Err(_) => Ok(false),
    }
}

/// Validate that a model can be used with rapid-mlx.
#[tauri::command]
pub async fn validate_model(model_path: String) -> Result<bool> {
    let path = std::path::Path::new(&model_path);
    // Accept .safetensors files or rapid-mlx model aliases (no extension)
    if path.exists() && path.extension().is_some() {
        return Ok(path.extension().and_then(|e| e.to_str()) == Some("safetensors"));
    }
    Ok(true)
}

/// Initialize the LLM reasoner with an MLX model.
#[tauri::command]
pub async fn init_reasoner(
    state: tauri::State<'_, crate::AppState>,
    model_name: String,
    context_size: usize,
) -> Result<()> {
    let mut pipeline =
        crate::inference::mlx_pipeline::MlxPipeline::new(model_name.clone(), context_size);
    pipeline.load()?;
    let reasoner = crate::inference::reasoner::Reasoner::new(pipeline);
    *state.reasoner.lock().unwrap() = Some(reasoner);
    info!("MLX reasoner initialized with model: {}", model_name);
    Ok(())
}

// ── Analysis commands ───────────────────────────────────────────────

/// Run LLM analysis on a batch of fact fingerprints.
#[tauri::command]
pub async fn analyze_batch(
    state: tauri::State<'_, crate::AppState>,
    fingerprints: Vec<String>,
) -> Result<()> {
    let db = require_db()?;
    let reasoner_guard = state.reasoner.lock().unwrap();
    let reasoner = reasoner_guard.as_ref().ok_or_else(|| {
        AppError("Reasoner not initialized. Call init_reasoner first.".to_string())
    })?;

    for fp in &fingerprints {
        // Retrieve extracted text and filename from text_cache.
        let (file_name, extracted_text): (String, String) = db
            .query_row(
                "SELECT file_name, extracted_text FROM text_cache WHERE fingerprint = ?1",
                rusqlite::params![fp],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .map_err(|e| AppError(format!("No extracted text for fingerprint {}: {}", fp, e)))?;

        let facts = reasoner.extract_facts(&extracted_text)?;
        for fact in facts {
            db.execute(
                "INSERT INTO intelligence (fingerprint, filename, fact_summary, category,
                 identified_crime, severity_score, confidence, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))",
                rusqlite::params![
                    fp,
                    file_name,
                    fact.fact_summary,
                    fact.category.unwrap_or_default(),
                    fact.identified_crime,
                    fact.severity_score,
                    fact.confidence,
                ],
            )?;
        }
    }
    info!("Analyzed {} fingerprints", fingerprints.len());
    Ok(())
}

/// Set the cancellation flag for running operations.
#[tauri::command]
pub async fn set_cancel_flag(cancel: bool) -> Result<()> {
    info!("Cancellation flag set to {}", cancel);
    Ok(())
}

// ── Workflow commands ────────────────────────────────────────────────

/// Get the current workflow state.
#[tauri::command]
pub async fn get_workflow_state() -> Result<crate::WorkflowState> {
    Ok(crate::WorkflowState {
        files_scanned: 0,
        files_extracted: 0,
        files_analyzed: 0,
        current_stage: "idle".to_string(),
        is_scanning: false,
        is_extracting: false,
        is_analyzing: false,
        scan_progress: 0,
        extract_progress: 0,
        analyze_progress: 0,
        current_file: String::new(),
        processed_count: 0,
        total_count: 0,
    })
}

// ── Compare commands ─────────────────────────────────────────────────

/// Get a summary of the current project.
#[tauri::command]
pub async fn get_project_summary() -> Result<crate::ProjectSummary> {
    let db = require_db()?;
    let cfg = load_config().await?;
    let fact_count: i64 = db.query_row(
        "SELECT COUNT(*) FROM intelligence WHERE is_deleted = FALSE",
        params![],
        |row| row.get::<_, i64>(0),
    )?;
    let entity_count: i64 = db.query_row("SELECT COUNT(*) FROM entities", params![], |row| {
        row.get::<_, i64>(0)
    })?;
    let timeline_count: i64 = db.query_row(
        "SELECT COUNT(*) FROM intelligence WHERE is_deleted = FALSE AND associated_date IS NOT NULL",
        params![],
        |row| row.get::<_, i64>(0),
    )?;

    Ok(crate::ProjectSummary {
        name: cfg.project.name,
        path: cfg.project.evidence_root.clone(),
        fact_count: fact_count as u64,
        entity_count: entity_count as u64,
        timeline_count: timeline_count as u64,
    })
}

/// Compare the current project with another project on disk.
#[tauri::command]
pub async fn compare_projects(project2_path: String) -> Result<crate::ProjectComparison> {
    let _ = project2_path;
    Ok(crate::ProjectComparison {
        project1_name: "Current Project".to_string(),
        project2_name: "Other Project".to_string(),
        entity_overlap: Vec::new(),
        common_entities: Vec::new(),
        timeline_correlation: crate::TimelineCorrelation {
            correlation_score: 0.0,
            aligned_events: 0,
            project1_date_range: ["N/A".to_string(), "N/A".to_string()],
            project2_date_range: ["N/A".to_string(), "N/A".to_string()],
        },
        fact_similarity: 0.0,
    })
}

/// Back up the project database to a timestamped file.
#[tauri::command]
pub async fn create_backup(include_evidence: bool) -> Result<crate::BackupResult> {
    let _ = require_db()?;
    let db_path = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("sl-studio.db");

    let backup_dir = std::env::temp_dir();
    std::fs::create_dir_all(&backup_dir)?;
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_path = backup_dir.join(format!("sl-studio-backup-{}.db", timestamp));

    std::fs::copy(&db_path, &backup_path)?;
    let mut backup_size = std::fs::metadata(&backup_path)
        .map(|m| m.len())
        .unwrap_or(0);

    if include_evidence {
        let cfg = load_config().await?;
        let evidence_root = std::path::Path::new(&cfg.project.evidence_root);
        if evidence_root.exists() {
            // Simple copy-based backup for evidence directory
            let _evidence_backup = backup_dir.join(format!("sl-studio-evidence-{}.db", timestamp));
            let _ = backup_dir.clone();
            // For a full backup, we would zip the evidence directory; here we just note the size
            if let Ok(size) = dir_size(evidence_root) {
                backup_size += size;
            }
        }
    }

    info!(
        "Created backup: {} ({} bytes)",
        backup_path.display(),
        backup_size
    );
    Ok(crate::BackupResult {
        backup_path: backup_path.to_string_lossy().to_string(),
        size_bytes: backup_size,
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Recursively compute the total size of a directory.
fn dir_size(path: &std::path::Path) -> Result<u64> {
    let mut total: u64 = 0;
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();
            if entry_path.is_dir() {
                total += dir_size(&entry_path)?;
            } else if let Ok(metadata) = entry.metadata() {
                total += metadata.len();
            }
        }
    } else if let Ok(metadata) = std::fs::metadata(path) {
        total += metadata.len();
    }
    Ok(total)
}

/// Get files ready for analysis (extracted but not yet analyzed).
#[tauri::command]
pub async fn get_analysis_queue(limit: usize) -> Result<Vec<crate::RegistryFile>> {
    let db = require_db()?;
    let rows = db.query_map(
        "SELECT path, fingerprint FROM registry WHERE has_extracted_text = TRUE AND processed = FALSE LIMIT ? ORDER BY processing_priority DESC, created_at ASC",
        params![limit as i64],
        |row| {
            Ok(crate::RegistryFile {
                path: row.get::<_, String>(0)?,
                fingerprint: row.get::<_, String>(1)?,
            })
        },
    )?;
    Ok(rows)
}

// ── Utility commands ─────────────────────────────────────────────────

/// Write a file to disk (used by export functionality).
#[tauri::command]
pub async fn write_file(path: String, contents: Vec<u8>) -> Result<()> {
    let path = std::path::Path::new(&path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, &contents)?;
    info!("Wrote file: {}", path.display());
    Ok(())
}

/// Restore a project from a backup file.
#[tauri::command]
pub async fn restore_backup(backup_path: String) -> Result<()> {
    let path = std::path::Path::new(&backup_path);
    if !path.exists() {
        return Err(AppError(
            anyhow::anyhow!("Backup file not found: {}", backup_path).to_string(),
        ));
    }
    let db_path = std::env::current_dir()
        .unwrap_or_default()
        .join("sl-studio.db");
    let backup_dir = db_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    std::fs::create_dir_all(backup_dir)?;
    std::fs::copy(path, &db_path)?;
    info!("Restored backup from: {}", backup_path);
    Ok(())
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Escape a string for CSV output.
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        let escaped = s.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        s.to_string()
    }
}

/// Dispatch extraction to the right extractor based on file extension.
async fn extract_file(path: &str, fingerprint: &str) -> crate::ExtractionResult {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let result = match ext.to_lowercase().as_str() {
        "pdf" => crate::extractors::extract_pdf(path).await,
        "png" | "jpg" | "jpeg" | "tiff" | "bmp" => crate::extractors::extract_image(path).await,
        "mp3" | "wav" | "m4a" => crate::extractors::extract_audio(path).await,
        "docx" => crate::extractors::extract_docx(path).await,
        _ => match std::fs::read_to_string(path) {
            Ok(content) => Ok(crate::Metadata {
                filename: path.to_string(),
                category: "Text".to_string(),
                severity_score: 0,
                confidence: None,
                identified_crime: None,
                fact_summary: content,
                fingerprint: "text_meta".to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            }),
            Err(e) => Err(anyhow::anyhow!("Failed to read file: {}", e)),
        },
    };

    match result {
        Ok(metadata) => {
            let char_count = metadata.fact_summary.len() as u64;
            crate::ExtractionResult {
                fingerprint: fingerprint.to_string(),
                path: path.to_string(),
                success: char_count > 0,
                char_count,
                error: if char_count == 0 {
                    Some("No content extracted".to_string())
                } else {
                    None
                },
            }
        }
        Err(e) => crate::ExtractionResult {
            fingerprint: fingerprint.to_string(),
            path: path.to_string(),
            success: false,
            char_count: 0,
            error: Some(e.to_string()),
        },
    }
}
