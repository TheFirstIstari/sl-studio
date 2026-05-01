use rusqlite::{params, Result};

use super::super::database::Database;
use super::super::database::*;

impl Database {
    pub fn checkpoint_start(&self, job_type: &str, job_id: &str) -> Result<i64> {
        let conn = self.intel_conn()?;
        conn.execute(
            "INSERT INTO checkpoints (job_type, job_id, status) VALUES (?1, ?2, 'running')",
            params![job_type, job_id],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn checkpoint_update(
        &self,
        job_id: &str,
        last_fingerprint: &str,
        total_processed: i64,
    ) -> Result<()> {
        let conn = self.intel_conn()?;
        conn.execute(
            "UPDATE checkpoints SET last_fingerprint = ?1, total_processed = ?2, updated_at = CURRENT_TIMESTAMP WHERE job_id = ?3",
            params![last_fingerprint, total_processed, job_id],
        )?;
        Ok(())
    }

    pub fn checkpoint_complete(&self, job_id: &str) -> Result<()> {
        let conn = self.intel_conn()?;
        conn.execute(
            "UPDATE checkpoints SET status = 'completed', completed_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE job_id = ?1",
            params![job_id],
        )?;
        Ok(())
    }

    pub fn get_active_checkpoint(&self, job_type: &str) -> Result<Option<JobCheckpoint>> {
        let conn = self.intel_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, job_type, job_id, last_fingerprint, total_processed, status
             FROM checkpoints
             WHERE job_type = ?1 AND status = 'running'
             ORDER BY created_at DESC
             LIMIT 1",
        )?;

        let mut rows = stmt.query(params![job_type])?;
        if let Some(row) = rows.next()? {
            Ok(Some(JobCheckpoint {
                id: row.get(0)?,
                job_type: row.get(1)?,
                job_id: row.get(2)?,
                last_fingerprint: row.get(3)?,
                total_processed: row.get(4)?,
                status: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn log_audit(&self, action: &str, details: &str, duration_ms: Option<i64>) -> Result<()> {
        let conn = self.intel_conn()?;
        conn.execute(
            "INSERT INTO audit_log (action, details, duration_ms) VALUES (?1, ?2, ?3)",
            params![action, details, duration_ms],
        )?;
        Ok(())
    }

    /// Push a processing failure onto the error queue so operators can
    /// review and retry failed files without re-running the full batch.
    pub fn push_error(
        &self,
        fingerprint: &str,
        job_type: &str,
        error_message: &str,
        error_details: Option<&str>,
    ) -> Result<()> {
        let conn = self.intel_conn()?;
        conn.execute(
            "INSERT INTO error_queue (fingerprint, job_type, error_message, error_details, last_attempt)
             VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP)
             ON CONFLICT DO NOTHING",
            params![fingerprint, job_type, error_message, error_details],
        )?;
        Ok(())
    }
}
