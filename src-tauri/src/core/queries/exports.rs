use rusqlite::Result;

use super::super::database::Database;
use super::super::database::*;

impl Database {
    pub fn export_facts_json(&self, filters: &ExportFilters) -> Result<String> {
        let facts = self.get_weighted_evidence(filters.min_weight, filters.limit)?;

        let export: Vec<serde_json::Value> = facts
            .into_iter()
            .map(|f| {
                serde_json::json!({
                    "id": f.id,
                    "fingerprint": f.fingerprint,
                    "filename": f.filename,
                    "summary": f.summary,
                    "category": f.category,
                    "severity": f.severity,
                    "confidence": f.confidence,
                    "quality": f.quality,
                    "weight": f.weight,
                    "created_at": f.created_at,
                })
            })
            .collect();

        serde_json::to_string_pretty(&export)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
    }

    pub fn export_entities_csv(
        &self,
        entity_type: Option<&str>,
        min_confidence: f64,
    ) -> Result<String> {
        let centrality = self.get_entity_centrality(entity_type, min_confidence)?;

        let mut csv = String::from("entity_id,entity_type,value,document_count,occurrence_count,avg_confidence,centrality_score\n");

        for e in centrality {
            csv.push_str(&format!(
                "{},{},\"{}\",{},{},{},{:.3}\n",
                e.entity_id,
                e.entity_type,
                e.value.replace('"', "\"\""),
                e.document_count,
                e.occurrence_count,
                e.avg_confidence.unwrap_or(0.0),
                e.centrality_score
            ));
        }

        Ok(csv)
    }

    pub fn export_timeline_json(
        &self,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Result<String> {
        let events = self.get_timeline_events(start_date, end_date, 10000)?;

        serde_json::to_string_pretty(&events)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
    }
}
