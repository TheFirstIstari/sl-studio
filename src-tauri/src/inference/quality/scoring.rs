use crate::inference::reasoner::{Fact, AnalysisResult};
use serde::{Deserialize, Serialize};

/// Represents the quality metrics for an extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionQuality {
    /// LLM's stated confidence (0.0-1.0)
    pub confidence: f32,
    /// Percentage of source text processed (0.0-1.0)
    pub text_coverage: f32,
    /// Entities per 1000 characters
    pub entity_density: f32,
    /// Length/relevance of source quote (0.0-1.0)
    pub quote_quality: f32,
    /// Weighted average of all metrics (0.0-1.0)
    pub overall: f32,
    /// Whether manual review is recommended
    pub retry_recommended: bool,
    /// Specific quality issues identified
    pub issues: Vec<QualityIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QualityIssue {
    LowConfidence,
    ShortQuote,
    PoorCoverage,
    EntityMismatch,
}

impl ExtractionQuality {
    /// Calculate quality metrics for a fact extraction
    pub fn calculate(
        fact: &Fact,
        source_text: &str,
        entities: &[String>,
        llm_confidence: f32,
        quote_length: usize,
    ) -> Self {
        // Confidence from LLM
        let confidence = llm_confidence.clamp(0.0, 1.0);

        // Text coverage: ratio of fact summary length to source text length (simplified)
        let text_coverage = (fact.summary.len() as f32 / source_text.len().max(1) as f32)
            .min(1.0)
            .max(0.0);

        // Entity density: number of entities per 1000 characters
        let entity_density = (entities.len() as f32 / source_text.len().max(1) as f32) * 1000.0;

        // Quote quality: based on quote length relative to fact summary (simplified)
        let quote_quality = (quote_length as f32 / fact.summary.len().max(1) as f32)
            .min(1.0)
            .max(0.0);

        // Weighted average (weights can be adjusted)
        let overall = (confidence * 0.4)
            + (text_coverage * 0.2)
            + (entity_density.min(10.0) / 10.0 * 0.2) // Cap entity density at 10 per 1000 chars for scoring
            + (quote_quality * 0.2);

        // Determine issues
        let mut issues = Vec::new();
        if confidence < 0.5 {
            issues.push(QualityIssue::LowConfidence);
        }
        if quote_length < 10 {
            issues.push(QualityIssue::ShortQuote);
        }
        if text_coverage < 0.1 {
            issues.push(QualityIssue::PoorCoverage);
        }
        // Entity mismatch: if we have entities in the fact but none extracted
        if !fact.summary.is_empty() && entities.is_empty() {
            issues.push(QualityIssue::EntityMismatch);
        }

        // Retry recommended if overall quality is below threshold or any critical issue
        let retry_recommended = overall < 0.5
            || issues.contains(&QualityIssue::LowConfidence)
            || issues.contains(&QualityIssue::PoorCoverage);

        ExtractionQuality {
            confidence,
            text_coverage,
            entity_density,
            quote_quality,
            overall,
            retry_recommended,
            issues,
        }
    }

    /// Get a color-coded quality level for UI display
    pub fn level(&self) -> QualityLevel {
        if self.overall >= 0.7 {
            QualityLevel::Good
        } else if self.overall >= 0.4 {
            QualityLevel::Marginal
        } else {
            QualityLevel::Poor
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityLevel {
    Good,
    Marginal,
    Poor,
}

impl QualityLevel {
    /// Get the color for this quality level (as hex string)
    pub fn color(&self) -> &'static str {
        match self {
            QualityLevel::Good => "#22C55E", // green
            QualityLevel::Marginal => "#F59E0B", // yellow
            QualityLevel::Poor => "#EF4444", // red
        }
    }

    /// Get the label for this quality level
    pub fn label(&self) -> &'static str {
        match self {
            QualityLevel::Good => "Good",
            QualityLevel::Marginal => "Marginal",
            QualityLevel::Poor => "Poor",
        }
    }
}