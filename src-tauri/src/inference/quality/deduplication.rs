//! Near-duplicate detection for intelligence rows.
//!
//! This module provides similarity-based grouping of facts so the user can
//! review and merge them. It operates on lightweight tuples rather than the
//! full `IntelligenceEntry` so it stays decoupled from storage details.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// Lightweight projection of an intelligence row used for dedup matching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupCandidate {
    pub id: i64,
    pub fact_summary: String,
    pub category: Option<String>,
    pub associated_date: Option<String>,
    pub severity_score: i32,
    pub confidence: Option<f64>,
}

/// A group of facts considered duplicates of one another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    /// The fact id chosen as the representative (highest severity, ties
    /// broken by highest confidence then lowest id).
    pub keeper_id: i64,
    /// All ids in the group, including the keeper.
    pub member_ids: Vec<i64>,
    /// Average pairwise similarity within the group, 0.0-1.0.
    pub similarity: f32,
}

/// Configuration knobs for `find_duplicate_groups`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicationConfig {
    /// Jaccard similarity threshold (0.0-1.0). Pairs above this are
    /// considered duplicates.
    pub similarity_threshold: f32,
    /// If true, candidates must share the same `category` (None == None
    /// counts as same).
    pub require_same_category: bool,
    /// If true, candidates must share the same `associated_date`.
    pub require_same_date: bool,
}

impl Default for DeduplicationConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.85,
            require_same_category: true,
            require_same_date: false,
        }
    }
}

/// Jaccard similarity over case-insensitive word sets. Returns 0.0 for empty
/// inputs and 1.0 for two empty-but-equal inputs (degenerate).
pub fn jaccard_similarity(a: &str, b: &str) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let words_a: HashSet<String> = a
        .to_lowercase()
        .split_whitespace()
        .map(String::from)
        .collect();
    let words_b: HashSet<String> = b
        .to_lowercase()
        .split_whitespace()
        .map(String::from)
        .collect();
    let intersection = words_a.intersection(&words_b).count();
    let union = words_a.union(&words_b).count();
    if union == 0 {
        0.0
    } else {
        intersection as f32 / union as f32
    }
}

fn category_matches(a: &Option<String>, b: &Option<String>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => x.eq_ignore_ascii_case(y),
        (None, None) => true,
        _ => false,
    }
}

fn date_matches(a: &Option<String>, b: &Option<String>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => x == y,
        (None, None) => true,
        _ => false,
    }
}

