//! FR-META Tauri commands.
//!
//! - `extract_metadata(path)` — read the file directly, return parsed
//!   metadata. No DB access.
//! - `cache_metadata(fingerprint, path)` — read + persist into
//!   `metadata_cache` so the UI can read it later without re-parsing.
//! - `get_cached_metadata(fingerprint)` — DB lookup.

use std::path::Path;

use tauri::State;

use crate::commands::require_db;
use crate::extractors::metadata::{extract_metadata as extract_metadata_inner, DocumentMetadata};
use crate::AppState;

#[tauri::command]
pub fn extract_metadata(path: String) -> Result<DocumentMetadata, String> {
    extract_metadata_inner(Path::new(&path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cache_metadata(
    state: State<'_, AppState>,
    fingerprint: String,
    path: String,
) -> Result<DocumentMetadata, String> {
    let meta = extract_metadata_inner(Path::new(&path)).map_err(|e| e.to_string())?;
    let json = serde_json::to_string(&meta).map_err(|e| e.to_string())?;
    require_db(&state)?
        .save_metadata_cache(&fingerprint, &meta.source, &json)
        .map_err(|e| e.to_string())?;
    Ok(meta)
}

#[tauri::command]
pub fn get_cached_metadata(
    state: State<'_, AppState>,
    fingerprint: String,
    metadata_type: String,
) -> Result<Option<DocumentMetadata>, String> {
    let db = require_db(&state)?;
    let cached = db
        .get_metadata_cache(&fingerprint, &metadata_type)
        .map_err(|e| e.to_string())?;
    match cached {
        None => Ok(None),
        Some(entry) => {
            let json = entry.metadata_json;
            if json.is_empty() {
                return Ok(None);
            }
            let meta: DocumentMetadata = serde_json::from_str(&json).map_err(|e| e.to_string())?;
            Ok(Some(meta))
        }
    }
}
