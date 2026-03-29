use crate::inference::reasoner::Fact;
use std::collections::HashMap;

/// Configuration for deduplication
#[derive(Debug, Clone)]
pub struct DeduplicationConfig {
    /// Similarity threshold (0.0-1.0), facts above this are considered duplicates
    pub similarity_threshold: f32,
    /// Fields to match on for deduplication
    pub match_on_fields: Vec<MatchField>,
    /// Strategy for merging duplicates
    pub merge_strategy: MergeStrategy,
}

/// Fields that can be used for deduplication matching
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MatchField {
    FactSummary,
    AssociatedDate,
    Category,
    IdentifiedCrime,
}

/// Strategy for handling duplicate facts
#[derive(Debug, Clone)]
pub enum MergeStrategy {
    /// Keep the fact with highest confidence
    KeepHighestConfidence,
    /// Keep the fact with highest severity
    KeepMostSevere,
    /// Merge all facts into one (concatenate summaries, etc.)
    MergeAll,
}

impl Default for DeduplicationConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.85,
            match_on_fields: vec![
                MatchField::FactSummary,
                MatchField::AssociatedDate,
                MatchField::Category,
            ],
            merge_strategy: MergeStrategy::KeepHighestConfidence,
        }
    }
}

impl DeduplicationConfig {
    /// Check if two facts are duplicates based on the configuration
    pub fn are_duplicates(&self, fact1: &Fact, fact2: &Fact) -> bool {
        // Quick check: if they don't match on any required field, they're not duplicates
        for field in &self.match_on_fields {
            if !self.field_matches(field, fact1, fact2) {
                return false;
            }
        }
        true
    }

    /// Check if a specific field matches between two facts
    fn field_matches(&self, field: &MatchField, fact1: &Fact, fact2: &Fact) -> bool {
        match field {
            MatchField::FactSummary => {
                // Simple similarity check - in production would use better string similarity
                let similarity = self.string_similarity(&fact1.summary, &fact2.summary);
                similarity >= self.similarity_threshold
            }
            MatchField::AssociatedDate => {
                // Dates must match exactly (both present or both absent)
                match (&fact1.date, &fact2.date) {
                    (Some(d1), Some(d2)) => d1 == d2,
                    (None, None) => true,
                    _ => false,
                }
            }
            MatchField::Category => {
                // Both must have same category (both present or both absent)
                match (&fact1.fact_type, &fact2.fact_type) {
                    (Some(c1), Some(c2)) => c1 == c2,
                    (None, None) => true,
                    _ => false,
                }
            }
            MatchField::IdentifiedCrime => {
                // Both must have same crime (both present or both absent)
                match (&fact1.crime, &fact2.crime) {
                    (Some(c1), Some(c2)) => c1 == c2,
                    (None, None) => true,
                    _ => false,
                }
            }
        }
    }

    /// Simple string similarity (0.0-1.0) - in production use better algorithm like Levenshtein
    fn string_similarity(&self, s1: &str, s2: &str) -> f32 {
        if s1.is_empty() || s2.is_empty() {
            return 0.0;
        }
        
        // Simple Jaccard similarity on word sets for demo
        let words1: std::collections::HashSet<_> = s1.to_lowercase().split_whitespace().collect();
        let words2: std::collections::HashSet<_> = s2.to_lowercase().split_whitespace().collect();
        
        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();
        
        if union == 0 {
            return 1.0;
        }
        
        intersection as f32 / union as f32
    }

    /// Merge two facts according to the merge strategy
    pub fn merge_facts(&self, fact1: Fact, fact2: Fact) -> Fact {
        match self.merge_strategy {
            MergeStrategy::KeepHighestConfidence => {
                // In a real implementation, we'd have confidence scores
                // For now, keep the first one (placeholder)
                fact1
            }
            MergeStrategy::KeepMostSevere => {
                // Keep the fact with higher severity
                if fact1.severity >= fact2.severity {
                    fact1
                } else {
                    fact2
                }
            }
            MergeStrategy::MergeAll => {
                // Merge facts - simplified implementation
                let mut merged = fact1;
                
                // Combine summaries
                if !merged.summary.is_empty() && !fact2.summary.is_empty() {
                    merged.summary = format!("{}; {}", merged.summary, fact2.summary);
                } else if !fact2.summary.is_empty() {
                    merged.summary = fact2.summary;
                }
                
                // Prefer non-empty values from fact2
                if merged.date.is_none() {
                    merged.date = fact2.date;
                }
                if merged.fact_type.is_empty() {
                    merged.fact_type = fact2.fact_type;
                }
                if merged.crime.is_none() {
                    merged.crime = fact2.crime;
                }
                
                merged
            }
        }
    }

