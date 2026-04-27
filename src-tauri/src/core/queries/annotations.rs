use rusqlite::{params, Result};

use super::super::database::Database;
use super::super::database::*;

impl Database {
    pub fn add_tag(&self, intelligence_id: i64, tag: &str) -> Result<()> {
        let conn = self.intel_conn()?;

        let current_tags: Option<String> = conn.query_row(
            "SELECT tags FROM intelligence WHERE id = ?1",
            [intelligence_id],
            |row| row.get(0),
        )?;

        let mut tags: Vec<String> = current_tags
            .map(|t| {
                t.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        if !tags.contains(&tag.to_string()) {
            tags.push(tag.to_string());
        }

        let tags_str = tags.join(",");

        conn.execute(
            "UPDATE intelligence SET tags = ?1 WHERE id = ?2",
            params![tags_str, intelligence_id],
        )?;

        Ok(())
    }

    pub fn remove_tag(&self, intelligence_id: i64, tag: &str) -> Result<()> {
        let conn = self.intel_conn()?;

        let current_tags: Option<String> = conn.query_row(
            "SELECT tags FROM intelligence WHERE id = ?1",
            [intelligence_id],
            |row| row.get(0),
        )?;

        let mut tags: Vec<String> = current_tags
            .map(|t| {
                t.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        tags.retain(|t| t != tag);

        let tags_str = tags.join(",");

        conn.execute(
            "UPDATE intelligence SET tags = ?1 WHERE id = ?2",
            params![tags_str, intelligence_id],
        )?;

        Ok(())
    }

    pub fn get_all_tags(&self) -> Result<Vec<String>> {
        let conn = self.intel_conn()?;

        let mut stmt = conn.prepare(
            "SELECT DISTINCT tags FROM intelligence WHERE tags IS NOT NULL AND tags != ''",
        )?;

        let all_tags: Vec<String> = stmt
            .query_map([], |row| {
                let tags_str: Option<String> = row.get(0)?;
                Ok(tags_str)
            })?
            .filter_map(|r| r.ok())
            .flatten()
            .flat_map(|t| {
                t.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            })
            .collect();

        let mut unique_tags: Vec<String> = all_tags.into_iter().collect();
        unique_tags.sort();
        unique_tags.dedup();

        Ok(unique_tags)
    }

    pub fn add_annotation(
        &self,
        intelligence_id: i64,
        content: &str,
        annotation_type: &str,
    ) -> Result<i64> {
        let conn = self.intel_conn()?;

        conn.execute(
            "INSERT INTO annotations (intelligence_id, content, annotation_type) VALUES (?1, ?2, ?3)",
            params![intelligence_id, content, annotation_type],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn update_annotation(&self, annotation_id: i64, content: &str) -> Result<()> {
        let conn = self.intel_conn()?;

        conn.execute(
            "UPDATE annotations SET content = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![content, annotation_id],
        )?;

        Ok(())
    }

    pub fn delete_annotation(&self, annotation_id: i64) -> Result<()> {
        let conn = self.intel_conn()?;

        conn.execute("DELETE FROM annotations WHERE id = ?1", [annotation_id])?;

        Ok(())
    }

    pub fn get_annotations(&self, intelligence_id: i64) -> Result<Vec<Annotation>> {
        let conn = self.intel_conn()?;

        let mut stmt = conn.prepare(
            "SELECT id, content, annotation_type, created_at, updated_at
             FROM annotations WHERE intelligence_id = ?1 ORDER BY created_at DESC",
        )?;

        let entries = stmt.query_map([intelligence_id], |row| {
            Ok(Annotation {
                id: row.get(0)?,
                intelligence_id,
                content: row.get(1)?,
                annotation_type: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;

        entries.collect()
    }
}
