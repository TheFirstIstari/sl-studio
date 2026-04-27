//! Entity resolution: surface likely-duplicate entities so the user can mark
//! them as aliases of a single canonical record.
//!
//! Strategy: within each entity_type bucket, compute a normalized form
//! (lowercase, strip punctuation, collapse whitespace) plus a token-Jaccard
//! similarity. Pairs above the threshold are reported as candidate matches.
//!
//! The actual aliasing decision is left to the user (and the
//! `add_entity_alias` Tauri command) — this module only proposes pairs.

use serde::{Deserialize, Serialize};

/// A single entity record provided to the resolver.
#[derive(Debug, Clone)]
pub struct EntityCandidate {
    pub id: i64,
    pub entity_type: String,
    pub value: String,
}

/// A suggested pair of entities that may represent the same real-world thing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMatchSuggestion {
    pub canonical_id: i64,
    pub canonical_value: String,
    pub alias_id: i64,
    pub alias_value: String,
    pub entity_type: String,
    pub similarity: f32,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityResolutionConfig {
    pub similarity_threshold: f32,
    /// Limit on entities scanned per type. Keeps O(n^2) bounded.
    pub per_type_limit: usize,
}

impl Default for EntityResolutionConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.80,
            per_type_limit: 1000,
        }
    }
}

/// Strip non-alphanumeric chars, lowercase, collapse whitespace.
pub fn normalize(value: &str) -> String {
    let lowered = value.to_lowercase();
    let cleaned: String = lowered
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Jaccard similarity over space-separated tokens of the normalized form.
pub fn token_jaccard(a: &str, b: &str) -> f32 {
    let na = normalize(a);
    let nb = normalize(b);
    if na.is_empty() && nb.is_empty() {
        return 1.0;
    }
    if na.is_empty() || nb.is_empty() {
        return 0.0;
    }
    let ta: std::collections::HashSet<&str> = na.split_whitespace().collect();
    let tb: std::collections::HashSet<&str> = nb.split_whitespace().collect();
    let inter = ta.intersection(&tb).count();
    let union = ta.union(&tb).count();
    if union == 0 {
        0.0
    } else {
        inter as f32 / union as f32
    }
}

/// Produce a list of suggested entity match pairs, grouped per type.
///
/// The output is sorted by descending similarity so high-confidence
/// suggestions appear first.
pub fn find_entity_matches(
    entities: &[EntityCandidate],
    config: &EntityResolutionConfig,
) -> Vec<EntityMatchSuggestion> {
    let mut by_type: std::collections::HashMap<String, Vec<&EntityCandidate>> = Default::default();
    for e in entities {
        by_type.entry(e.entity_type.clone()).or_default().push(e);
    }

    let mut out = Vec::new();
    for (entity_type, mut bucket) in by_type {
        if bucket.len() > config.per_type_limit {
            bucket.truncate(config.per_type_limit);
        }
        for i in 0..bucket.len() {
            for j in (i + 1)..bucket.len() {
                let a = bucket[i];
                let b = bucket[j];
                if normalize(&a.value) == normalize(&b.value) {
                    out.push(EntityMatchSuggestion {
                        canonical_id: a.id.min(b.id),
                        canonical_value: if a.id < b.id {
                            a.value.clone()
                        } else {
                            b.value.clone()
                        },
                        alias_id: a.id.max(b.id),
                        alias_value: if a.id < b.id {
                            b.value.clone()
                        } else {
                            a.value.clone()
                        },
                        entity_type: entity_type.clone(),
                        similarity: 1.0,
                        reason: "exact match after normalization".to_string(),
                    });
                    continue;
                }
                let sim = token_jaccard(&a.value, &b.value);
                if sim >= config.similarity_threshold {
                    let (can, ali) = if a.id < b.id { (a, b) } else { (b, a) };
                    out.push(EntityMatchSuggestion {
                        canonical_id: can.id,
                        canonical_value: can.value.clone(),
                        alias_id: ali.id,
                        alias_value: ali.value.clone(),
                        entity_type: entity_type.clone(),
                        similarity: sim,
                        reason: format!("token Jaccard {:.2}", sim),
                    });
                }
            }
        }
    }

    out.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ent(id: i64, ty: &str, val: &str) -> EntityCandidate {
        EntityCandidate {
            id,
            entity_type: ty.to_string(),
            value: val.to_string(),
        }
    }

    #[test]
    fn normalize_strips_punctuation() {
        assert_eq!(normalize("J. Smith"), "j smith");
        assert_eq!(normalize("ACME, Inc."), "acme inc");
        assert_eq!(normalize("  spaced  out  "), "spaced out");
    }

    #[test]
    fn exact_normalization_matches_grouped() {
        let entities = vec![
            ent(1, "person", "John Smith"),
            ent(2, "person", "john smith"),
            ent(3, "person", "Jane Doe"),
        ];
        let matches = find_entity_matches(&entities, &EntityResolutionConfig::default());
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].canonical_id, 1);
        assert_eq!(matches[0].alias_id, 2);
        assert!((matches[0].similarity - 1.0).abs() < 1e-6);
    }

    #[test]
    fn token_jaccard_finds_partial_matches() {
        let entities = vec![
            ent(1, "person", "John A. Smith"),
            ent(2, "person", "John Smith"),
        ];
        let matches = find_entity_matches(
            &entities,
            &EntityResolutionConfig {
                similarity_threshold: 0.5,
                per_type_limit: 100,
            },
        );
        assert_eq!(matches.len(), 1);
        assert!(matches[0].similarity >= 0.5);
    }

    #[test]
    fn does_not_match_across_types() {
        let entities = vec![ent(1, "person", "Atlas"), ent(2, "organization", "Atlas")];
        let matches = find_entity_matches(&entities, &EntityResolutionConfig::default());
        assert!(matches.is_empty());
    }
}
