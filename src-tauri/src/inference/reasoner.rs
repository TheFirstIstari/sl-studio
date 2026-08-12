// Reasoner pipeline for SL Studio

use anyhow::Result;
use tracing::info;

/// Inference reasoner that extracts facts and entities from extracted text.
pub struct Reasoner {
    pipeline: crate::inference::mlx_pipeline::MlxPipeline,
}

impl Reasoner {
    pub fn new(pipeline: crate::inference::mlx_pipeline::MlxPipeline) -> Self {
        Self { pipeline }
    }

    pub fn reason(&self, text: &str) -> Result<String> {
        info!("Reasoning on input: {} chars", text.len());
        let prompt = format!("Extract facts and entities from: {}", text);
        self.pipeline.infer(&prompt, 2048)
    }

    pub fn extract_facts(&self, text: &str) -> Result<Vec<crate::Fact>> {
        info!("Extracting facts from: {} chars", text.len());
        let prompt = format!("Extract facts and entities from: {}", text);
        let content = self.pipeline.infer(&prompt, 2048)?;
        Ok(vec![crate::Fact {
            id: 0,
            fingerprint: "generated".to_string(),
            filename: "unknown".to_string(),
            fact_summary: content,
            category: Some("Unknown".to_string()),
            identified_crime: None,
            severity_score: 5,
            confidence: Some(0.8),
            created_at: chrono::Utc::now().to_rfc3339(),
        }])
    }
}
