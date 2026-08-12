// Audio extractor for SL Studio

use crate::Metadata;
use anyhow::Result;
use tracing::info;

pub async fn extract_audio(path: &str) -> Result<Metadata> {
    let _content = std::fs::read(path)?;
    info!("Extracted audio: {}", path);

    Ok(Metadata {
        filename: path.to_string(),
        category: "Audio".to_string(),
        severity_score: 2,
        confidence: Some(0.6),
        identified_crime: None,
        fact_summary: "Audio extracted".to_string(),
        fingerprint: "audio_meta".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    })
}
