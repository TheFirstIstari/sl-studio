use crate::extractors::audio::AudioExtractor;
use crate::extractors::ocr::OcrExtractor;
use crate::extractors::pdf::PdfExtractor;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{info, warn};

#[derive(Error, Debug)]
pub enum ExtractionError {
    #[error("PDF extraction error: {0}")]
    PdfError(String),
    #[error("OCR error: {0}")]
    OcrError(String),
    #[error("Audio transcription error: {0}")]
    AudioError(String),
    #[error("Unsupported file type: {0}")]
    UnsupportedType(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("No extractors initialized")]
    NotInitialized,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub text: String,
    pub source_file: String,
    pub file_type: String,
    pub char_count: usize,
    pub is_partial: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractorConfig {
    pub use_gpu_ocr: bool,
    pub whisper_model_path: Option<PathBuf>,
}

impl Default for ExtractorConfig {
    fn default() -> Self {
        ExtractorConfig {
            use_gpu_ocr: false,
            whisper_model_path: None,
        }
    }
}

pub struct Deconstructor {
    pdf: PdfExtractor,
    ocr: OcrExtractor,
    audio: Option<AudioExtractor>,
    config: ExtractorConfig,
}

impl Deconstructor {
    pub fn new(config: ExtractorConfig) -> Result<Self, ExtractionError> {
        let pdf = PdfExtractor::new();
        let ocr = OcrExtractor::new().map_err(|e| ExtractionError::OcrError(e.to_string()))?;

        let audio = if let Some(model_path) = &config.whisper_model_path {
            Some(
                AudioExtractor::new(model_path)
                    .map_err(|e| ExtractionError::AudioError(e.to_string()))?,
            )
        } else {
            None
        };

        info!("Deconstructor initialized");

        Ok(Deconstructor {
            pdf,
            ocr,
            audio,
            config,
        })
    }

    pub fn extract(&self, path: &Path) -> Result<ExtractionResult, ExtractionError> {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        let (text, file_type) = match ext.as_str() {
            "pdf" => {
                let text = self
                    .pdf
                    .extract_text_with_fallback(path)
                    .map_err(|e| ExtractionError::PdfError(e.to_string()))?;
                (text, "pdf".to_string())
            }
            "jpg" | "jpeg" | "png" | "bmp" | "tiff" | "tif" => {
                let text = self
                    .ocr
                    .extract_text(path)
                    .map_err(|e| ExtractionError::OcrError(e.to_string()))?;
                (text, "image".to_string())
            }
            "mp3" | "wav" | "mp4" | "m4a" | "m4v" | "ogg" | "flac" => {
                if let Some(audio) = &self.audio {
                    let text = audio
                        .transcribe(path)
                        .map_err(|e| ExtractionError::AudioError(e.to_string()))?;
                    (text, "audio".to_string())
                } else {
                    return Err(ExtractionError::AudioError(
                        "Audio extractor not initialized (no model path)".to_string(),
                    ));
                }
            }
            "txt" | "md" | "json" | "xml" | "csv" => {
                let text = std::fs::read_to_string(path)?;
                (text.trim().to_string(), "text".to_string())
            }
            _ => {
                warn!("Unsupported file type: {}", ext);
                return Err(ExtractionError::UnsupportedType(ext));
            }
        };

        let char_count = text.len();
        info!(
            "Extracted {} chars from {} ({})",
            char_count, file_name, file_type
        );

        Ok(ExtractionResult {
            text,
            source_file: file_name,
            file_type,
            char_count,
            is_partial: false,
        })
    }

    pub fn extract_with_chunking(
        &self,
        path: &Path,
        max_chars: usize,
        overlap: usize,
    ) -> Result<Vec<ExtractionResult>, ExtractionError> {
        let result = self.extract(path)?;

        if result.char_count <= max_chars {
            return Ok(vec![result]);
        }

        let text = result.text;
        let file_name = result.source_file.clone();
        let file_type = result.file_type.clone();

        let mut chunks = Vec::new();
        let mut start = 0;

        while start < text.len() {
            let end = std::cmp::min(start + max_chars, text.len());
            let chunk_text = if end < text.len() {
                text[start..end].to_string()
            } else {
                text[start..].to_string()
            };

            chunks.push(ExtractionResult {
                text: chunk_text.clone(),
                source_file: if start == 0 {
                    file_name.clone()
                } else {
                    format!("{} (Part {})", file_name, start / max_chars + 1)
                },
                file_type: file_type.clone(),
                char_count: chunk_text.len(),
                is_partial: end < text.len(),
            });

            if end >= text.len() {
                break;
            }

            start += max_chars - overlap;
        }

        info!("Split {} chars into {} chunks", text.len(), chunks.len());
        Ok(chunks)
    }

    pub fn is_audio_available(&self) -> bool {
        self.audio.is_some()
    }

    pub fn supported_extensions() -> Vec<&'static str> {
        vec![
            "pdf", "jpg", "jpeg", "png", "bmp", "tiff", "tif", "mp3", "wav", "mp4", "m4a", "m4v",
            "ogg", "flac", "txt", "md", "json", "xml", "csv",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_deconstructor_creation() {
        let config = ExtractorConfig::default();
        let result = Deconstructor::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_supported_extensions() {
        let exts = Deconstructor::supported_extensions();
        assert!(exts.contains(&"pdf"));
        assert!(exts.contains(&"mp3"));
    }
}