    /// Deduplicate a vector of facts
    pub fn deduplicate(&self, mut facts: Vec<Fact>) -> Vec<Fact> {
        if facts.is_empty() {
            return vec![];
        }
        
        let mut unique_facts = Vec::new();
        let mut processed = std::collections::HashSet::new();
        
        for (i, fact1) in facts.iter().enumerate() {
            if processed.contains(&i) {
                continue;
            }
            
            // Start a new group with this fact
            let mut group = Vec::new();
            group.push((i, fact1.clone()));
            processed.insert(i);
            
            // Find all facts that are duplicates of this one
            for (j, fact2) in facts.iter().enumerate().skip(i + 1) {
                if processed.contains(&j) {
                    continue;
                }
                
                if self.are_duplicates(fact1, fact2) {
                    group.push((j, fact2.clone()));
                    processed.insert(j);
                }
            }
            
            // Merge all facts in the group
            if group.len() == 1 {
                // No duplicates, keep original
                unique_facts.push(group[0].1.clone());
            } else {
                // Merge all facts in the group
                let mut merged = group[0].1.clone();
                for (_, fact) in group.iter().skip(1) {
                    merged = self.merge_facts(merged, fact.clone());
                }
                unique_facts.push(merged);
            }
        }
        
        unique_facts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_deduplication_config_default() {
        let config = DeduplicationConfig::default();
        assert_eq!(config.similarity_threshold, 0.85);
        assert!(config.merge_strategy == MergeStrategy::KeepHighestConfidence);
    }
    
    #[test]
    fn test_are_duplicates_exact_match() {
        let config = DeduplicationConfig::default();
        let fact1 = Fact {
            source: "doc1.pdf".to_string(),
            date: Some("2023-01-15".to_string()),
            summary": "Payment of $1000 to Acme Corp".to_string(),
            fact_type: "Financial".to_string(),
            crime: Some("Fraud".to_string()),
            severity: 5,
        };
        
        let fact2 = Fact {
            source: "doc2.pdf".to_string(),
            date: Some("2023-01-15".to_string()),
            summary": "Payment of $1000 to Acme Corp".to_string(),
            fact_type: "Financial".to_string(),
            crime: Some("Fraud".to_string()),
            severity: 3,
        };
        
        assert!(config.are_duplicates(&fact1, &fact2));
    }
    
    #[test]
    fn test_are_duplicates_different_dates() {
        let config = DeduplicationConfig::default();
        let fact1 = Fact {
            source: "doc1.pdf".to_string(),
            date: Some("2023-01-15".to_string()),
            summary": "Payment of $1000 to Acme Corp".to_string(),
            fact_type: "Financial".to_string(),
            crime: Some("Fraud".to_string()),
            severity: 5,
        };
        
        let fact2 = Fact {
            source: "doc2.pdf".to_string(),
            date: Some("2023-01-16".to_string()),
            summary": "Payment of $1000 to Acme Corp".to_string(),
            fact_type: "Financial".to_string(),
            crime: Some("Fraud".to_string()),
            severity: 3,
        };
        
        assert!(!config.are_duplicates(&fact1, &fact2));
    }
    
    #[test]
    fn test_merge_facts_keep_most_severe() {
        let config = DeduplicationConfig {
            merge_strategy: MergeStrategy::KeepMostSevere,
            ..DeduplicationConfig::default()
        };
        
        let fact1 = Fact {
            source: "doc1.pdf".to_string(),
            date: None,
            summary": "Payment of $1000".to_string(),
            fact_type: "Financial".to_string(),
            crime: None,
            severity: 3,
        };
        
        let fact2 = Fact {
            source: "doc2.pdf".to_string(),
            date: None,
            summary": "Payment of $1500".to_string(),
            fact_type: "Financial".to_string(),
            crime: None,
            severity: 8,
        };
        
        let merged = config.merge_facts(fact1, fact2);
        assert_eq!(merged.severity, 8);
        assert_eq!(merged.summary, "Payment of $1500");
    }
}