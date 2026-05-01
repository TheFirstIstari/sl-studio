use crate::commands::require_db;
use crate::core;
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportReport {
    pub facts: Vec<core::WeightedEvidence>,
    pub statistics: core::OverallStatistics,
    pub categories: Vec<core::CategoryStats>,
}

#[derive(Serialize)]
struct ExcelData {
    facts: Vec<core::WeightedEvidence>,
    categories: Vec<core::CategoryStats>,
    entities: Vec<core::EntityCentrality>,
    timeline: Vec<core::TimelineEvent>,
}

#[tauri::command]
pub fn export_facts_json(
    state: State<AppState>,
    min_weight: f64,
    limit: i64,
    categories: Option<Vec<String>>,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<String, String> {
    let filters = core::ExportFilters {
        min_weight,
        limit,
        categories,
        start_date,
        end_date,
    };
    require_db(&state)?
        .export_facts_json(&filters)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_entities_csv(
    state: State<AppState>,
    entity_type: Option<String>,
    min_confidence: f64,
) -> Result<String, String> {
    require_db(&state)?
        .export_entities_csv(entity_type.as_deref(), min_confidence)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_timeline_json(
    state: State<AppState>,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<String, String> {
    require_db(&state)?
        .export_timeline_json(start_date.as_deref(), end_date.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_full_report_json(state: State<AppState>) -> Result<String, String> {
    let db = require_db(&state)?;
    let facts = db
        .get_weighted_evidence(0.0, 10000)
        .map_err(|e| e.to_string())?;
    let statistics = db.get_overall_statistics().map_err(|e| e.to_string())?;
    let categories = db.get_category_distribution().map_err(|e| e.to_string())?;
    let report = ExportReport {
        facts,
        statistics,
        categories,
    };
    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_facts_csv(
    state: State<AppState>,
    min_weight: f64,
    limit: i64,
) -> Result<String, String> {
    let facts = require_db(&state)?
        .get_weighted_evidence(min_weight, limit)
        .map_err(|e| e.to_string())?;
    let mut csv = String::from(
        "id,fingerprint,filename,category,severity,confidence,quality,weight,summary,created_at\n",
    );
    for f in facts {
        csv.push_str(&format!(
            "{},{},\"{}\",\"{}\",{},{},{},{},\"{}\",\"{}\"\n",
            f.id,
            f.fingerprint,
            f.filename.replace('"', "\"\""),
            f.category.unwrap_or_default(),
            f.severity,
            f.confidence.unwrap_or(0.0),
            f.quality.unwrap_or(0.0),
            f.weight,
            f.summary.replace('"', "\"\""),
            f.created_at.unwrap_or_default()
        ));
    }
    Ok(csv)
}

#[tauri::command]
pub fn write_file(path: String, contents: Vec<u8>) -> Result<(), String> {
    std::fs::write(&path, contents).map_err(|e| e.to_string())?;
    info!("Wrote file: {}", path);
    Ok(())
}

#[tauri::command]
pub fn export_pdf_report(state: State<AppState>) -> Result<Vec<u8>, String> {
    use printpdf::*;
    use std::io::BufWriter;

    let db = require_db(&state)?;

    let facts = db
        .get_weighted_evidence(0.0, 100)
        .map_err(|e| e.to_string())?;
    let stats = db.get_overall_statistics().map_err(|e| e.to_string())?;
    let categories = db.get_category_distribution().map_err(|e| e.to_string())?;

    let (doc, page1, layer1) =
        PdfDocument::new("SL Studio Forensic Report", Mm(210.0), Mm(297.0), "Layer 1");

    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| e.to_string())?;
    let font_bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| e.to_string())?;

    let current_layer = doc.get_page(page1).get_layer(layer1);

    current_layer.use_text(
        "SL Studio - Forensic Document Analysis Report",
        24.0,
        Mm(20.0),
        Mm(277.0),
        &font_bold,
    );
    current_layer.use_text(
        format!(
            "Generated: {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        ),
        10.0,
        Mm(20.0),
        Mm(268.0),
        &font,
    );

    current_layer.use_text("Summary Statistics", 16.0, Mm(20.0), Mm(250.0), &font_bold);
    current_layer.use_text(
        format!("Total Facts: {}", stats.total_facts),
        12.0,
        Mm(25.0),
        Mm(240.0),
        &font,
    );
    current_layer.use_text(
        format!("Total Entities: {}", stats.total_entities),
        12.0,
        Mm(25.0),
        Mm(232.0),
        &font,
    );
    current_layer.use_text(
        format!("Unique Entities: {}", stats.unique_entities),
        12.0,
        Mm(25.0),
        Mm(224.0),
        &font,
    );
    current_layer.use_text(
        format!("Total Chains: {}", stats.total_chains),
        12.0,
        Mm(25.0),
        Mm(216.0),
        &font,
    );

    current_layer.use_text(
        "Category Distribution",
        16.0,
        Mm(20.0),
        Mm(198.0),
        &font_bold,
    );
    let mut y_pos = 188.0;
    for cat in categories.iter().take(10) {
        current_layer.use_text(
            format!("{}: {} items", cat.category, cat.count),
            11.0,
            Mm(25.0),
            Mm(y_pos),
            &font,
        );
        y_pos -= 7.0;
    }

    current_layer.use_text("Top Facts", 16.0, Mm(20.0), Mm(y_pos - 15.0), &font_bold);
    y_pos -= 25.0;
    for (i, fact) in facts.iter().take(15).enumerate() {
        if y_pos < 30.0 {
            break;
        }
        let summary = if fact.summary.len() > 60 {
            format!("{}...", &fact.summary[..60])
        } else {
            fact.summary.clone()
        };
        current_layer.use_text(
            format!(
                "{}. [{}] {}",
                i + 1,
                fact.category.as_deref().unwrap_or("N/A"),
                summary
            ),
            9.0,
            Mm(25.0),
            Mm(y_pos),
            &font,
        );
        y_pos -= 6.0;
    }

    let mut buffer = BufWriter::new(Vec::new());
    doc.save(&mut buffer).map_err(|e| e.to_string())?;
    let pdf_bytes = buffer.into_inner().map_err(|e| e.to_string())?;

    Ok(pdf_bytes)
}

#[tauri::command]
pub fn export_excel_data(state: State<AppState>) -> Result<String, String> {
    let db = require_db(&state)?;

    let facts = db
        .get_weighted_evidence(0.0, 1000)
        .map_err(|e| e.to_string())?;
    let categories = db.get_category_distribution().map_err(|e| e.to_string())?;
    let entities = db
        .get_entity_centrality(None, 0.0)
        .map_err(|e| e.to_string())?;
    let timeline = db
        .get_timeline_events(None, None, 1000)
        .map_err(|e| e.to_string())?;

    let data = ExcelData {
        facts,
        categories,
        entities,
        timeline,
    };

    serde_json::to_string_pretty(&data).map_err(|e| e.to_string())
}
