use crate::extractors::audio::AudioExtractor;
use crate::extractors::ocr::OcrExtractor;
use crate::extractors::pdf::PdfExtractor;
use rayon::prelude::*;
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtractorConfig {
    pub use_gpu_ocr: bool,
    pub whisper_model_path: Option<PathBuf>,
}

pub struct Deconstructor {
    pdf: PdfExtractor,
    ocr: OcrExtractor,
    audio: Option<AudioExtractor>,
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

        Ok(Deconstructor { pdf, ocr, audio })
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

                if text.len() < 100 {
                    warn!(
                        "PDF extracted minimal text ({} chars), attempting page-by-page OCR",
                        text.len()
                    );
                    let ocr_result = self.extract_scanned_pdf(path);
                    match ocr_result {
                        Ok(ocr_text) if !ocr_text.is_empty() => {
                            info!(
                                "Scanned PDF OCR successful: {} chars extracted",
                                ocr_text.len()
                            );
                            (ocr_text, "pdf_ocr".to_string())
                        }
                        Ok(_) => {
                            warn!("Scanned PDF OCR returned empty text");
                            (text, "pdf".to_string())
                        }
                        Err(e) => {
                            warn!("Scanned PDF OCR failed: {}", e);
                            (text, "pdf".to_string())
                        }
                    }
                } else {
                    (text, "pdf".to_string())
                }
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

    /// Extract text from a scanned PDF by rendering pages and running OCR
    /// Uses parallel processing for efficiency
    pub fn extract_scanned_pdf(&self, path: &Path) -> Result<String, ExtractionError> {
        let path_str = path.to_string_lossy();
        info!(
            "Extracting scanned PDF with parallel page-by-page OCR: {}",
            path_str
        );

        let page_count = self
            .pdf
            .get_page_count(path)
            .map_err(|e| ExtractionError::PdfError(e.to_string()))?;

        info!("Rendering {} pages for OCR", page_count);

        // Render all pages first (parallel)
        let pages: Vec<(u32, image::DynamicImage)> = (1..=page_count as u32)
            .into_par_iter()
            .filter_map(|page_num| match self.pdf.render_page(path, page_num) {
                Ok(img) => Some((page_num, img)),
                Err(e) => {
                    warn!("Failed to render page {}: {}", page_num, e);
                    None
                }
            })
            .collect();

        // Sort by page number to maintain order
        let mut pages: Vec<_> = pages;
        pages.sort_by_key(|(num, _)| *num);

        let mut full_text = String::new();
        let early_termination = 500;

        // Process OCR on rendered pages
        for (page_num, page_img) in pages {
            match self.ocr.extract_text_from_image(&page_img) {
                Ok(page_text) => {
                    if !page_text.is_empty() {
                        full_text.push_str(&page_text);
                        full_text.push('\n');

                        if full_text.len() >= early_termination {
                            info!(
                                "Early termination at page {} ({} chars total)",
                                page_num,
                                full_text.len()
                            );
                            break;
                        }
                    }
                }
                Err(e) => {
                    warn!("OCR failed for page {}: {}", page_num, e);
                }
            }
        }

        let result = full_text.trim().to_string();
        info!(
            "Scanned PDF OCR complete: {} chars from {} pages",
            result.len(),
            page_count
        );

        Ok(result)
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
