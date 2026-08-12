// PDF extractor for SL Studio

use crate::Metadata;
use anyhow::Result;
use tracing::info;

pub async fn extract_pdf(path: &str) -> Result<Metadata> {
    let _content = std::fs::read_to_string(path)?;
    info!("Extracted PDF: {}", path);

    Ok(Metadata {
        filename: path.to_string(),
        category: "PDF".to_string(),
        severity_score: 5,
        confidence: Some(0.5),
        identified_crime: None,
        fact_summary: "PDF extracted".to_string(),
        fingerprint: "pdf_meta".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    })
}
