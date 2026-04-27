use rusqlite::{params, Result};

use super::super::database::Database;
use super::super::database::*;

impl Database {
    pub fn get_corroboration_candidates(
        &self,
        intelligence_id: i64,
    ) -> Result<(String, String, Vec<(i64, String, String, Option<String>)>)> {
        let conn = self.intel_conn()?;
        let (filename, fact_summary, category): (String, String, Option<String>) = conn.query_row(
            "SELECT filename, fact_summary, category
                   FROM intelligence
                  WHERE id = ?1",
            params![intelligence_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;

        let mut stmt = conn.prepare(
            "SELECT id, filename, fact_summary, category
               FROM intelligence
              WHERE is_deleted = FALSE
                AND id != ?1
                AND filename != ?2
                AND (
                    (?3 IS NULL AND category IS NULL)
                    OR category = ?3
                )",
        )?;
        let rows = stmt.query_map(params![intelligence_id, filename, category], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        let candidates: Vec<(i64, String, String, Option<String>)> =
            rows.collect::<Result<Vec<_>>>()?;
        Ok((fact_summary, filename, candidates))
    }

    /// Build a deduped list of (id, type, value) entity tuples for the
    /// resolution scanner. Distinct by (entity_type, lower(value)) so we
    /// don't churn on per-document duplicates that already exist by design.

    pub fn list_distinct_entities(&self, limit: i64) -> Result<Vec<(i64, String, String)>> {
        let conn = self.intel_conn()?;
        let mut stmt = conn.prepare(
            "SELECT MIN(id) AS id, entity_type, value
               FROM entities
              WHERE is_deleted = FALSE
              GROUP BY entity_type, LOWER(value)
              ORDER BY entity_type, value
              LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok((row.get::<_, i64>(0)?, row.get(1)?, row.get(2)?))
        })?;
        rows.collect()
    }

    // Entity alias methods for entity resolution

    pub fn add_entity_alias(
        &self,
        canonical_id: i64,
        alias: &str,
        alias_type: &str,
        confidence: f64,
    ) -> Result<()> {
        let conn = self.intel_conn()?;
        conn.execute(
            "INSERT INTO entity_aliases (canonical_entity_id, alias_value, alias_type, confidence) VALUES (?1, ?2, ?3, ?4)",
            params![canonical_id, alias, alias_type, confidence],
        )?;
        Ok(())
    }

    pub fn resolve_entity(&self, alias: &str) -> Result<Vec<ResolvedEntity>> {
        let conn = self.intel_conn()?;
        let mut stmt = conn.prepare(
            "SELECT e.id, e.entity_type, e.value, e.normalized_value, e.fingerprint, a.confidence
             FROM entity_aliases a
             JOIN entities e ON a.canonical_entity_id = e.id
             WHERE a.alias_value = ?1
             ORDER BY a.confidence DESC",
        )?;

        let entries = stmt.query_map(params![alias], |row| {
            Ok(ResolvedEntity {
                entity_id: row.get(0)?,
                entity_type: row.get(1)?,
                value: row.get(2)?,
                normalized_value: row.get(3)?,
                fingerprint: row.get(4)?,
                confidence: row.get(5)?,
            })
        })?;

        entries.collect()
    }

    // Evidence chain methods

    pub fn get_entity_relationships(
        &self,
        entity_id: Option<i64>,
        min_confidence: f64,
    ) -> Result<Vec<EntityRelationship>> {
        let conn = self.intel_conn()?;

        let sql = if let Some(eid) = entity_id {
            format!(
                "SELECT e1.id, e1.entity_type, e1.value, e2.id, e2.entity_type, e2.value,
                        COUNT(*) as cooccurrence, AVG(i.confidence) as avg_confidence
                 FROM entities e1
                 JOIN entities e2 ON e1.fingerprint = e2.fingerprint AND e1.id < e2.id
                 JOIN intelligence i ON e1.fingerprint = i.fingerprint
                 WHERE e1.id = {} AND i.confidence >= {}
                 GROUP BY e1.id, e2.id
                 ORDER BY cooccurrence DESC
                 LIMIT 100",
                eid, min_confidence
            )
        } else {
            format!(
                "SELECT e1.id, e1.entity_type, e1.value, e2.id, e2.entity_type, e2.value,
                        COUNT(*) as cooccurrence, AVG(i.confidence) as avg_confidence
                 FROM entities e1
                 JOIN entities e2 ON e1.fingerprint = e2.fingerprint AND e1.id < e2.id
                 JOIN intelligence i ON e1.fingerprint = i.fingerprint
                 WHERE i.confidence >= {}
                 GROUP BY e1.id, e2.id
                 ORDER BY cooccurrence DESC
                 LIMIT 100",
                min_confidence
            )
        };

        let mut stmt = conn.prepare(&sql)?;
        let entries = stmt.query_map([], |row| {
            Ok(EntityRelationship {
                entity1_id: row.get(0)?,
                entity1_type: row.get(1)?,
                entity1_value: row.get(2)?,
                entity2_id: row.get(3)?,
                entity2_type: row.get(4)?,
                entity2_value: row.get(5)?,
                cooccurrence: row.get(6)?,
                avg_confidence: row.get(7)?,
            })
        })?;

        entries.collect()
    }

    /// FR-NET-005: build an undirected entity co-occurrence graph from the
    /// `entities` / `intelligence` tables. Returns the distinct node ids and
    /// the edge list (weight = co-occurrence count) for edges where the
    /// co-occurrence is at least `min_cooccurrence`.

    pub fn get_entity_centrality(
        &self,
        entity_type: Option<&str>,
        min_confidence: f64,
    ) -> Result<Vec<EntityCentrality>> {
        let conn = self.intel_conn()?;

        let type_filter = if let Some(et) = entity_type {
            format!("AND e.entity_type = '{}'", et)
        } else {
            String::new()
        };

        let sql = format!(
            "SELECT e.id, e.entity_type, e.value, 
                    COUNT(DISTINCT e.fingerprint) as document_count,
                    COUNT(e.id) as occurrence_count,
                    AVG(e.confidence) as avg_confidence
             FROM entities e
             JOIN intelligence i ON e.fingerprint = i.fingerprint
             WHERE i.confidence >= {} {}
             GROUP BY e.id
             ORDER BY occurrence_count DESC
             LIMIT 50",
            min_confidence, type_filter
        );

        let mut stmt = conn.prepare(&sql)?;
        let entries = stmt.query_map([], |row| {
            Ok(EntityCentrality {
                entity_id: row.get(0)?,
                entity_type: row.get(1)?,
                value: row.get(2)?,
                document_count: row.get(3)?,
                occurrence_count: row.get(4)?,
                avg_confidence: row.get(5)?,
                centrality_score: 0.0,
            })
        })?;

        let mut results: Vec<EntityCentrality> = entries.filter_map(|r| r.ok()).collect();

        if let Some(max_occ) = results.iter().map(|e| e.occurrence_count).max() {
            if max_occ > 0 {
                for r in &mut results {
                    r.centrality_score = r.occurrence_count as f64 / max_occ as f64;
                }
            }
        }

        results.sort_by(|a, b| {
            b.centrality_score
                .partial_cmp(&a.centrality_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results)
    }

    /// Get entities connected to a given entity through shared evidence.
    /// Currently returns direct connections (distance=1) only.
    /// The `depth` parameter is reserved for future recursive traversal implementation.

    pub fn get_connected_entities(
        &self,
        entity_id: i64,
        #[allow(unused)] depth: i32,
        min_confidence: f64,
    ) -> Result<Vec<ConnectedEntity>> {
        let conn = self.intel_conn()?;

        let mut stmt = conn.prepare(
            "SELECT DISTINCT e2.id, e2.entity_type, e2.value, e2.confidence, i.filename
             FROM entities e1
             JOIN entities e2 ON e1.fingerprint = e2.fingerprint AND e1.id != e2.id
             JOIN intelligence i ON e1.fingerprint = i.fingerprint
             WHERE e1.id = ?1 AND i.confidence >= ?2",
        )?;

        let entries = stmt.query_map(params![entity_id, min_confidence], |row| {
            Ok(ConnectedEntity {
                entity_id: row.get(0)?,
                entity_type: row.get(1)?,
                value: row.get(2)?,
                confidence: row.get(3)?,
                source_file: row.get(4)?,
                distance: 1,
            })
        })?;

        entries.collect()
    }
}
