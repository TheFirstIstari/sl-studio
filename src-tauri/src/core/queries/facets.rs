use rusqlite::{params, Result};

use super::super::database::Database;

impl Database {
    pub fn save_facet_preset(&self, page: &str, name: &str, state_json: &str) -> Result<i64> {
        let conn = self.intel_conn()?;
        conn.execute(
            "INSERT INTO facet_presets (page, name, state_json) VALUES (?1, ?2, ?3)
             ON CONFLICT(page, name) DO UPDATE SET
                state_json = excluded.state_json,
                updated_at = CURRENT_TIMESTAMP",
            params![page, name, state_json],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Returns (id, page, name, state_json, updated_at) tuples sorted by
    /// most-recently-updated first.
    #[allow(clippy::type_complexity)]
    pub fn list_facet_presets(
        &self,
        page: &str,
    ) -> Result<Vec<(i64, String, String, String, Option<String>)>> {
        let conn = self.intel_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, page, name, state_json, updated_at
               FROM facet_presets
              WHERE page = ?1
              ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map(params![page], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })?;
        rows.collect()
    }

    pub fn delete_facet_preset(&self, id: i64) -> Result<()> {
        let conn = self.intel_conn()?;
        let n = conn.execute("DELETE FROM facet_presets WHERE id = ?1", params![id])?;
        if n == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }
}
