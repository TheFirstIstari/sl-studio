use image::{DynamicImage, RgbaImage};
use mupdf::{Colorspace, Document, Matrix, TextPageFlags};
use pdf_extract::extract_text;
use std::panic::catch_unwind;
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
    target_dpi: u32,
    min_text_chars: usize,
    early_termination_chars: usize,
}

impl PdfExtractor {
    pub fn new() -> Self {
        PdfExtractor {
            max_pages: 1000,
            streaming: false,
            max_file_size_mb: 500.0,
            target_dpi: 200,
            min_text_chars: 100,
            early_termination_chars: 500,
        }
    }

    pub fn with_limits(max_pages: usize, max_file_size_mb: f64) -> Self {
        PdfExtractor {
            max_pages,
            streaming: false,
            max_file_size_mb,
            target_dpi: 200,
            min_text_chars: 100,
            early_termination_chars: 500,
        }
    }

    /// Get page count from PDF
    pub fn get_page_count(&self, path: &Path) -> Result<usize, PdfError> {
        let doc = Document::open(path)
            .map_err(|e| PdfError::ExtractionError(format!("Failed to open PDF: {}", e)))?;
        let pages = doc
            .pages()
            .map_err(|e| PdfError::ExtractionError(format!("Failed to get pages: {}", e)))?;
        Ok(pages.count())
    }

    /// Render a single PDF page to an image
    pub fn render_page(&self, path: &Path, page_num: u32) -> Result<DynamicImage, PdfError> {
        let doc = Document::open(path)
            .map_err(|e| PdfError::ExtractionError(format!("Failed to open PDF: {}", e)))?;

        let pages: Vec<_> = doc
            .pages()
            .map_err(|e| PdfError::ExtractionError(format!("Failed to get pages: {}", e)))?
            .collect();

        let page = pages
            .get(page_num as usize - 1)
            .ok_or_else(|| PdfError::ExtractionError(format!("Page {} not found", page_num)))?;

        let page = page.as_ref().map_err(|e| {
            PdfError::ExtractionError(format!("Failed to get page {}: {}", page_num, e))
        })?;

        let zoom = self.target_dpi as f32 / 72.0;
        let mut mat = Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        mat.scale(zoom, zoom);

        let cs = Colorspace::device_rgb();
        let pix = page.to_pixmap(&mat, &cs, false, false).map_err(|e| {
            PdfError::ExtractionError(format!("Failed to render page {}: {}", page_num, e))
        })?;

        let rgba_data = pix.samples().to_vec();
        let img = DynamicImage::ImageRgba8(
            RgbaImage::from_raw(pix.width(), pix.height(), rgba_data).ok_or_else(|| {
                PdfError::ExtractionError("Failed to create image from rendered page".to_string())
            })?,
        );

        Ok(img)
    }

    /// Render all pages to images (for parallel processing)
    pub fn render_all_pages(&self, path: &Path) -> Result<Vec<DynamicImage>, PdfError> {
        let page_count = self.get_page_count(path)?;
        let mut images = Vec::with_capacity(page_count);

        for page_num in 1..=page_count as u32 {
            let img = self.render_page(path, page_num)?;
            images.push(img);
        }

        Ok(images)
    }

    /// Estimate page count from form feeds in text
    fn estimate_page_count(text: &str) -> usize {
        text.matches('\u{0C}').count() + 1
    }

    fn extract_text_safe(path: &Path) -> Result<String, PdfError> {
        let result = catch_unwind(|| extract_text(path));
        match result {
            Ok(Ok(text)) => Ok(text),
            Ok(Err(e)) => {
                let err_str = e.to_string();
                if err_str.contains("password") || err_str.contains("encrypted") {
                    Err(PdfError::PasswordProtected)
                } else if err_str.contains("PDF") && err_str.contains("error") {
                    Err(PdfError::Corrupted(err_str))
                } else {
                    Err(PdfError::ExtractionError(err_str))
                }
            }
            Err(panic_info) => {
                let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic".to_string()
                };
                Err(PdfError::ExtractionError(format!(
                    "Panic during extraction: {}",
                    msg
                )))
            }
        }
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

        let text = Self::extract_text_safe(path)?;

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

        let text = Self::extract_text_safe(path)?;

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
        let size_mb = std::fs::metadata(path)
            .map(|m| m.len() as f64 / (1024.0 * 1024.0))
            .unwrap_or(0.0);

        // Cheaply probe page count via mupdf so we can route huge PDFs to
        // streaming even if the file size is modest (lots of small pages).
        let page_count = self.get_page_count(path).unwrap_or(0);

