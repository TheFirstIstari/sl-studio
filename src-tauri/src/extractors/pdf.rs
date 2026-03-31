use pdf_extract::extract_text;
use std::path::Path;
use thiserror::Error;
use tracing::{error, info, warn};

#[derive(Error, Debug)]
pub enum PdfError {
    #[error("Failed to extract text from PDF: {0}")]
    ExtractionError(String),
    #[error("Failed to read file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("PDF too large: {0} pages (max: {1})")]
    TooLarge(usize, usize),
    #[error("Password protected PDF")]
    PasswordProtected,
    #[error("Corrupted PDF: {0}")]
    Corrupted(String),
}

/// Quality metrics for extracted text
#[derive(Debug, Clone)]
pub struct ExtractionQuality {
    pub char_count: usize,
    pub word_count: usize,
    pub line_count: usize,
    pub page_count: usize,
    pub confidence: f64,
    pub is_scanned: bool,
    pub issues: Vec<String>,
}

impl ExtractionQuality {
    pub fn calculate(text: &str, page_count: usize) -> Self {
        let char_count = text.len();
        let word_count = text.split_whitespace().count();
        let line_count = text.lines().count();

        // Calculate confidence based on various factors
        let mut confidence: f64 = 1.0;
        let mut issues = Vec::new();

        // Low character count
        if char_count < 100 {
            confidence *= 0.3;
            issues.push("Very low character count".to_string());
        } else if char_count < 500 {
            confidence *= 0.7;
        }

        // Check for scanned document indicators
        let is_scanned = char_count < 50
            || text
                .chars()
                .all(|c| c.is_whitespace() || c.is_ascii_punctuation());

        if is_scanned {
            confidence *= 0.2;
            issues.push("Document appears to be scanned (no text layer)".to_string());
        }

        // Check for reasonable word density
        let avg_word_len = if word_count > 0 {
            char_count as f64 / word_count as f64
        } else {
            0.0
        };
        if avg_word_len > 20.0 {
            confidence *= 0.5;
            issues.push("Unusual word length distribution".to_string());
        }

        ExtractionQuality {
            char_count,
            word_count,
            line_count,
            page_count,
            confidence: confidence.clamp(0.0, 1.0),
            is_scanned,
            issues,
        }
    }

    pub fn overall_score(&self) -> f64 {
        self.confidence
    }
}

#[allow(dead_code)]
pub struct PdfExtractor {
    max_pages: usize,
    streaming: bool,
    max_file_size_mb: f64,
}

impl PdfExtractor {
    pub fn new() -> Self {
        PdfExtractor {
            max_pages: 1000,
            streaming: false,
            max_file_size_mb: 500.0,
        }
    }

    pub fn with_limits(max_pages: usize, max_file_size_mb: f64) -> Self {
        PdfExtractor {
            max_pages,
            streaming: false,
            max_file_size_mb,
        }
    }

    /// Estimate page count from form feeds in text
    fn estimate_page_count(text: &str) -> usize {
        text.matches('\u{0C}').count() + 1
    }

    pub fn extract_text(&self, path: &Path) -> Result<String, PdfError> {
        let path_str = path.to_string_lossy();
        info!("Extracting text from PDF: {}", path_str);

        // Check file size
        if let Ok(metadata) = std::fs::metadata(path) {
            let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
            if size_mb > self.max_file_size_mb {
                error!(
                    "PDF too large: {} MB (max: {} MB)",
                    size_mb, self.max_file_size_mb
                );
                let page_count = 0;
                return Err(PdfError::TooLarge(page_count, self.max_pages));
            }
        }

        let text = extract_text(path).map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("password") || err_str.contains("encrypted") {
                PdfError::PasswordProtected
            } else if err_str.contains("PDF") && err_str.contains("error") {
                PdfError::Corrupted(err_str)
            } else {
                PdfError::ExtractionError(err_str)
            }
        })?;

        let trimmed = text.trim();
        if trimmed.is_empty() {
            warn!("PDF extracted empty text: {}", path_str);
            return Ok(String::new());
        }

        info!("Extracted {} chars from PDF: {}", trimmed.len(), path_str);
        Ok(trimmed.to_string())
    }

    pub fn extract_with_quality(
        &self,
        path: &Path,
    ) -> Result<(String, ExtractionQuality), PdfError> {
        let path_str = path.to_string_lossy();
        info!(
            "Extracting text with quality assessment from PDF: {}",
            path_str
        );

        // Check file size first
        if let Ok(metadata) = std::fs::metadata(path) {
            let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
            if size_mb > self.max_file_size_mb {
                let page_count = 0;
                return Err(PdfError::TooLarge(page_count, self.max_pages));
            }
        }

        let text = extract_text(path).map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("password") || err_str.contains("encrypted") {
                PdfError::PasswordProtected
            } else if err_str.contains("PDF") && err_str.contains("error") {
                PdfError::Corrupted(err_str)
            } else {
                PdfError::ExtractionError(err_str)
            }
        })?;

        // Estimate page count from form feeds
        let page_count = Self::estimate_page_count(&text);

        let trimmed = text.trim().to_string();
        let quality = ExtractionQuality::calculate(&trimmed, page_count);

        if quality.is_scanned {
            warn!("PDF appears to be scanned: {}", path_str);
        }

        info!(
            "Extracted {} chars with quality score {:.2}",
            trimmed.len(),
            quality.confidence
        );
        Ok((trimmed, quality))
    }

    pub fn extract_text_with_fallback(&self, path: &Path) -> Result<String, PdfError> {
        // Check file size first
        if let Ok(metadata) = std::fs::metadata(path) {
            let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
            if size_mb > 100.0 {
                warn!(
                    "Large PDF detected ({} MB), using limited extraction",
                    size_mb
                );
                return self.extract_text_limited(path, 100);
            }
        }

        let text = self.extract_text(path)?;

        if text.len() < 100 {
            warn!(
                "PDF has minimal text ({} chars), may be scanned",
                text.len()
            );
        }

        Ok(text)
    }

    pub fn extract_text_limited(&self, path: &Path, _max_pages: usize) -> Result<String, PdfError> {
        // Note: pdf_extract doesn't easily allow page-limited extraction
        // This extracts all text - for true page-limited we'd need a different crate
        let path_str = path.to_string_lossy();
        info!("Extracting text from PDF: {}", path_str);

        let text = extract_text(path).map_err(|e| PdfError::ExtractionError(e.to_string()))?;

        let trimmed = text.trim();
        if trimmed.is_empty() {
            warn!("PDF extracted empty text: {}", path_str);
            return Ok(String::new());
        }

        info!("Extracted {} chars from PDF", trimmed.len());
        Ok(trimmed.to_string())
    }
}

impl Default for PdfExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_extractor_creation() {
        let extractor = PdfExtractor::new();
        assert!(extractor
            .extract_text(Path::new("nonexistent.pdf"))
            .is_err());
    }

    #[test]
    fn test_quality_calculation() {
        let quality = ExtractionQuality::calculate("This is a test document with some content. It has enough characters to pass the quality threshold and demonstrate that the extraction quality algorithm works correctly for normal documents with plenty of text to analyze.", 1);
        assert!(quality.confidence >= 0.7);
        assert!(!quality.is_scanned);
    }

    #[test]
    fn test_quality_scanned() {
        let quality = ExtractionQuality::calculate("", 1);
        assert!(quality.is_scanned);
        assert!(quality.confidence < 0.5);
    }
}
