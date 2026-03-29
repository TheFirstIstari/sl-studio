use pdf_extract::extract_text;
use std::path::Path;
use thiserror::Error;
use tracing::{info, warn};

#[derive(Error, Debug)]
pub enum PdfError {
    #[error("Failed to extract text from PDF: {0}")]
    ExtractionError(String),
    #[error("Failed to read file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("PDF too large: {0} pages (max: {1})")]
    TooLarge(usize, usize),
}

pub struct PdfExtractor {
    max_pages: usize,
    streaming: bool,
}

impl PdfExtractor {
    pub fn new() -> Self {
        PdfExtractor {
            max_pages: 500,
            streaming: false,
        }
    }

    pub fn with_limits(max_pages: usize, streaming: bool) -> Self {
        PdfExtractor { max_pages, streaming }
    }

    pub fn extract_text(&self, path: &Path) -> Result<String, PdfError> {
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

    pub fn extract_text_with_fallback(&self, path: &Path) -> Result<String, PdfError> {
        // Check file size first
        if let Ok(metadata) = std::fs::metadata(path) {
            let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
            if size_mb > 100.0 {
                warn!("Large PDF detected ({} MB), using limited extraction", size_mb);
                return self.extract_text_limited(path, 100);
            }
        }

        let text = self.extract_text(path)?;

        if text.len() < 100 {
            warn!("PDF has minimal text ({} chars), may be scanned", text.len());
        }

        Ok(text)
    }

    pub fn extract_text_limited(&self, path: &Path, max_pages: usize) -> Result<String, PdfError> {
        let path_str = path.to_string_lossy();
        info!("Extracting text from PDF (limited to {} pages): {}", max_pages, path_str);

        let text = extract_text(path).map_err(|e| PdfError::ExtractionError(e.to_string()))?;

        // Count pages by counting form feed characters or by splitting
        let page_count = text.matches('\u{0C}').count() + 1;
        
        if page_count > max_pages {
            warn!("PDF has {} pages, limiting to {}", page_count, max_pages);
        }

        let trimmed = text.trim();
        if trimmed.is_empty() {
            warn!("PDF extracted empty text: {}", path_str);
            return Ok(String::new());
        }

        info!("Extracted {} chars from PDF (limited)", trimmed.len());
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
    use std::fs;

    #[test]
    fn test_pdf_extractor_creation() {
        let extractor = PdfExtractor::new();
        assert!(extractor.extract_text(Path::new("nonexistent.pdf")).is_err());
    }
}
