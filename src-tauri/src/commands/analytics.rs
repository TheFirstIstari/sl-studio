use crate::commands::require_db;
use crate::core;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn get_overall_statistics(state: State<AppState>) -> Result<core::OverallStatistics, String> {
    require_db(&state)?
        .get_overall_statistics()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_category_distribution(
    state: State<AppState>,
) -> Result<Vec<core::CategoryStats>, String> {
    require_db(&state)?
        .get_category_distribution()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_severity_distribution(
    state: State<AppState>,
) -> Result<Vec<core::SeverityStats>, String> {
    require_db(&state)?
        .get_severity_distribution()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_entity_centrality(
    state: State<AppState>,
    entity_type: Option<String>,
    min_confidence: f64,
) -> Result<Vec<core::EntityCentrality>, String> {
    require_db(&state)?
        .get_entity_centrality(entity_type.as_deref(), min_confidence)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn detect_anomalies(
    state: State<AppState>,
    metric: String,
    threshold_std: f64,
) -> Result<Vec<core::Anomaly>, String> {
    require_db(&state)?
        .detect_anomalies(&metric, threshold_std)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_evidence_weight(state: State<AppState>, intelligence_id: i64) -> Result<f64, String> {
    require_db(&state)?
        .calculate_evidence_weight(intelligence_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_weighted_evidence(
    state: State<AppState>,
    min_weight: f64,
    limit: i64,
) -> Result<Vec<core::WeightedEvidence>, String> {
    require_db(&state)?
        .get_weighted_evidence(min_weight, limit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_entity_relationships(
    state: State<AppState>,
    entity_id: Option<i64>,
    min_confidence: f64,
) -> Result<Vec<core::EntityRelationship>, String> {
    require_db(&state)?
        .get_entity_relationships(entity_id, min_confidence)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_connected_entities(
    state: State<AppState>,
    entity_id: i64,
    min_confidence: f64,
) -> Result<Vec<core::ConnectedEntity>, String> {
    require_db(&state)?
        .get_connected_entities(entity_id, 1, min_confidence)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_location_entities(
    state: State<AppState>,
    min_confidence: f64,
) -> Result<Vec<core::LocationEntity>, String> {
    require_db(&state)?
        .get_location_entities(min_confidence)
        .map_err(|e| e.to_string())
}
