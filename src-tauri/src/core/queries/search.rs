use rusqlite::Result;

use super::super::database::Database;
use super::super::database::*;

impl Database {
    pub fn search_facts(&self, query: &str, limit: i64) -> Result<Vec<SearchResult>> {
        self.search_facts_with_filters(query, limit, None, None, None, None)
    }

    pub fn search_facts_with_filters(
        &self,
        query: &str,
        limit: i64,
        categories: Option<&[String]>,
        min_severity: Option<i32>,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Result<Vec<SearchResult>> {
        let conn = self.intel_conn()?;

        let fts_query = Self::parse_search_query(query);

        let mut sql = String::from(
            "SELECT i.id, i.fingerprint, i.filename, i.fact_summary, i.category, i.severity_score, i.confidence, i.created_at,
                    bm25(facts_fts) as rank
             FROM facts_fts f
             JOIN intelligence i ON f.rowid = i.id
             WHERE facts_fts MATCH ?1"
        );

        let mut conditions = Vec::new();
        let mut param_idx = 2;

        if let Some(cats) = categories {
            if !cats.is_empty() {
                let placeholders: Vec<String> =
                    cats.iter().map(|_| format!("?{}", param_idx)).collect();
                conditions.push(format!("category IN ({})", placeholders.join(",")));
                param_idx += cats.len() as i32;
            }
        }

        if min_severity.is_some() {
            conditions.push(format!("severity_score >= ?{}", param_idx));
            param_idx += 1;
        }

        if start_date.is_some() {
            conditions.push(format!("associated_date >= ?{}", param_idx));
            param_idx += 1;
        }

        if end_date.is_some() {
            conditions.push(format!("associated_date <= ?{}", param_idx));
        }

        if !conditions.is_empty() {
            sql.push_str(" AND ");
            sql.push_str(&conditions.join(" AND "));
        }

        sql.push_str(" ORDER BY rank LIMIT ?");

        let mut stmt = conn.prepare(&sql)?;

        let mut params: Vec<Box<dyn rusqlite::ToSql>> =
            vec![Box::new(fts_query.clone()), Box::new(limit)];

        if let Some(cats) = categories {
            for cat in cats {
                params.push(Box::new(cat.clone()));
            }
        }

        if let Some(severity) = min_severity {
            params.push(Box::new(severity));
        }

        if let Some(start) = start_date {
            params.push(Box::new(start.to_string()));
        }

        if let Some(end) = end_date {
            params.push(Box::new(end.to_string()));
        }

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let entries = stmt.query_map(rusqlite::params_from_iter(param_refs.iter()), |row| {
            Ok(SearchResult {
                id: row.get(0)?,
                fingerprint: row.get(1)?,
                filename: row.get(2)?,
                summary: row.get(3)?,
                category: row.get(4)?,
                severity: row.get(5)?,
                confidence: row.get(6)?,
                rank: row.get(7)?,
                result_type: "fact".to_string(),
            })
        })?;

        entries.collect()
    }

    pub fn search_entities(&self, query: &str, limit: i64) -> Result<Vec<EntitySearchResult>> {
        self.search_entities_with_filters(query, limit, None, None)
    }

    pub fn search_entities_with_filters(
        &self,
        query: &str,
        limit: i64,
        entity_types: Option<&[String]>,
        min_confidence: Option<f64>,
    ) -> Result<Vec<EntitySearchResult>> {
        let conn = self.intel_conn()?;

        let fts_query = Self::parse_search_query(query);

        let mut sql = String::from(
            "SELECT e.id, e.fingerprint, e.entity_type, e.value, e.normalized_value, e.confidence,
                    i.filename, bm25(entities_fts) as rank
             FROM entities_fts f
             JOIN entities e ON f.rowid = e.id
             JOIN intelligence i ON e.fingerprint = i.fingerprint
             WHERE entities_fts MATCH ?1",
        );

        let mut conditions = Vec::new();
        let mut param_idx = 2;

        if let Some(types) = entity_types {
            if !types.is_empty() {
                let placeholders: Vec<String> =
                    types.iter().map(|_| format!("?{}", param_idx)).collect();
                conditions.push(format!("e.entity_type IN ({})", placeholders.join(",")));
                param_idx += types.len() as i32;
            }
        }

        if min_confidence.is_some() {
            conditions.push(format!("e.confidence >= ?{}", param_idx));
        }

        if !conditions.is_empty() {
            sql.push_str(" AND ");
            sql.push_str(&conditions.join(" AND "));
        }

        sql.push_str(" ORDER BY rank LIMIT ?");

        let mut stmt = conn.prepare(&sql)?;

        let mut params: Vec<Box<dyn rusqlite::ToSql>> =
            vec![Box::new(fts_query.clone()), Box::new(limit)];

        if let Some(types) = entity_types {
            for t in types {
                params.push(Box::new(t.clone()));
            }
        }

        if let Some(confidence) = min_confidence {
            params.push(Box::new(confidence));
        }

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let entries = stmt.query_map(rusqlite::params_from_iter(param_refs.iter()), |row| {
            Ok(EntitySearchResult {
                id: row.get(0)?,
                fingerprint: row.get(1)?,
                entity_type: row.get(2)?,
                value: row.get(3)?,
                normalized_value: row.get(4)?,
                confidence: row.get(5)?,
                source_file: row.get(6)?,
                rank: row.get(7)?,
            })
        })?;

        entries.collect()
    }

    fn parse_search_query(input: &str) -> String {
        let mut result = String::with_capacity(input.len() + 64);
        let mut in_phrase = false;
        let mut pos = 0;
        let chars: Vec<char> = input.chars().collect();

        while pos < chars.len() {
            let c = chars[pos];

            if c == '"' {
                result.push('"');
                in_phrase = !in_phrase;
                pos += 1;
            } else if c == ' ' && !in_phrase {
                if !result.ends_with(' ') {
                    result.push(' ');
                }
                pos += 1;
            } else if c == '(' || c == ')' {
                result.push(c);
                pos += 1;
            } else if c.eq_ignore_ascii_case(&'A') && result.ends_with(' ') && pos + 2 < chars.len()
            {
                let next = chars[pos + 1];
                let next2 = chars[pos + 2];
                if next.eq_ignore_ascii_case(&'N') && next2.eq_ignore_ascii_case(&'D') {
                    result.push_str("AND ");
                    pos += 3;
                    continue;
                }
                result.push(c);
                pos += 1;
            } else if c.eq_ignore_ascii_case(&'O') && result.ends_with(' ') && pos + 1 < chars.len()
            {
                let next = chars[pos + 1];
                if next.eq_ignore_ascii_case(&'R') {
                    result.push_str("OR ");
                    pos += 2;
                    continue;
                }
                result.push(c);
                pos += 1;
            } else if c.eq_ignore_ascii_case(&'N') && result.ends_with(' ') && pos + 2 < chars.len()
            {
                let next = chars[pos + 1];
                let next2 = chars[pos + 2];
                if next.eq_ignore_ascii_case(&'O') && next2.eq_ignore_ascii_case(&'T') {
                    result.push_str("NOT ");
                    pos += 3;
                    continue;
                }
                result.push(c);
                pos += 1;
            } else {
                result.push(c);
                pos += 1;
            }
        }

        result.trim().to_string()
    }

    pub fn search_combined(&self, query: &str, limit: i64) -> Result<Vec<CombinedSearchResult>> {
        let facts = self.search_facts(query, limit)?;
        let entities = self.search_entities(query, limit)?;

        let mut combined: Vec<CombinedSearchResult> = facts
            .into_iter()
            .map(|f| CombinedSearchResult {
                id: f.id,
                result_type: f.result_type,
                fingerprint: f.fingerprint,
                filename: f.filename,
                title: f.summary.clone(),
                description: Some(f.summary),
                category: f.category,
                severity: Some(f.severity),
                confidence: f.confidence,
                rank: f.rank,
            })
            .collect();

        for e in entities {
            combined.push(CombinedSearchResult {
                id: e.id,
                result_type: "entity".to_string(),
                fingerprint: e.fingerprint,
                filename: e.source_file,
                title: e.value.clone(),
                description: e.normalized_value,
                category: Some(e.entity_type),
                severity: None,
                confidence: e.confidence,
                rank: e.rank,
            });
        }

        combined.sort_by(|a, b| {
            a.rank
                .partial_cmp(&b.rank)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        combined.truncate(limit as usize);

        Ok(combined)
    }

    // ----------------------------------------------------------------------
    // FR-PLP: persisted user-defined pipelines (table created by migration v2)
    // ----------------------------------------------------------------------

    /// Persist a pipeline. Acts as upsert (id is the primary key).
    /// `passes_json` is the JSON-serialized Vec<PipelinePass>.

    pub fn search_by_tags(
        &self,
        tags: &[String],
        match_all: bool,
        limit: i64,
    ) -> Result<Vec<SearchResult>> {
        let conn = self.intel_conn()?;

        if tags.is_empty() {
            return Ok(Vec::new());
        }

        let conditions: Vec<String> = tags
            .iter()
            .map(|t| {
                let escaped = t.replace('\'', "''").replace('\\', "\\\\");
                format!("tags LIKE '%{}%' ESCAPE '\\'", escaped)
            })
            .collect();

        let where_clause = if match_all {
            conditions.join(" AND ")
        } else {
            conditions.join(" OR ")
        };

        let sql = format!(
            "SELECT id, fingerprint, filename, fact_summary, category, severity_score, confidence, created_at,
                    0.0 as rank
             FROM intelligence
             WHERE is_deleted = FALSE AND ({}) LIMIT ?",
            where_clause
        );

        let mut stmt = conn.prepare(&sql)?;

        let entries = stmt.query_map([limit], |row| {
            Ok(SearchResult {
                id: row.get(0)?,
                fingerprint: row.get(1)?,
                filename: row.get(2)?,
                summary: row.get(3)?,
                category: row.get(4)?,
                severity: row.get(5)?,
                confidence: row.get(6)?,
                rank: row.get(7)?,
                result_type: "fact".to_string(),
            })
        })?;

        entries.collect()
    }
}
