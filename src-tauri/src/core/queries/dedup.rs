use rusqlite::{params, Result};

use super::super::database::Database;

impl Database {
    pub fn get_dedup_candidates(&self) -> Result<Vec<crate::inference::quality::DedupCandidate>> {
        let conn = self.intel_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, fact_summary, category, associated_date, severity_score, confidence
             FROM intelligence
             WHERE is_deleted = FALSE",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(crate::inference::quality::DedupCandidate {
                id: row.get(0)?,
                fact_summary: row.get(1)?,
                category: row.get(2)?,
                associated_date: row.get(3)?,
                severity_score: row.get(4)?,
                confidence: row.get(5)?,
            })
        })?;
        rows.collect()
    }

    /// Merge a duplicate group: soft-delete every member that is not the
    /// keeper and (optionally) annotate the keeper with the merge.
    /// Returns the number of rows soft-deleted.

    pub fn merge_duplicate_facts(&self, keeper_id: i64, member_ids: &[i64]) -> Result<usize> {
        let conn = self.intel_conn()?;
        let mut deleted = 0usize;
        for id in member_ids {
            if *id == keeper_id {
                continue;
            }
            let n = conn.execute(
                "UPDATE intelligence
                    SET is_deleted = TRUE,
                        deleted_at = CURRENT_TIMESTAMP
                  WHERE id = ?1 AND is_deleted = FALSE",
                params![id],
            )?;
            deleted += n;
        }
        Ok(deleted)
    }
}
