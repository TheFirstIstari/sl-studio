use crate::commands::require_db;
use crate::core;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn search_facts(
    state: State<AppState>,
    query: String,
    limit: i64,
) -> Result<Vec<core::SearchResult>, String> {
    require_db(&state)?
        .search_facts(&query, limit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_fact_verification(
    state: State<AppState>,
    id: i64,
    status: String,
    review_notes: Option<String>,
) -> Result<(), String> {
    require_db(&state)?
        .update_fact_verification(id, &status, review_notes.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_entities(
    state: State<AppState>,
    query: String,
    limit: i64,
) -> Result<Vec<core::EntitySearchResult>, String> {
    require_db(&state)?
        .search_entities(&query, limit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_combined(
    state: State<AppState>,
    query: String,
    limit: i64,
) -> Result<Vec<core::CombinedSearchResult>, String> {
    require_db(&state)?
        .search_combined(&query, limit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_by_tags(
    state: State<AppState>,
    tags: Vec<String>,
    match_all: bool,
    limit: i64,
) -> Result<Vec<core::SearchResult>, String> {
    require_db(&state)?
        .search_by_tags(&tags, match_all, limit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_timeline_events(
    state: State<AppState>,
    start_date: Option<String>,
    end_date: Option<String>,
    limit: i64,
) -> Result<Vec<core::TimelineEvent>, String> {
    require_db(&state)?
        .get_timeline_events(start_date.as_deref(), end_date.as_deref(), limit)
        .map_err(|e| e.to_string())
}
