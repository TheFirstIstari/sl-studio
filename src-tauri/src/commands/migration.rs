use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn get_schema_version(state: State<AppState>) -> Result<i64, String> {
    let db_opt = {
        let guard = state
            .db
            .lock()
            .map_err(|e| format!("Database mutex poisoned: {e}"))?;
        guard.as_ref().cloned()
    };
    if let Some(db) = db_opt {
        db.schema_version().map_err(|e| e.to_string())
    } else {
        Ok(0)
    }
}
