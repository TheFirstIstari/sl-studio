// Image extractor for SL Studio

use crate::Metadata;
use anyhow::Result;
use tracing::info;

pub async fn extract_image(path: &str) -> Result<Metadata> {
    let _content = std::fs::read(path)?;
    info!("Extracted image: {}", path);

    Ok(Metadata {
        filename: path.to_string(),
        category: "Image".to_string(),
        severity_score: 3,
        confidence: Some(0.7),
        identified_crime: None,
        fact_summary: "Image extracted".to_string(),
        fingerprint: "img_meta".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    })
}
