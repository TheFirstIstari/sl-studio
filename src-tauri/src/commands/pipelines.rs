use crate::commands::require_db;
use crate::inference;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn list_pipelines(state: State<AppState>) -> Result<Vec<inference::Pipeline>, String> {
    let db = require_db(&state)?;
    let rows = db.list_pipelines().map_err(|e| e.to_string())?;
    let mut pipelines = Vec::with_capacity(rows.len());
    for (id, name, description, passes_json, is_builtin) in rows {
        let passes: Vec<inference::PipelinePass> =
            serde_json::from_str(&passes_json).map_err(|e| e.to_string())?;
        pipelines.push(inference::Pipeline {
            id,
            name,
            description,
            passes,
            is_builtin,
        });
    }
    let mut seen_ids: std::collections::HashSet<String> =
        pipelines.iter().map(|p| p.id.clone()).collect();
    for builtin in inference::get_builtin_pipelines() {
        if !seen_ids.contains(&builtin.id) {
            seen_ids.insert(builtin.id.clone());
            pipelines.push(builtin);
        }
    }
    Ok(pipelines)
}

#[tauri::command]
pub fn save_pipeline(state: State<AppState>, pipeline: inference::Pipeline) -> Result<(), String> {
    if pipeline.is_builtin {
        return Err("Cannot persist builtin pipelines".to_string());
    }
    let passes_json = serde_json::to_string(&pipeline.passes).map_err(|e| e.to_string())?;
    require_db(&state)?
        .save_pipeline(
            &pipeline.id,
            &pipeline.name,
            &pipeline.description,
            &passes_json,
            false,
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_pipeline(
    state: State<AppState>,
    pipeline_id: String,
) -> Result<Option<inference::Pipeline>, String> {
    let db = require_db(&state)?;
    if let Some((id, name, description, passes_json, is_builtin)) =
        db.get_pipeline(&pipeline_id).map_err(|e| e.to_string())?
    {
        let passes: Vec<inference::PipelinePass> =
            serde_json::from_str(&passes_json).map_err(|e| e.to_string())?;
        return Ok(Some(inference::Pipeline {
            id,
            name,
            description,
            passes,
            is_builtin,
        }));
    }
    Ok(inference::get_pipeline_by_id(&pipeline_id))
}

#[tauri::command]
pub fn delete_pipeline(state: State<AppState>, pipeline_id: String) -> Result<(), String> {
    require_db(&state)?
        .delete_pipeline(&pipeline_id)
        .map_err(|e| e.to_string())
}
