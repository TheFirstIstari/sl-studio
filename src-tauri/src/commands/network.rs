use crate::inference;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn detect_entity_communities(
    state: State<AppState>,
    min_cooccurrence: Option<i32>,
) -> Result<Vec<inference::network::EntityCommunity>, String> {
    let db = state
        .db
        .lock()
        .map_err(|e| format!("Database mutex poisoned: {e}"))?;
    if let Some(db) = db.as_ref() {
        let (nodes, edges) = db
            .get_entity_graph(min_cooccurrence.unwrap_or(2))
            .map_err(|e| e.to_string())?;
        Ok(inference::network::detect_communities(
            &edges,
            nodes.len(),
            &nodes,
        ))
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
pub fn compute_betweenness_centrality(
    state: State<AppState>,
    min_cooccurrence: Option<i32>,
    top_k: Option<usize>,
) -> Result<Vec<inference::network::EntityBetweenness>, String> {
    let db = state
        .db
        .lock()
        .map_err(|e| format!("Database mutex poisoned: {e}"))?;
    if let Some(db) = db.as_ref() {
        let (nodes, edges) = db
            .get_entity_graph(min_cooccurrence.unwrap_or(2))
            .map_err(|e| e.to_string())?;
        let mut bc = inference::network::betweenness_centrality(&edges, &nodes);
        bc.sort_by(|a, b| {
            b.betweenness
                .partial_cmp(&a.betweenness)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let k = top_k.unwrap_or(100);
        bc.truncate(k);
        Ok(bc)
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
pub fn compute_clustering_coefficients(
    state: State<AppState>,
    min_cooccurrence: Option<i32>,
) -> Result<Vec<(i64, f64)>, String> {
    let db = state
        .db
        .lock()
        .map_err(|e| format!("Database mutex poisoned: {e}"))?;
    if let Some(db) = db.as_ref() {
        let (nodes, edges) = db
            .get_entity_graph(min_cooccurrence.unwrap_or(2))
            .map_err(|e| e.to_string())?;
        Ok(inference::network::clustering_coefficient(&edges, &nodes))
    } else {
        Err("Database not initialized".to_string())
    }
}
