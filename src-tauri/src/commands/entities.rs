use crate::core;
use crate::inference;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn suggest_entity_matches(
    state: State<AppState>,
    threshold: Option<f32>,
    per_type_limit: Option<u32>,
    scan_limit: Option<i64>,
) -> Result<Vec<inference::quality::EntityMatchSuggestion>, String> {
    use inference::quality::{find_entity_matches, EntityCandidate, EntityResolutionConfig};
    let db = state
        .db
        .lock()
        .map_err(|e| format!("Database mutex poisoned: {e}"))?;
    let Some(db) = db.as_ref() else {
        return Err("Database not initialized".to_string());
    };
    let raw = db
        .list_distinct_entities(scan_limit.unwrap_or(2000))
        .map_err(|e| e.to_string())?;
    let candidates: Vec<EntityCandidate> = raw
        .into_iter()
        .map(|(id, entity_type, value)| EntityCandidate {
            id,
            entity_type,
            value,
        })
        .collect();
    let config = EntityResolutionConfig {
        similarity_threshold: threshold.unwrap_or(0.80),
        per_type_limit: per_type_limit.unwrap_or(1000) as usize,
    };
    Ok(find_entity_matches(&candidates, &config))
}

#[tauri::command]
pub fn add_entity_alias(
    state: State<AppState>,
    canonical_id: i64,
    alias: String,
    alias_type: Option<String>,
    confidence: Option<f64>,
) -> Result<(), String> {
    let db = state
        .db
        .lock()
        .map_err(|e| format!("Database mutex poisoned: {e}"))?;
    let Some(db) = db.as_ref() else {
        return Err("Database not initialized".to_string());
    };
    db.add_entity_alias(
        canonical_id,
        &alias,
        alias_type.as_deref().unwrap_or("manual"),
        confidence.unwrap_or(1.0),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn resolve_entity_alias(
    state: State<AppState>,
    alias: String,
) -> Result<Vec<core::ResolvedEntity>, String> {
    let db = state
        .db
        .lock()
        .map_err(|e| format!("Database mutex poisoned: {e}"))?;
    let Some(db) = db.as_ref() else {
        return Err("Database not initialized".to_string());
    };
    db.resolve_entity(&alias).map_err(|e| e.to_string())
}
