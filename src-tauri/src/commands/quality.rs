use crate::commands::require_db;
use crate::inference;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn find_duplicate_facts(
    state: State<AppState>,
    threshold: Option<f32>,
    require_same_category: Option<bool>,
    require_same_date: Option<bool>,
) -> Result<Vec<inference::quality::DuplicateGroup>, String> {
    use inference::quality::{find_duplicate_groups, DeduplicationConfig};
    let db = require_db(&state)?;
    let candidates = db.get_dedup_candidates().map_err(|e| e.to_string())?;
    let config = DeduplicationConfig {
        similarity_threshold: threshold.unwrap_or(0.85),
        require_same_category: require_same_category.unwrap_or(true),
        require_same_date: require_same_date.unwrap_or(false),
    };
    Ok(find_duplicate_groups(&candidates, &config))
}

#[tauri::command]
pub fn merge_duplicate_facts(
    state: State<AppState>,
    keeper_id: i64,
    member_ids: Vec<i64>,
) -> Result<usize, String> {
    require_db(&state)?
        .merge_duplicate_facts(keeper_id, &member_ids)
        .map_err(|e| e.to_string())
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CorroborationMatch {
    intelligence_id: i64,
    filename: String,
    fact_summary: String,
    similarity: f32,
    agreement: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CrossValidationResult {
    intelligence_id: i64,
    source_filename: String,
    matches: Vec<CorroborationMatch>,
    consensus_score: f32,
}

#[tauri::command]
pub fn cross_validate_fact(
    state: State<AppState>,
    intelligence_id: i64,
    threshold: Option<f32>,
) -> Result<CrossValidationResult, String> {
    use inference::quality::jaccard_similarity;
    let threshold = threshold.unwrap_or(0.5);
    let db = require_db(&state)?;
    let (target_summary, source_filename, candidates) = db
        .get_corroboration_candidates(intelligence_id)
        .map_err(|e| e.to_string())?;

    let mut matches: Vec<CorroborationMatch> = Vec::new();
    let mut total_sim = 0.0_f32;
    let mut sources = std::collections::HashSet::new();
    for (id, filename, summary, _category) in candidates {
        let sim = jaccard_similarity(&target_summary, &summary);
        if sim < threshold {
            continue;
        }
        let agreement = if sim >= 0.85 {
            "agree".to_string()
        } else if sim >= 0.6 {
            "partial".to_string()
        } else {
            "conflict".to_string()
        };
        total_sim += sim;
        sources.insert(filename.clone());
        matches.push(CorroborationMatch {
            intelligence_id: id,
            filename,
            fact_summary: summary,
            similarity: sim,
            agreement,
        });
    }
    matches.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let diversity = match sources.len() {
        0 => 0.0,
        1 => 0.5,
        2 => 0.75,
        _ => 1.0,
    };
    let avg_sim = if matches.is_empty() {
        0.0
    } else {
        total_sim / matches.len() as f32
    };
    let consensus_score = avg_sim * diversity;

    Ok(CrossValidationResult {
        intelligence_id,
        source_filename,
        matches,
        consensus_score,
    })
}
