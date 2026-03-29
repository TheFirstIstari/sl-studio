use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;
use tracing::{info, warn};

#[derive(Error, Debug)]
pub enum OcrError {
    #[error("Failed to read image: {0}")]
    ImageError(String),
    #[error("OCR processing failed: {0}")]
    ProcessingError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Multi-page error: {0}")]
    MultiPageError(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OcrResult {
    pub path: String,
    pub text: String,
    pub char_count: usize,
}

#[allow(dead_code)]
pub struct OcrExtractor {
    engine: ocrs::OcrEngine,
    batch_size: usize,
}

impl OcrExtractor {
    pub fn new() -> Result<Self, OcrError> {
        let engine = ocrs::OcrEngine::new(Default::default())
            .map_err(|e| OcrError::ProcessingError(format!("{:?}", e)))?;

        info!("OCR engine initialized");
        Ok(OcrExtractor {
            engine,
            batch_size: 8,
        })
    }

    pub fn with_batch_size(batch_size: usize) -> Result<Self, OcrError> {
        let engine = ocrs::OcrEngine::new(Default::default())
            .map_err(|e| OcrError::ProcessingError(format!("{:?}", e)))?;

        info!("OCR engine initialized with batch size {}", batch_size);
        Ok(OcrExtractor { engine, batch_size })
    }

    pub fn extract_text(&self, path: &Path) -> Result<String, OcrError> {
        let path_str = path.to_string_lossy();
        info!("Running OCR on: {}", path_str);

        let img = image::open(path).map_err(|e| OcrError::ImageError(e.to_string()))?;

        let rgb = img.to_rgb8();
        let (width, height) = rgb.dimensions();

        let input = self
            .engine
            .prepare_input(
                ocrs::ImageSource::from_bytes(rgb.as_raw(), (width, height))
                    .map_err(|e| OcrError::ProcessingError(format!("{:?}", e)))?,
            )
            .map_err(|e| OcrError::ProcessingError(format!("{:?}", e)))?;

        let text = self
            .engine
            .get_text(&input)
            .map_err(|e| OcrError::ProcessingError(format!("{:?}", e)))?;

        let trimmed = text.trim();
        if trimmed.is_empty() {
            warn!("OCR returned empty text for: {}", path_str);
        } else {
            info!("OCR extracted {} chars from {}", trimmed.len(), path_str);
        }

        Ok(trimmed.to_string())
    }

    pub fn extract_batch(&self, paths: &[PathBuf]) -> Vec<OcrResult> {
        let mut results = Vec::new();

        for path in paths {
            match self.extract_text(path) {
                Ok(text) => {
                    results.push(OcrResult {
                        path: path.to_string_lossy().to_string(),
                        text: text.clone(),
                        char_count: text.len(),
                    });
                }
                Err(e) => {
                    warn!("OCR failed for {}: {}", path.display(), e);
                    results.push(OcrResult {
                        path: path.to_string_lossy().to_string(),
                        text: String::new(),
                        char_count: 0,
                    });
                }
            }
        }

        results
    }

    pub fn extract_multipage_tiff(&self, path: &Path) -> Result<String, OcrError> {
        let path_str = path.to_string_lossy();
        info!("Running OCR on multi-page TIFF: {}", path_str);

        let img = image::open(path).map_err(|e| OcrError::ImageError(e.to_string()))?;

        let rgb = img.to_rgb8();
        let (width, height) = rgb.dimensions();

        info!("Processing TIFF ({}x{})", width, height);

        let input = self
            .engine
            .prepare_input(
                ocrs::ImageSource::from_bytes(rgb.as_raw(), (width, height))
                    .map_err(|e| OcrError::ProcessingError(format!("{:?}", e)))?,
            )
            .map_err(|e| OcrError::ProcessingError(format!("{:?}", e)))?;

        let text = self
            .engine
            .get_text(&input)
            .map_err(|e| OcrError::ProcessingError(format!("{:?}", e)))?;

        let trimmed = text.trim();

        if trimmed.is_empty() {
            warn!("TIFF returned empty text: {}", path_str);
        } else {
            info!("OCR extracted {} chars from TIFF", trimmed.len());
        }

        Ok(trimmed.to_string())
    }

    #[allow(unused_variables)]
    pub fn is_multipage_tiff(path: &Path) -> bool {
        false
    }
}

impl Default for OcrExtractor {
    fn default() -> Self {
        Self::new().expect("Failed to create OCR extractor")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_ocr_extractor_creation() {
        let result = OcrExtractor::new();
        assert!(result.is_ok());
    }
}
