use rusqlite::Result;

use super::super::database::Database;
use super::super::database::*;

impl Database {
    pub fn calculate_evidence_weight(&self, intelligence_id: i64) -> Result<f64> {
        let conn = self.intel_conn()?;

        let mut stmt = conn.prepare(
            "SELECT severity_score, confidence, quality_score, created_at
             FROM intelligence WHERE id = ?1",
        )?;

        let (severity, confidence, quality): (i32, Option<f64>, Option<f64>) = stmt
            .query_row([intelligence_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?;

        let severity_weight = (severity as f64 / 5.0).min(1.0) * 0.4;
        let confidence_weight = confidence.unwrap_or(0.5) * 0.35;
        let quality_weight = quality.unwrap_or(0.5) * 0.25;

        Ok(severity_weight + confidence_weight + quality_weight)
    }

    pub fn get_weighted_evidence(
        &self,
        min_weight: f64,
        limit: i64,
    ) -> Result<Vec<WeightedEvidence>> {
        let conn = self.intel_conn()?;

        let mut stmt = conn.prepare(
            "SELECT i.id, i.fingerprint, i.filename, i.fact_summary, i.category, 
                    i.severity_score, i.confidence, i.quality_score, i.created_at,
                    (i.severity_score / 5.0 * 0.4 + COALESCE(i.confidence, 0.5) * 0.35 + COALESCE(i.quality_score, 0.5) * 0.25) as weight
             FROM intelligence i
             WHERE i.is_deleted = FALSE
             ORDER BY weight DESC
             LIMIT ?1"
        )?;

        let entries = stmt.query_map([limit], |row| {
            Ok(WeightedEvidence {
                id: row.get(0)?,
                fingerprint: row.get(1)?,
                filename: row.get(2)?,
                summary: row.get(3)?,
                category: row.get(4)?,
                severity: row.get(5)?,
                confidence: row.get(6)?,
                quality: row.get(7)?,
                created_at: row.get(8)?,
                weight: row.get(9)?,
            })
        })?;

        Ok(entries
            .filter_map(|r| r.ok())
            .filter(|e| e.weight >= min_weight)
            .collect())
    }
}
