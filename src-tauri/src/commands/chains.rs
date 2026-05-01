use crate::commands::require_db;
use crate::core;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn create_evidence_chain(
    state: State<AppState>,
    name: String,
    chain_type: String,
    description: Option<String>,
    created_by: Option<String>,
) -> Result<i64, String> {
    require_db(&state)?
        .create_chain(
            &name,
            &chain_type,
            description.as_deref().unwrap_or(""),
            created_by.as_deref().unwrap_or(""),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_evidence_chains(
    state: State<AppState>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<core::ChainSummary>, String> {
    require_db(&state)?
        .get_all_chains(limit.unwrap_or(100), offset.unwrap_or(0))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_evidence_chain(
    state: State<AppState>,
    chain_id: i64,
) -> Result<Option<core::EvidenceChain>, String> {
    let db = require_db(&state)?;
    let mut chain = db.get_chain(chain_id).map_err(|e| e.to_string())?;
    if let Some(ref mut c) = chain {
        c.items = db.get_chain_items(chain_id).map_err(|e| e.to_string())?;
    }
    Ok(chain)
}

#[tauri::command]
pub fn add_to_evidence_chain(
    state: State<AppState>,
    chain_id: i64,
    intelligence_id: i64,
    relationship_type: String,
    strength: f64,
    notes: Option<String>,
    linked_by: Option<String>,
) -> Result<(), String> {
    require_db(&state)?
        .add_to_chain(
            chain_id,
            intelligence_id,
            &relationship_type,
            strength,
            notes.as_deref().unwrap_or(""),
            linked_by.as_deref().unwrap_or(""),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_from_evidence_chain(
    state: State<AppState>,
    chain_id: i64,
    intelligence_id: i64,
) -> Result<(), String> {
    require_db(&state)?
        .remove_from_chain(chain_id, intelligence_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_evidence_chain(
    state: State<AppState>,
    chain_id: i64,
    name: Option<String>,
    description: Option<String>,
) -> Result<(), String> {
    require_db(&state)?
        .update_chain(chain_id, name.as_deref(), description.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_evidence_chain(state: State<AppState>, chain_id: i64) -> Result<(), String> {
    require_db(&state)?
        .delete_chain(chain_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_evidence_chain_statistics(
    state: State<AppState>,
    chain_id: i64,
) -> Result<core::ChainStatistics, String> {
    require_db(&state)?
        .get_chain_statistics(chain_id)
        .map_err(|e| e.to_string())
}
