// DOCX extractor for SL Studio

use crate::Metadata;
use anyhow::Result;
use tracing::info;

pub async fn extract_docx(path: &str) -> Result<Metadata> {
    let _content = std::fs::read_to_string(path)?;
    info!("Extracted DOCX: {}", path);

    Ok(Metadata {
        filename: path.to_string(),
        category: "Document".to_string(),
        severity_score: 4,
        confidence: Some(0.8),
        identified_crime: None,
        fact_summary: "DOCX extracted".to_string(),
        fingerprint: "docx_meta".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    })
}
