// Extractors for SL Studio

pub mod audio;
pub mod docx;
pub mod image;
pub mod pdf;

pub use audio::extract_audio;
pub use docx::extract_docx;
pub use image::extract_image;
pub use pdf::extract_pdf;

use anyhow::Result;
use std::collections::HashMap;
use tracing::info;

/// Extract document metadata from a file path.
/// Dispatches to the appropriate extractor based on file extension.
pub async fn extract_metadata_from_path(path: &str) -> Result<crate::DocumentMetadata> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let source = match ext.as_str() {
        "pdf" => "pdf",
        "png" | "jpg" | "jpeg" | "tiff" | "bmp" => "image",
        "mp3" | "wav" | "m4a" | "flac" | "aac" => "audio",
        "docx" => "docx",
        _ => "text",
    };

    let mut raw = HashMap::new();
    raw.insert("file_type".to_string(), ext);

    let metadata = std::fs::metadata(path)?;
    raw.insert("file_size".to_string(), metadata.len().to_string());

    let file_name = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    raw.insert("file_name".to_string(), file_name.clone());

    info!("Extracting metadata for: {}", path);

    Ok(crate::DocumentMetadata {
        source: source.to_string(),
        title: Some(file_name),
        author: None,
        subject: None,
        creator: None,
        producer: None,
        created_at: None,
        modified_at: None,
        keywords: None,
        camera_model: None,
        gps_latitude: None,
        gps_longitude: None,
        audio_duration_seconds: None,
        audio_sample_rate: None,
        audio_channels: None,
        audio_format: None,
        audio_codec: None,
        audio_bits_per_sample: None,
        raw,
    })
}
