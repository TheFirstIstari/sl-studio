use crate::commands::require_db;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn delete_facts(state: State<AppState>, ids: Vec<i64>) -> Result<usize, String> {
    let db = require_db(&state)?;
    let total = ids.len();
    let mut count = 0;
    for id in ids {
        if db.delete_intelligence(id).is_ok() {
            count += 1;
        }
    }
    let _ = db.log_audit(
        "delete_facts",
        &format!("deleted={count},requested={total}"),
        None,
    );
    Ok(count)
}
