use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn delete_facts(state: State<AppState>, ids: Vec<i64>) -> Result<usize, String> {
    let db = state
        .db
        .lock()
        .map_err(|e| format!("Database mutex poisoned: {e}"))?;
    if let Some(db) = db.as_ref() {
        let mut count = 0;
        for id in ids {
            if db.delete_intelligence(id).is_ok() {
                count += 1;
            }
        }
        Ok(count)
    } else {
        Err("Database not initialized".to_string())
    }
}