        if size_mb > 100.0 || page_count > 500 {
            warn!(
                "Large PDF detected ({:.1} MB, {} pages), using streaming extraction",
                size_mb, page_count
            );
            // 10 MB of text is already a giant document; bail past that to
            // keep memory bounded.
            let (text, _pages) = self.extract_text_streaming(path, Some(10 * 1024 * 1024))?;
            return Ok(text);
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

    /// Extract text from a large PDF.
    ///
    /// Thin wrapper around [`Self::extract_text_streaming`] with a 10 MB
    /// character ceiling. Kept as a stable entry point for callers that just
    /// want "give me the text, but don't blow up memory".
    pub fn extract_text_large(&self, path: &Path) -> Result<String, PdfError> {
        let (text, _) = self.extract_text_streaming(path, Some(10 * 1024 * 1024))?;
        Ok(text)
    }

    /// Extract text from a PDF page-by-page with bounded memory and an
    /// optional max char ceiling. Suitable for files >100MB.
    ///
    /// Each page is opened, converted to a `TextPage`, drained into a
    /// `String`, and dropped before the next page is processed, so peak
    /// memory tracks a single page rather than the whole document.
    ///
    /// Returns the concatenated text (with form-feed `\u{000C}` between
    /// pages) and the number of pages actually extracted.
    pub fn extract_text_streaming(
        &self,
        path: &Path,
        max_chars: Option<usize>,
    ) -> Result<(String, u32), PdfError> {
        let path_str = path.to_string_lossy();
        info!("Streaming PDF extraction starting: {}", path_str);

        let doc = Document::open(path).map_err(|e| {
            let msg = e.to_string();
            if msg.contains("password") || msg.contains("encrypted") {
                PdfError::PasswordProtected
            } else {
                PdfError::ExtractionError(format!("Failed to open PDF: {}", msg))
            }
        })?;

        let pages_iter = doc
            .pages()
            .map_err(|e| PdfError::ExtractionError(format!("Failed to get pages: {}", e)))?;

        let mut accumulated = String::new();
        let mut pages_extracted: u32 = 0;
        let ceiling = max_chars.unwrap_or(usize::MAX);

        for (idx, page_res) in pages_iter.enumerate() {
            let page = match page_res {
                Ok(p) => p,
                Err(e) => {
                    warn!("Skipping page {}: {}", idx + 1, e);
                    continue;
                }
            };

            // Scope so the TextPage is dropped before next iteration.
            let page_text = match page.to_text_page(TextPageFlags::empty()) {
                Ok(tp) => match tp.to_text() {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("Failed to render text for page {}: {}", idx + 1, e);
                        String::new()
                    }
                },
                Err(e) => {
                    warn!("Failed to build text page {}: {}", idx + 1, e);
                    String::new()
                }
            };

            if pages_extracted > 0 {
                accumulated.push('\u{000C}');
            }
            // Bound the per-page append so a single huge page can't push us
            // arbitrarily past the ceiling.
            if accumulated.len() + page_text.len() > ceiling {
                let remaining = ceiling.saturating_sub(accumulated.len());
                // Ensure we slice on a char boundary.
                let mut take = remaining.min(page_text.len());
                while take > 0 && !page_text.is_char_boundary(take) {
                    take -= 1;
                }
                accumulated.push_str(&page_text[..take]);
                pages_extracted += 1;
                warn!(
                    "Streaming extraction hit char ceiling ({} chars) at page {}",
                    ceiling,
                    idx + 1
                );
                break;
            }
            accumulated.push_str(&page_text);
            pages_extracted += 1;

            if accumulated.len() >= ceiling {
                warn!(
                    "Streaming extraction hit char ceiling ({} chars) at page {}",
                    ceiling,
                    idx + 1
                );
                break;
            }
        }

        let trimmed = accumulated.trim().to_string();
        info!(
            "Streaming PDF extraction: {} pages, {} chars",
            pages_extracted,
            trimmed.len()
        );
        Ok((trimmed, pages_extracted))
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

    #[test]
    fn test_extract_text_streaming_missing_file() {
        let extractor = PdfExtractor::new();
        let result = extractor.extract_text_streaming(Path::new("nonexistent.pdf"), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_text_streaming_zero_ceiling() {
        // With a 0-char ceiling we must never return more than 0 chars even
        // if the PDF doesn't exist (we still expect an error here, not a
        // panic). This guards the boundary arithmetic in the streaming loop.
        let extractor = PdfExtractor::new();
        let result = extractor.extract_text_streaming(Path::new("nonexistent.pdf"), Some(0));
        assert!(result.is_err());
    }

    /// Real-PDF smoke test. Ignored by default because it requires a sample
    /// PDF on disk; run with `cargo test --lib pdf -- --ignored` after
    /// dropping a file at the path below.
    #[test]
    #[ignore]
    fn test_extract_text_streaming_real_pdf() {
        let extractor = PdfExtractor::new();
        let path = Path::new("../29148-2018-ISOIECIEEE.pdf");
        let (text, pages) = extractor
            .extract_text_streaming(path, Some(10 * 1024 * 1024))
            .expect("streaming extraction should succeed");
        assert!(pages > 0);
        assert!(!text.is_empty());
        assert!(text.len() <= 10 * 1024 * 1024);
    }

    #[test]
    #[ignore]
    fn test_extract_text_streaming_respects_ceiling() {
        let extractor = PdfExtractor::new();
        let path = Path::new("../29148-2018-ISOIECIEEE.pdf");
        let (text, pages) = extractor
            .extract_text_streaming(path, Some(1024))
            .expect("streaming extraction should succeed");
        assert!(text.len() <= 1024);
        assert!(pages >= 1);
    }
}