/// Pick the "keeper" id from a set of candidate indices.
///
/// Heuristic: highest severity, ties broken by highest confidence, ties
/// broken by lowest id (oldest fact wins to preserve provenance).
fn pick_keeper(candidates: &[DedupCandidate], indices: &[usize]) -> i64 {
    let best = indices
        .iter()
        .map(|&i| &candidates[i])
        .max_by(|a, b| {
            a.severity_score
                .cmp(&b.severity_score)
                .then_with(|| {
                    a.confidence
                        .unwrap_or(0.0)
                        .partial_cmp(&b.confidence.unwrap_or(0.0))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| b.id.cmp(&a.id))
        })
        .expect("group always has at least one member");
    best.id
}

/// Group candidates into duplicate clusters using a simple union-find walk.
///
/// O(n^2) on the candidate slice; acceptable for the modest fact volumes a
/// single investigator works with. For very large databases, callers should
/// pre-filter by category/date in SQL before passing in.
pub fn find_duplicate_groups(
    candidates: &[DedupCandidate],
    config: &DeduplicationConfig,
) -> Vec<DuplicateGroup> {
    let n = candidates.len();
    if n < 2 {
        return Vec::new();
    }

    // parent[i] = representative index; classic union-find
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut [usize], i: usize) -> usize {
        let mut root = i;
        while parent[root] != root {
            root = parent[root];
        }
        let mut cur = i;
        while parent[cur] != root {
            let next = parent[cur];
            parent[cur] = root;
            cur = next;
        }
        root
    }

    // Track per-pair similarity for averaging later.
    let mut sim_sum: Vec<f32> = vec![0.0; n];
    let mut sim_count: Vec<u32> = vec![0; n];

    for i in 0..n {
        for j in (i + 1)..n {
            if config.require_same_category
                && !category_matches(&candidates[i].category, &candidates[j].category)
            {
                continue;
            }
            if config.require_same_date
                && !date_matches(
                    &candidates[i].associated_date,
                    &candidates[j].associated_date,
                )
            {
                continue;
            }
            let sim = jaccard_similarity(&candidates[i].fact_summary, &candidates[j].fact_summary);
            if sim >= config.similarity_threshold {
                let ri = find(&mut parent, i);
                let rj = find(&mut parent, j);
                if ri != rj {
                    parent[ri] = rj;
                }
                sim_sum[i] += sim;
                sim_sum[j] += sim;
                sim_count[i] += 1;
                sim_count[j] += 1;
            }
        }
    }

    // Bucket members by root
    let mut groups: std::collections::HashMap<usize, Vec<usize>> = Default::default();
    for i in 0..n {
        let root = find(&mut parent, i);
        groups.entry(root).or_default().push(i);
    }

    let mut out = Vec::new();
    for (_root, indices) in groups {
        if indices.len() < 2 {
            continue;
        }
        let keeper_id = pick_keeper(candidates, &indices);
        let member_ids: Vec<i64> = indices.iter().map(|&i| candidates[i].id).collect();
        let total: f32 = indices.iter().map(|&i| sim_sum[i]).sum();
        let count: u32 = indices.iter().map(|&i| sim_count[i]).sum();
        let similarity = if count > 0 { total / count as f32 } else { 0.0 };
        out.push(DuplicateGroup {
            keeper_id,
            member_ids,
            similarity,
        });
    }

    // Sort largest-first so the UI shows the highest-impact groups first.
    out.sort_by(|a, b| b.member_ids.len().cmp(&a.member_ids.len()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(id: i64, summary: &str, category: Option<&str>, severity: i32) -> DedupCandidate {
        DedupCandidate {
            id,
            fact_summary: summary.to_string(),
            category: category.map(String::from),
            associated_date: None,
            severity_score: severity,
            confidence: None,
        }
    }

    #[test]
    fn jaccard_basic() {
        assert!((jaccard_similarity("hello world", "hello world") - 1.0).abs() < 1e-6);
        assert!((jaccard_similarity("hello world", "hello there") - (1.0 / 3.0)).abs() < 1e-6);
        assert_eq!(jaccard_similarity("", "anything"), 0.0);
        assert_eq!(jaccard_similarity("", ""), 1.0);
    }

    #[test]
    fn groups_near_duplicates() {
        let candidates = vec![
            cand(1, "Payment of $1000 to Acme Corp", Some("Financial"), 5),
            cand(2, "Payment of $1000 to Acme Corp", Some("Financial"), 8),
            cand(3, "Unrelated fact about a meeting", Some("Meeting"), 3),
        ];
        let cfg = DeduplicationConfig {
            similarity_threshold: 0.5,
            require_same_category: true,
            require_same_date: false,
        };
        let groups = find_duplicate_groups(&candidates, &cfg);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].member_ids.len(), 2);
        // Highest-severity wins as keeper.
        assert_eq!(groups[0].keeper_id, 2);
    }

    #[test]
    fn ignores_singletons() {
        let candidates = vec![cand(1, "Single fact", Some("X"), 1)];
        let groups = find_duplicate_groups(&candidates, &DeduplicationConfig::default());
        assert!(groups.is_empty());
    }

    #[test]
    fn respects_category_constraint() {
        let candidates = vec![
            cand(1, "shared identical text here", Some("A"), 1),
            cand(2, "shared identical text here", Some("B"), 1),
        ];
        let cfg = DeduplicationConfig {
            similarity_threshold: 0.5,
            require_same_category: true,
            require_same_date: false,
        };
        assert!(find_duplicate_groups(&candidates, &cfg).is_empty());

        let cfg2 = DeduplicationConfig {
            require_same_category: false,
            ..cfg
        };
        assert_eq!(find_duplicate_groups(&candidates, &cfg2).len(), 1);
    }
}
