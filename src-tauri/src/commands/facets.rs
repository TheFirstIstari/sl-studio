use crate::AppState;
use tauri::State;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FacetPreset {
    id: i64,
    page: String,
    name: String,
    state_json: String,
    updated_at: Option<String>,
}

#[tauri::command]
pub fn save_facet_preset(
    state: State<AppState>,
    page: String,
    name: String,
    state_json: String,
) -> Result<i64, String> {
    let db = state
        .db
        .lock()
        .map_err(|e| format!("Database mutex poisoned: {e}"))?;
    let Some(db) = db.as_ref() else {
        return Err("Database not initialized".to_string());
    };
    db.save_facet_preset(&page, &name, &state_json)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_facet_presets(
    state: State<AppState>,
    page: String,
) -> Result<Vec<FacetPreset>, String> {
    let db = state
        .db
        .lock()
        .map_err(|e| format!("Database mutex poisoned: {e}"))?;
    let Some(db) = db.as_ref() else {
        return Err("Database not initialized".to_string());
    };
    let rows = db.list_facet_presets(&page).map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|(id, page, name, state_json, updated_at)| FacetPreset {
            id,
            page,
            name,
            state_json,
            updated_at,
        })
        .collect())
}

#[tauri::command]
pub fn delete_facet_preset(state: State<AppState>, preset_id: i64) -> Result<(), String> {
    let db = state
        .db
        .lock()
        .map_err(|e| format!("Database mutex poisoned: {e}"))?;
    let Some(db) = db.as_ref() else {
        return Err("Database not initialized".to_string());
    };
    db.delete_facet_preset(preset_id).map_err(|e| e.to_string())
}
