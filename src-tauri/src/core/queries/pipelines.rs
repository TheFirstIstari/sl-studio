use rusqlite::{params, Result};

use super::super::database::Database;

impl Database {
    pub fn save_pipeline(
        &self,
        id: &str,
        name: &str,
        description: &str,
        passes_json: &str,
        is_builtin: bool,
    ) -> Result<()> {
        let conn = self.intel_conn()?;
        conn.execute(
            "INSERT INTO pipelines (id, name, description, passes_json, is_builtin)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                passes_json = excluded.passes_json,
                is_builtin = excluded.is_builtin,
                updated_at = CURRENT_TIMESTAMP",
            params![id, name, description, passes_json, is_builtin as i64],
        )?;
        Ok(())
    }

    /// Returns (id, name, description, passes_json, is_builtin) for every
    /// stored pipeline, newest-updated first.
    #[allow(clippy::type_complexity)]
    pub fn list_pipelines(&self) -> Result<Vec<(String, String, String, String, bool)>> {
        let conn = self.intel_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, description, passes_json, is_builtin
               FROM pipelines
              ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let is_builtin: i64 = row.get(4)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                is_builtin != 0,
            ))
        })?;
        rows.collect()
    }

    #[allow(clippy::type_complexity)]
    pub fn get_pipeline(&self, id: &str) -> Result<Option<(String, String, String, String, bool)>> {
        let conn = self.intel_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, description, passes_json, is_builtin
               FROM pipelines WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let is_builtin: i64 = row.get(4)?;
            Ok(Some((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                is_builtin != 0,
            )))
        } else {
            Ok(None)
        }
    }

    pub fn delete_pipeline(&self, id: &str) -> Result<()> {
        let conn = self.intel_conn()?;
        conn.execute("DELETE FROM pipelines WHERE id = ?1", params![id])?;
        Ok(())
    }
}
