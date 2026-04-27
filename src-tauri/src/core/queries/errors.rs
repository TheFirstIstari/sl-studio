use rusqlite::{params, Result};

use super::super::database::Database;
use super::super::database::*;

impl Database {
    pub fn add_error(
        &self,
        fingerprint: &str,
        job_type: &str,
        error_message: &str,
        error_details: &str,
    ) -> Result<()> {
        let conn = self.intel_conn()?;
        conn.execute(
            "INSERT INTO error_queue (fingerprint, job_type, error_message, error_details, next_attempt)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params![fingerprint, job_type, error_message, error_details],
        )?;
        Ok(())
    }

    pub fn get_pending_errors(&self, limit: i64) -> Result<Vec<ErrorQueueEntry>> {
        let conn = self.intel_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, fingerprint, job_type, error_message, error_details, retry_count, max_retries, last_attempt, next_attempt, resolved, resolution, created_at
             FROM error_queue
             WHERE resolved = 0 AND datetime(next_attempt) <= datetime('now')
             ORDER BY next_attempt ASC
             LIMIT ?1",
        )?;

        let entries = stmt.query_map([limit], |row| {
            Ok(ErrorQueueEntry {
                id: row.get(0)?,
                fingerprint: row.get(1)?,
                job_type: row.get(2)?,
                error_message: row.get(3)?,
                error_details: row.get(4)?,
                retry_count: row.get(5)?,
                max_retries: row.get(6)?,
                last_attempt: row.get(7)?,
                next_attempt: row.get(8)?,
                resolved: row.get(9)?,
                resolution: row.get(10)?,
                created_at: row.get(11)?,
            })
        })?;

        entries.collect()
    }

    pub fn update_error(
        &self,
        error_id: i64,
        retry_count: i32,
        error_message: &str,
        next_attempt: Option<String>,
    ) -> Result<()> {
        let conn = self.intel_conn()?;
        let mut stmt = conn.prepare(
            "UPDATE error_queue SET
                 retry_count = ?2,
                 error_message = ?3,
                 last_attempt = CURRENT_TIMESTAMP,
                 next_attempt = COALESCE(?4, datetime('now', '+' || (retry_count * 2) || ' minutes')),
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
        )?;
        stmt.execute(params![error_id, retry_count, error_message, next_attempt])?;
        Ok(())
    }

    pub fn resolve_error(&self, error_id: i64, resolution: &str, resolved_by: &str) -> Result<()> {
        let conn = self.intel_conn()?;
        conn.execute(
            "UPDATE error_queue SET
                 resolved = 1,
                 resolution = ?2,
                 resolved_by = ?3,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
            params![error_id, resolution, resolved_by],
        )?;
        Ok(())
    }
}
