use rusqlite::{params, Result};

use super::super::database::Database;
use super::super::database::*;

impl Database {
    pub fn insert_intelligence(&self, entry: &IntelligenceEntry) -> Result<()> {
        let conn = self.intel_conn()?;
        conn.execute(
            "INSERT OR REPLACE INTO intelligence
             (registry_id, fingerprint, filename, source_quote, page_number, evidence_full, evidence_hash,
              associated_date, location, people, fact_summary, category, identified_crime, severity_score,
              confidence, quality_score, source_language, translated_quote, pipeline_id, pass_name,
              is_deleted, deleted_at, processing_time_ms, created_at)
              VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)",
            params![
                entry.registry_id,
                entry.fingerprint,
                entry.filename,
                entry.source_quote,
                entry.page_number,
                entry.evidence_full,
                entry.evidence_hash,
                entry.associated_date,
                entry.location,
                entry.people,
                entry.fact_summary,
                entry.category,
                entry.identified_crime,
                entry.severity_score,
                entry.confidence,
                entry.quality_score,
                entry.source_language,
                entry.translated_quote,
                entry.pipeline_id,
                entry.pass_name,
                entry.is_deleted,
                entry.deleted_at,
                entry.processing_time_ms,
                entry.created_at
            ],
        )?;

        // Invalidate cache since data changed
        self.invalidate_cache();

        Ok(())
    }

    /// Bulk-insert intelligence rows in a single transaction. Saves
    /// per-row pool checkout + autocommit overhead when a single LLM
    /// pass produces dozens of facts.
    pub fn insert_intelligence_batch(&self, entries: &[IntelligenceEntry]) -> Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }
        let mut conn = self.intel_conn()?;
        let tx = conn.transaction()?;
        let inserted = {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO intelligence
                 (registry_id, fingerprint, filename, source_quote, page_number, evidence_full, evidence_hash,
                  associated_date, location, people, fact_summary, category, identified_crime, severity_score,
                  confidence, quality_score, source_language, translated_quote, pipeline_id, pass_name,
                  is_deleted, deleted_at, processing_time_ms, created_at)
                  VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)",
            )?;
            let mut count = 0usize;
            for entry in entries {
                stmt.execute(params![
                    entry.registry_id,
                    entry.fingerprint,
                    entry.filename,
                    entry.source_quote,
                    entry.page_number,
                    entry.evidence_full,
                    entry.evidence_hash,
                    entry.associated_date,
                    entry.location,
                    entry.people,
                    entry.fact_summary,
                    entry.category,
                    entry.identified_crime,
                    entry.severity_score,
                    entry.confidence,
                    entry.quality_score,
                    entry.source_language,
                    entry.translated_quote,
                    entry.pipeline_id,
                    entry.pass_name,
                    entry.is_deleted,
                    entry.deleted_at,
                    entry.processing_time_ms,
                    entry.created_at,
                ])?;
                count += 1;
            }
            count
        };
        tx.commit()?;
        self.invalidate_cache();
        Ok(inserted)
    }

    pub fn get_intelligence(&self, limit: i64, offset: i64) -> Result<Vec<IntelligenceEntry>> {
        let conn = self.intel_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, registry_id, fingerprint, filename, source_quote, page_number, evidence_full, evidence_hash,
                    associated_date, location, people, fact_summary, category, identified_crime, severity_score, 
                    confidence, quality_score, source_language, translated_quote, pipeline_id, pass_name, 
                    is_deleted, deleted_at, processing_time_ms, created_at
             FROM intelligence
             WHERE is_deleted = FALSE
             ORDER BY severity_score DESC, created_at DESC
             LIMIT ?1 OFFSET ?2",
        )?;

        let entries = stmt.query_map(params![limit, offset], |row| {
            Ok(IntelligenceEntry {
                id: row.get(0)?,
                registry_id: row.get(1)?,
                fingerprint: row.get(2)?,
                filename: row.get(3)?,
                source_quote: row.get(4)?,
                page_number: row.get(5)?,
                evidence_full: row.get(6)?,
                evidence_hash: row.get(7)?,
                associated_date: row.get(8)?,
                location: row.get(9)?,
                people: row.get(10)?,
                fact_summary: row.get(11)?,
                category: row.get(12)?,
                identified_crime: row.get(13)?,
                severity_score: row.get(14)?,
                confidence: row.get(15)?,
                quality_score: row.get(16)?,
                source_language: row.get(17)?,
                translated_quote: row.get(18)?,
                pipeline_id: row.get(19)?,
                pass_name: row.get(20)?,
                is_deleted: row.get(21)?,
                deleted_at: row.get(22)?,
                processing_time_ms: row.get(23)?,
                created_at: row.get(24)?,
            })
        })?;

        entries.collect()
    }

    pub fn delete_intelligence(&self, id: i64) -> Result<()> {
        let conn = self.intel_conn()?;
        let affected = conn.execute(
            "UPDATE intelligence
                SET is_deleted = TRUE,
                    deleted_at = CURRENT_TIMESTAMP
              WHERE id = ?1 AND is_deleted = FALSE",
            params![id],
        )?;
        if affected == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    /// Load lightweight candidates for the deduplication scanner.
    ///
    /// Returns only non-deleted intelligence rows projected down to the
    /// fields the dedup engine actually needs.

    /// Update the detected source language (ISO 639-3) for a single
    /// intelligence row. Used by FR-LANG to populate the column after
    /// extraction-time language detection. Returns the number of rows
    /// affected (0 if the id doesn't exist).
    pub fn update_intelligence_language(&self, id: i64, code: &str) -> Result<usize> {
        let conn = self.intel_conn()?;
        let affected = conn.execute(
            "UPDATE intelligence SET source_language = ?1 WHERE id = ?2",
            params![code, id],
        )?;
        if affected > 0 {
            self.invalidate_cache();
        }
        Ok(affected)
    }

    pub fn update_fact_verification(
        &self,
        id: i64,
        status: &str,
        review_notes: Option<&str>,
    ) -> Result<()> {
        let conn = self.intel_conn()?;
        conn.execute(
            "UPDATE intelligence SET verification_status = ?1, review_notes = ?2 WHERE id = ?3",
            params![status, review_notes, id],
        )?;
        Ok(())
    }
}
