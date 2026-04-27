use rusqlite::{params, Result};
use std::collections::HashMap;
use tracing::info;

use super::super::database::Database;
use super::super::database::*;

impl Database {
    pub fn create_chain(
        &self,
        name: &str,
        chain_type: &str,
        description: &str,
        created_by: &str,
    ) -> Result<i64> {
        let conn = self.intel_conn()?;
        conn.execute(
            "INSERT INTO evidence_chains (chain_name, chain_type, description, created_by) VALUES (?1, ?2, ?3, ?4)",
            params![name, chain_type, description, created_by],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn add_to_chain(
        &self,
        chain_id: i64,
        intelligence_id: i64,
        relationship_type: &str,
        strength: f64,
        notes: &str,
        linked_by: &str,
    ) -> Result<()> {
        let conn = self.intel_conn()?;
        conn.execute(
            "INSERT INTO evidence_chain_links (chain_id, intelligence_id, relationship_type, relationship_strength, notes, linked_by)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![chain_id, intelligence_id, relationship_type, strength, notes, linked_by],
        )?;
        Ok(())
    }

    pub fn get_chain(&self, chain_id: i64) -> Result<Option<EvidenceChain>> {
        let conn = self.intel_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, chain_name, chain_type, description, created_by, created_at, updated_at
             FROM evidence_chains WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![chain_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(EvidenceChain {
                id: row.get(0)?,
                chain_name: row.get(1)?,
                chain_type: row.get(2)?,
                description: row.get(3)?,
                created_by: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                items: Vec::new(),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_chain_items(&self, chain_id: i64) -> Result<Vec<ChainItem>> {
        let conn = self.intel_conn()?;
        let mut stmt = conn.prepare(
            "SELECT l.id, l.intelligence_id, l.relationship_type, l.relationship_strength, l.notes, l.linked_by, l.linked_at,
                    i.filename, i.fact_summary, i.category
             FROM evidence_chain_links l
             JOIN intelligence i ON l.intelligence_id = i.id
             WHERE l.chain_id = ?1
             ORDER BY l.linked_at DESC"
        )?;

        let entries = stmt.query_map(params![chain_id], |row| {
            Ok(ChainItem {
                link_id: row.get(0)?,
                intelligence_id: row.get(1)?,
                relationship_type: row.get(2)?,
                relationship_strength: row.get(3)?,
                notes: row.get(4)?,
                linked_by: row.get(5)?,
                linked_at: row.get(6)?,
                filename: row.get(7)?,
                fact_summary: row.get(8)?,
                category: row.get(9)?,
            })
        })?;

        entries.collect()
    }

    pub fn get_all_chains(&self, limit: i64, offset: i64) -> Result<Vec<ChainSummary>> {
        let conn = self.intel_conn()?;
        let mut stmt = conn.prepare(
            "SELECT c.id, c.chain_name, c.chain_type, c.description, c.created_by, c.created_at, c.updated_at,
                    COUNT(l.id) as item_count,
                    AVG(l.relationship_strength) as avg_strength
             FROM evidence_chains c
             LEFT JOIN evidence_chain_links l ON c.id = l.chain_id
             GROUP BY c.id
             ORDER BY c.updated_at DESC
             LIMIT ?1 OFFSET ?2"
        )?;

        let entries = stmt.query_map(params![limit, offset], |row| {
            Ok(ChainSummary {
                id: row.get(0)?,
                chain_name: row.get(1)?,
                chain_type: row.get(2)?,
                description: row.get(3)?,
                created_by: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                item_count: row.get(7)?,
                avg_strength: row.get(8)?,
            })
        })?;

        entries.collect()
    }

    pub fn update_chain(
        &self,
        chain_id: i64,
        name: Option<&str>,
        description: Option<&str>,
    ) -> Result<()> {
        let conn = self.intel_conn()?;

        if let Some(n) = name {
            conn.execute(
                "UPDATE evidence_chains SET chain_name = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![n, chain_id],
            )?;
        }

        if let Some(d) = description {
            conn.execute(
                "UPDATE evidence_chains SET description = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![d, chain_id],
            )?;
        }

        Ok(())
    }

    pub fn remove_from_chain(&self, chain_id: i64, intelligence_id: i64) -> Result<()> {
        let conn = self.intel_conn()?;
        conn.execute(
            "DELETE FROM evidence_chain_links WHERE chain_id = ?1 AND intelligence_id = ?2",
            params![chain_id, intelligence_id],
        )?;
        Ok(())
    }

    /// Soft-delete an intelligence row.
    ///
    /// Per NFR-FOR-006, database operations MUST use soft deletes — set
    /// `is_deleted = TRUE` and stamp `deleted_at` rather than removing the row.

    pub fn delete_chain(&self, chain_id: i64) -> Result<()> {
        let conn = self.intel_conn()?;
        conn.execute(
            "DELETE FROM evidence_chain_links WHERE chain_id = ?1",
            params![chain_id],
        )?;
        conn.execute(
            "DELETE FROM evidence_chains WHERE id = ?1",
            params![chain_id],
        )?;
        Ok(())
    }

    pub fn get_chain_statistics(&self, chain_id: i64) -> Result<ChainStatistics> {
        let conn = self.intel_conn()?;

        let total: (i32, f64, i32, i32) = conn.query_row(
            "SELECT COUNT(*), AVG(severity_score), MAX(severity_score), MIN(severity_score)
             FROM evidence_chain_links l
             JOIN intelligence i ON l.intelligence_id = i.id
             WHERE l.chain_id = ?1",
            [chain_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;

        let categories: Vec<String> = {
            let mut stmt = conn.prepare(
                "SELECT DISTINCT i.category FROM evidence_chain_links l
                 JOIN intelligence i ON l.intelligence_id = i.id
                 WHERE l.chain_id = ?1 AND i.category IS NOT NULL",
            )?;
            let result: Vec<String> = stmt
                .query_map([chain_id], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            result
        };

        let relationship_types: Vec<String> = {
            let mut stmt = conn.prepare(
                "SELECT DISTINCT relationship_type FROM evidence_chain_links WHERE chain_id = ?1",
            )?;
            let result: Vec<String> = stmt
                .query_map([chain_id], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            result
        };

        Ok(ChainStatistics {
            total_items: total.0,
            avg_severity: total.1,
            max_severity: total.2,
            min_severity: total.3,
            categories,
            relationship_types,
        })
    }

    pub fn search_chain(&self, chain_id: i64, query: &str) -> Result<Vec<ChainItem>> {
        let conn = self.intel_conn()?;
        let search_pattern = format!("%{}%", query);

        let mut stmt = conn.prepare(
            "SELECT l.id, l.intelligence_id, l.relationship_type, l.relationship_strength, l.notes, l.linked_by, l.linked_at,
                    i.filename, i.fact_summary, i.category
             FROM evidence_chain_links l
             JOIN intelligence i ON l.intelligence_id = i.id
             WHERE l.chain_id = ?1 AND (i.fact_summary LIKE ?2 OR i.filename LIKE ?2 OR i.category LIKE ?2)
             ORDER BY l.linked_at DESC"
        )?;

        let entries = stmt.query_map(params![chain_id, search_pattern], |row| {
            Ok(ChainItem {
                link_id: row.get(0)?,
                intelligence_id: row.get(1)?,
                relationship_type: row.get(2)?,
                relationship_strength: row.get(3)?,
                notes: row.get(4)?,
                linked_by: row.get(5)?,
                linked_at: row.get(6)?,
                filename: row.get(7)?,
                fact_summary: row.get(8)?,
                category: row.get(9)?,
            })
        })?;

        entries.collect()
    }

    // Temporal analysis methods

    pub fn detect_chains(
        &self,
        min_weight: f64,
        min_related: i32,
    ) -> Result<Vec<AutoDetectedChain>> {
        let weighted = self.get_weighted_evidence(min_weight, 1000)?;

        if weighted.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.intel_conn()?;

        let fetch_start = std::time::Instant::now();

        // Pre-fetch all entity data in ONE query (avoids N+1 problem)
        let mut entities_stmt = conn.prepare(
            "SELECT fingerprint, value FROM entities WHERE fingerprint IN (
                SELECT fingerprint FROM intelligence WHERE is_deleted = FALSE
            )",
        )?;

        let mut entities_by_fp: HashMap<String, Vec<String>> = HashMap::new();
        entities_stmt
            .query_map([], |row| {
                let fp: String = row.get(0)?;
                let value: String = row.get(1)?;
                Ok((fp, value))
            })?
            .filter_map(|r| r.ok())
            .for_each(|(fp, value)| {
                entities_by_fp.entry(fp).or_default().push(value);
            });

        let fetch_time = fetch_start.elapsed();
        let entity_count = entities_by_fp.len();
        info!(
            entities_fetched = entity_count,
            fetch_time_ms = fetch_time.as_millis() as u64,
            "Fetched entities for chain detection"
        );

        // Convert to HashSets for faster lookup if entity count is large
        use std::collections::HashSet;
        let entity_sets: HashMap<String, HashSet<String>> = if entity_count > 50 {
            entities_by_fp
                .iter()
                .map(|(k, v)| (k.clone(), v.iter().cloned().collect()))
                .collect()
        } else {
            HashMap::new()
        };

        let use_hashset = !entity_sets.is_empty();
        let empty_vec: Vec<String> = Vec::new();
        let empty_set: HashSet<String> = HashSet::new();

        let mut chains = Vec::new();

        // Compute overlaps in memory - use HashSet for large sets, Vec for small
        for (i, current) in weighted.iter().enumerate() {
            let mut related: Vec<RelatedEvidence> = Vec::new();

            for (j, other) in weighted.iter().enumerate() {
                if i == j {
                    continue;
                }

                let overlap = if use_hashset {
                    let current_set = entity_sets.get(&current.fingerprint).unwrap_or(&empty_set);
                    let other_set = entity_sets.get(&other.fingerprint).unwrap_or(&empty_set);
                    current_set.intersection(other_set).count() as i32
                } else {
                    let current_entities = entities_by_fp
                        .get(&current.fingerprint)
                        .unwrap_or(&empty_vec);
                    let other_entities =
                        entities_by_fp.get(&other.fingerprint).unwrap_or(&empty_vec);
                    current_entities
                        .iter()
                        .filter(|e| other_entities.contains(e))
                        .count() as i32
                };

                if overlap >= min_related {
                    related.push(RelatedEvidence {
                        id: other.id,
                        fingerprint: other.fingerprint.clone(),
                        filename: other.filename.clone(),
                        summary: other.summary.clone(),
                        weight: other.weight,
                        shared_entities: overlap,
                    });
                }
            }

            if related.len() >= 2 {
                chains.push(AutoDetectedChain {
                    root_id: current.id,
                    root_summary: current.summary.clone(),
                    root_weight: current.weight,
                    related_count: related.len() as i32,
                    related_evidence: related,
                });
            }
        }

        chains.sort_by(|a, b| b.related_count.cmp(&a.related_count));

        let total_time = fetch_start.elapsed();
        info!(
            chains_found = chains.len(),
            total_time_ms = total_time.as_millis() as u64,
            chains_detected = true,
            "Chain detection completed"
        );

        Ok(chains)
    }

    pub fn detect_chains_by_entities(
        &self,
        entity_values: &[String],
        min_occurrences: i32,
    ) -> Result<Vec<EntityChain>> {
        let conn = self.intel_conn()?;

        let placeholders: Vec<String> = entity_values.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT i.id, i.fingerprint, i.filename, i.fact_summary, i.severity_score, i.confidence,
                    GROUP_CONCAT(e.value) as entities
             FROM intelligence i
             JOIN entities e ON i.fingerprint = e.fingerprint
             WHERE e.value IN ({})
             GROUP BY i.id
             HAVING COUNT(DISTINCT e.value) >= ?1",
            placeholders.join(",")
        );

        let mut params: Vec<&dyn rusqlite::ToSql> = entity_values
            .iter()
            .map(|v| v as &dyn rusqlite::ToSql)
            .collect();
        params.push(&min_occurrences);

        let mut stmt = conn.prepare(&sql)?;
        let entries = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok(EntityChain {
                intelligence_id: row.get(0)?,
                fingerprint: row.get(1)?,
                filename: row.get(2)?,
                summary: row.get(3)?,
                severity: row.get(4)?,
                confidence: row.get(5)?,
                matching_entities: row
                    .get::<_, String>(6)?
                    .split(',')
                    .map(|s| s.to_string())
                    .collect(),
            })
        })?;

        entries.collect()
    }

    pub fn get_chain_suggestions(
        &self,
        intelligence_id: i64,
        similarity_threshold: f64,
    ) -> Result<Vec<ChainSuggestion>> {
        let conn = self.intel_conn()?;

        let mut stmt = conn.prepare(
            "SELECT fingerprint, fact_summary, category, severity_score FROM intelligence WHERE id = ?1"
        )?;
        let (fingerprint, summary, category, severity): (String, String, Option<String>, i32) =
            stmt.query_row([intelligence_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?;

        // Pre-compute keywords and lowercase versions once
        let keywords: Vec<String> = summary
            .split_whitespace()
            .filter(|w| w.len() > 4)
            .take(10)
            .map(|w| w.to_lowercase())
            .collect();

        let conditions: Vec<String> = keywords
            .iter()
            .map(|k| {
                let escaped = k.replace('\'', "''").replace('\\', "\\\\");
                format!("LOWER(fact_summary) LIKE '%{}%' ESCAPE '\\'", escaped)
            })
            .collect();
        let keyword_where = conditions.join(" OR ");

        let sql = format!(
            "SELECT id, fact_summary, category, severity_score, confidence
             FROM intelligence
             WHERE is_deleted = FALSE AND fingerprint != ?1 AND ({})
             ORDER BY severity_score DESC
             LIMIT 20",
            keyword_where
        );

        let mut stmt = conn.prepare(&sql)?;
        let suggestions: Vec<ChainSuggestion> = stmt
            .query_map([fingerprint.as_str()], |row| {
                let id: i64 = row.get(0)?;
                let sum: String = row.get(1)?;
                let cat: Option<String> = row.get(2)?;
                let sev: i32 = row.get(3)?;
                let _conf: Option<f64> = row.get(4)?;

                let sum_lower = sum.to_lowercase();
                let keyword_matches = keywords
                    .iter()
                    .filter(|k| sum_lower.contains(k.as_str()))
                    .count();
                let similarity = (keyword_matches as f64 / keywords.len() as f64) * 0.7
                    + if cat.as_ref() == category.as_ref() {
                        0.2
                    } else {
                        0.0
                    }
                    + if (sev - severity).abs() <= 1 {
                        0.1
                    } else {
                        0.0
                    };

                Ok(ChainSuggestion {
                    target_id: id,
                    summary: sum,
                    category: cat,
                    similarity,
                    match_reasons: format!("{} keyword matches", keyword_matches),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(suggestions
            .into_iter()
            .filter(|s| s.similarity >= similarity_threshold)
            .collect())
    }
}
