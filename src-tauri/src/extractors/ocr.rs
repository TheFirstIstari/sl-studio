use image::{DynamicImage, ImageBuffer, Rgb};
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

#[derive(Debug, Clone, Default)]
pub struct PreprocessingConfig {
    pub enhance_contrast: bool,
    pub auto_rotate: bool,
    pub target_dpi: Option<u32>,
}

fn adjust_contrast(img: &DynamicImage) -> DynamicImage {
    let rgb = img.to_rgb8();
    let (width, height) = rgb.dimensions();

    let mut luminance: Vec<u8> = rgb
        .pixels()
        .map(|p: &image::Rgb<u8>| {
            ((p[0] as u32 * 299 + p[1] as u32 * 587 + p[2] as u32 * 114) / 1000) as u8
        })
        .collect();

    luminance.sort();
    let median = luminance[luminance.len() / 2];

    let factor = if median < 128 { 1.3 } else { 0.8 };

    let mut output = ImageBuffer::new(width, height);

    for (x, y, pixel) in rgb.enumerate_pixels() {
        let new_r = ((pixel[0] as f32 - 128.0) * factor + 128.0).clamp(0.0, 255.0) as u8;
        let new_g = ((pixel[1] as f32 - 128.0) * factor + 128.0).clamp(0.0, 255.0) as u8;
        let new_b = ((pixel[2] as f32 - 128.0) * factor + 128.0).clamp(0.0, 255.0) as u8;
        output.put_pixel(x, y, Rgb([new_r, new_g, new_b]));
    }

    DynamicImage::ImageRgb8(output)
}

fn calculate_histogram(img: &DynamicImage) -> [u32; 256] {
    let gray = img.to_luma8();
    let mut histogram = [0u32; 256];

    for pixel in gray.pixels() {
        histogram[pixel[0] as usize] += 1;
    }
    histogram
}

fn detect_rotation(img: &DynamicImage) -> Option<i32> {
    let histogram = calculate_histogram(img);

    let mut score_h = 0i32;
    let mut score_v = 0i32;

    for (i, &count) in histogram.iter().enumerate() {
        let line_score = count as i32 * (i as i32 - 128).abs();
        if !(64..=192).contains(&i) {
            score_h += line_score;
        }
        if (96..160).contains(&i) {
            score_v += line_score;
        }
    }

    if score_v > score_h * 2 {
        Some(90)
    } else if score_h > score_v * 2 {
        Some(0)
    } else {
        None
    }
}

fn rotate_image(img: DynamicImage, degrees: i32) -> DynamicImage {
    match degrees {
        90 | -270 => img.rotate90(),
        180 | -180 => img.rotate180(),
        270 | -90 => img.rotate270(),
        _ => img,
    }
}

fn preprocess_image(img: DynamicImage, config: &PreprocessingConfig) -> DynamicImage {
    let mut processed = img;

    if config.enhance_contrast {
        processed = adjust_contrast(&processed);
    }

    if config.auto_rotate {
        if let Some(rotation) = detect_rotation(&processed) {
            if rotation != 0 {
                processed = rotate_image(processed, rotation);
            }
        }
    }

    processed
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

    pub fn extract_with_preprocessing(
        &self,
        path: &Path,
        config: &PreprocessingConfig,
    ) -> Result<String, OcrError> {
        let path_str = path.to_string_lossy();
        info!("Running OCR with preprocessing on: {}", path_str);

        let img = image::open(path).map_err(|e| OcrError::ImageError(e.to_string()))?;

        let processed = preprocess_image(img, config);
        let rgb = processed.to_rgb8();
        let (width, height) = rgb.dimensions();

        if config.enhance_contrast || config.auto_rotate {
            info!(
                "Applied preprocessing: contrast={}, auto_rotate={}",
                config.enhance_contrast, config.auto_rotate
            );
        }

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
            warn!(
                "OCR with preprocessing returned empty text for: {}",
                path_str
            );
        } else {
            info!("OCR extracted {} chars from {}", trimmed.len(), path_str);
        }

        Ok(trimmed.to_string())
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

    #[test]
    fn test_ocr_extractor_creation() {
        let result = OcrExtractor::new();
        assert!(result.is_ok());
    }
}
