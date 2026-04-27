use crate::core;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn add_tag(state: State<AppState>, intelligence_id: i64, tag: String) -> Result<(), String> {
    let db = state
        .db
        .lock()
        .map_err(|e| format!("Database mutex poisoned: {e}"))?;
    if let Some(db) = db.as_ref() {
        db.add_tag(intelligence_id, &tag).map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
pub fn remove_tag(state: State<AppState>, intelligence_id: i64, tag: String) -> Result<(), String> {
    let db = state
        .db
        .lock()
        .map_err(|e| format!("Database mutex poisoned: {e}"))?;
    if let Some(db) = db.as_ref() {
        db.remove_tag(intelligence_id, &tag)
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
pub fn get_all_tags(state: State<AppState>) -> Result<Vec<String>, String> {
    let db = state
        .db
        .lock()
        .map_err(|e| format!("Database mutex poisoned: {e}"))?;
    if let Some(db) = db.as_ref() {
        db.get_all_tags().map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
pub fn add_annotation(
    state: State<AppState>,
    intelligence_id: i64,
    content: String,
    annotation_type: String,
) -> Result<i64, String> {
    let db = state
        .db
        .lock()
        .map_err(|e| format!("Database mutex poisoned: {e}"))?;
    if let Some(db) = db.as_ref() {
        db.add_annotation(intelligence_id, &content, &annotation_type)
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
pub fn update_annotation(
    state: State<AppState>,
    annotation_id: i64,
    content: String,
) -> Result<(), String> {
    let db = state
        .db
        .lock()
        .map_err(|e| format!("Database mutex poisoned: {e}"))?;
    if let Some(db) = db.as_ref() {
        db.update_annotation(annotation_id, &content)
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
pub fn delete_annotation(state: State<AppState>, annotation_id: i64) -> Result<(), String> {
    let db = state
        .db
        .lock()
        .map_err(|e| format!("Database mutex poisoned: {e}"))?;
    if let Some(db) = db.as_ref() {
        db.delete_annotation(annotation_id)
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
pub fn get_annotations(
    state: State<AppState>,
    intelligence_id: i64,
) -> Result<Vec<core::Annotation>, String> {
    let db = state
        .db
        .lock()
        .map_err(|e| format!("Database mutex poisoned: {e}"))?;
    if let Some(db) = db.as_ref() {
        db.get_annotations(intelligence_id)
            .map_err(|e| e.to_string())
    } else {
        Err("Database not initialized".to_string())
    }
}
