use crate::commands::require_db;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn get_schema_version(state: State<AppState>) -> Result<i64, String> {
    let Ok(db) = require_db(&state) else {
        return Ok(0);
    };
    db.schema_version().map_err(|e| e.to_string())
}
