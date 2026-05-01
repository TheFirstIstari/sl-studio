use crate::core::{self, Database};
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tracing::info;

#[derive(Serialize, Deserialize, Clone)]
pub struct ProjectComparison {
    pub project1_name: String,
    pub project2_name: String,
    pub entity_overlap: Vec<EntityOverlap>,
    pub common_entities: Vec<core::EntityCentrality>,
    pub timeline_correlation: TimelineCorrelation,
    pub fact_similarity: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EntityOverlap {
    pub entity_value: String,
    pub entity_type: String,
    pub count_project1: i32,
    pub count_project2: i32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TimelineCorrelation {
    pub correlation_score: f64,
    pub aligned_events: i32,
    pub project1_date_range: (String, String),
    pub project2_date_range: (String, String),
}

#[derive(Serialize, Deserialize)]
pub struct ProjectSummary {
    pub name: String,
    pub path: String,
    pub fact_count: i64,
    pub entity_count: i64,
    pub timeline_count: i64,
}

#[derive(Serialize, Clone)]
pub struct BackupInfo {
    pub backup_path: String,
    pub size_bytes: u64,
    pub created_at: String,
    pub includes_evidence: bool,
}

fn open_project_db(path: &str) -> Result<Database, String> {
    let db_path = std::path::Path::new(path);
    if !db_path.exists() {
        return Err(format!("Database file not found: {}", path));
    }

    let registry_db = db_path.join("registry.db");
    let intelligence_db = db_path.join("intelligence.db");

    if !registry_db.exists() || !intelligence_db.exists() {
        return Err("Invalid project directory - missing database files".to_string());
    }

    Database::new(
        registry_db.to_string_lossy().as_ref(),
        intelligence_db.to_string_lossy().as_ref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn compare_projects(
    state: State<AppState>,
    project2_path: String,
) -> Result<ProjectComparison, String> {
    let (entities1, timeline1, project1_name) = {
        let db = {
            let guard = state
                .db
                .read()
                .map_err(|e| format!("Database lock poisoned: {e}"))?;
            guard.as_ref().ok_or("Database not initialized")?.clone()
        };

        let entities1 = db
            .get_entity_centrality(None, 0.0)
            .map_err(|e| e.to_string())?;
        let timeline1 = db
            .get_timeline_events(None, None, 1000)
            .map_err(|e| e.to_string())?;

        let config = state
            .config
            .read()
            .map_err(|e| format!("Config lock poisoned: {e}"))?;
        let project1_name = config.project.name.clone();

        (entities1, timeline1, project1_name)
    };

    {
        let db2 = open_project_db(&project2_path)?;

        let entities2 = db2
            .get_entity_centrality(None, 0.0)
            .map_err(|e| e.to_string())?;
        let timeline2 = db2
            .get_timeline_events(None, None, 1000)
            .map_err(|e| e.to_string())?;

        let mut entity_overlap = Vec::new();
        let mut common_entities = Vec::new();

        let mut entity_map1: std::collections::HashMap<String, i32> =
            std::collections::HashMap::new();
        for e in &entities1 {
            *entity_map1.entry(e.value.clone()).or_insert(0) += e.occurrence_count;
        }

        let mut entity_map2: std::collections::HashMap<String, i32> =
            std::collections::HashMap::new();
        for e in &entities2 {
            *entity_map2.entry(e.value.clone()).or_insert(0) += e.occurrence_count;
        }

        for (value, count1) in &entity_map1 {
            if let Some(&count2) = entity_map2.get(value) {
                let entity_type = entities1
                    .iter()
                    .find(|e| &e.value == value)
                    .map(|e| e.entity_type.clone())
                    .unwrap_or_default();

                entity_overlap.push(EntityOverlap {
                    entity_value: value.clone(),
                    entity_type,
                    count_project1: *count1,
                    count_project2: count2,
                });

                if let Some(e1) = entities1.iter().find(|e| &e.value == value) {
                    common_entities.push(e1.clone());
                }
            }
        }

        let mut dates1: std::collections::HashSet<String> = std::collections::HashSet::new();
        for e in &timeline1 {
            dates1.insert(e.date.clone());
        }

        let mut dates2: std::collections::HashSet<String> = std::collections::HashSet::new();
        for e in &timeline2 {
            dates2.insert(e.date.clone());
        }

        let intersection: std::collections::HashSet<_> = dates1.intersection(&dates2).collect();
        let union: std::collections::HashSet<_> = dates1.union(&dates2).collect();

        let correlation_score = if union.is_empty() {
            0.0
        } else {
            intersection.len() as f64 / union.len() as f64
        };

        let timeline_correlation = TimelineCorrelation {
            correlation_score,
            aligned_events: intersection.len() as i32,
            project1_date_range: (
                timeline1
                    .first()
                    .map(|e| e.date.clone())
                    .unwrap_or_default(),
                timeline1.last().map(|e| e.date.clone()).unwrap_or_default(),
            ),
            project2_date_range: (
                timeline2
                    .first()
                    .map(|e| e.date.clone())
                    .unwrap_or_default(),
                timeline2.last().map(|e| e.date.clone()).unwrap_or_default(),
            ),
        };

        let fact_similarity = if common_entities.is_empty() {
            0.0
        } else {
            let total_entities = entity_map1.len() + entity_map2.len();
            if total_entities == 0 {
                0.0
            } else {
                2.0 * common_entities.len() as f64 / total_entities as f64
            }
        };

        Ok(ProjectComparison {
            project1_name,
            project2_name: std::path::Path::new(&project2_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Project 2".to_string()),
            entity_overlap,
            common_entities,
            timeline_correlation,
            fact_similarity,
        })
    }
}

#[tauri::command]
pub fn get_project_summary(state: State<AppState>) -> Result<ProjectSummary, String> {
    let db = {
        let guard = state
            .db
            .read()
            .map_err(|e| format!("Database lock poisoned: {e}"))?;
        guard.as_ref().ok_or("Database not initialized")?.clone()
    };

    let stats = db.get_overall_statistics().map_err(|e| e.to_string())?;
    let timeline = db
        .get_timeline_events(None, None, 1000)
        .map_err(|e| e.to_string())?;

    let config = state
        .config
        .read()
        .map_err(|e| format!("Config lock poisoned: {e}"))?;

    let project_path = std::path::Path::new(&config.project.registry_db)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    Ok(ProjectSummary {
        name: config.project.name.clone(),
        path: project_path,
        fact_count: stats.total_facts,
        entity_count: stats.total_entities,
        timeline_count: timeline.len() as i64,
    })
}

#[tauri::command]
pub fn create_backup(state: State<AppState>, include_evidence: bool) -> Result<BackupInfo, String> {
    use std::io::Write;

    let config = state
        .config
        .read()
        .map_err(|e| format!("Config lock poisoned: {e}"))?;
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let backup_name = format!("slstudio_backup_{}.zip", timestamp);

    let export_dir = dirs::data_dir()
        .unwrap_or_default()
        .join("slstudio")
        .join("backups");

    if !export_dir.exists() {
        std::fs::create_dir_all(&export_dir).map_err(|e| e.to_string())?;
    }

    let backup_path = export_dir.join(&backup_name);
    let file = std::fs::File::create(&backup_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let registry_db = std::path::Path::new(&config.project.registry_db);
    let intelligence_db = std::path::Path::new(&config.project.intelligence_db);

    if registry_db.exists() {
        let data = std::fs::read(registry_db).map_err(|e| e.to_string())?;
        zip.start_file("registry.db", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(&data).map_err(|e| e.to_string())?;
    }

    if intelligence_db.exists() {
        let data = std::fs::read(intelligence_db).map_err(|e| e.to_string())?;
        zip.start_file("intelligence.db", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(&data).map_err(|e| e.to_string())?;
    }

    let config_data = serde_json::to_string_pretty(&*config).map_err(|e| e.to_string())?;
    zip.start_file("config.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(config_data.as_bytes())
        .map_err(|e| e.to_string())?;

    if include_evidence {
        let evidence_root = std::path::Path::new(&config.project.evidence_root);
        if evidence_root.exists() {
            for entry in walkdir::WalkDir::new(evidence_root)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    let path = entry.path();
                    let name = path.strip_prefix(evidence_root).unwrap().to_string_lossy();
                    let data = std::fs::read(path).map_err(|e| e.to_string())?;
                    zip.start_file(format!("evidence/{}", name), options)
                        .map_err(|e| e.to_string())?;
                    zip.write_all(&data).map_err(|e| e.to_string())?;
                }
            }
        }
    }

    zip.finish().map_err(|e| e.to_string())?;

    let metadata = std::fs::metadata(&backup_path).map_err(|e| e.to_string())?;

    info!("Backup created: {}", backup_path.display());

    Ok(BackupInfo {
        backup_path: backup_path.to_string_lossy().to_string(),
        size_bytes: metadata.len(),
        created_at: chrono::Local::now().to_rfc3339(),
        includes_evidence: include_evidence,
    })
}

#[tauri::command]
pub fn restore_backup(state: State<AppState>, backup_path: String) -> Result<(), String> {
    let file = std::fs::File::open(&backup_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    let (registry_db, intelligence_db) = {
        let config = state
            .config
            .read()
            .map_err(|e| format!("Config lock poisoned: {e}"))?;
        (
            config.project.registry_db.clone(),
            config.project.intelligence_db.clone(),
        )
    };

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name().to_string();

        match name.as_str() {
            "registry.db" => {
                let path = std::path::Path::new(&registry_db);
                let mut out = std::fs::File::create(path).map_err(|e| e.to_string())?;
                std::io::copy(&mut file, &mut out).map_err(|e| e.to_string())?;
            }
            "intelligence.db" => {
                let path = std::path::Path::new(&intelligence_db);
                let mut out = std::fs::File::create(path).map_err(|e| e.to_string())?;
                std::io::copy(&mut file, &mut out).map_err(|e| e.to_string())?;
            }
            n if n.starts_with("evidence/") => {
                let rel_path = &n[9..];
                let config = state
                    .config
                    .read()
                    .map_err(|e| format!("Config lock poisoned: {e}"))?;
                let dest = std::path::Path::new(&config.project.evidence_root).join(rel_path);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let mut out = std::fs::File::create(dest).map_err(|e| e.to_string())?;
                std::io::copy(&mut file, &mut out).map_err(|e| e.to_string())?;
            }
            _ => {}
        }
    }

    let db = Database::new(&registry_db, &intelligence_db)
        .map_err(|e| format!("Failed to reopen database: {}", e))?;

    // Audit: record the restore before handing db ownership to AppState.
    let _ = db.log_audit("restore_backup", &backup_path, None);

    *state
        .db
        .write()
        .map_err(|e| format!("Database lock poisoned: {e}"))? = Some(Arc::new(db));

    info!("Backup restored from: {}", backup_path);
    Ok(())
}
