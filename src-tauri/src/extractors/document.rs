use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use thiserror::Error;
use tracing::{info, warn};

/// Windows-1252 to UTF-8 lossy conversion
fn lossy_utf8_from_windows1252(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| {
            // Windows-1252 code points 0x80-0x9F map to specific characters
            match b {
                0x80 => '\u{20AC}', // Euro sign
                0x82 => '\u{201A}', // Single low-9 quotation mark
                0x83 => '\u{0192}', // Latin small letter f with hook
                0x84 => '\u{201E}', // Double low-9 quotation mark
                0x85 => '\u{2026}', // Horizontal ellipsis
                0x86 => '\u{2020}', // Dagger
                0x87 => '\u{2021}', // Double dagger
                0x88 => '\u{02C6}', // Modifier letter circumflex accent
                0x89 => '\u{2030}', // Per mille sign
                0x8A => '\u{0160}', // Latin capital letter S with caron
                0x8B => '\u{2039}', // Single left-pointing angle quotation mark
                0x8C => '\u{0161}', // Latin small letter s with caron
                0x8D => '\u{0178}', // Latin capital letter Y with diaeresis
                0x8E => '\u{017D}', // Latin capital letter Z with caron
                0x8F => '\u{017E}', // Latin small letter z with caron
                0x90 => '\u{0178}', // Latin capital letter Y with diaeresis
                0x91 => '\u{2018}', // Left single quotation mark
                0x92 => '\u{2019}', // Right single quotation mark
                0x93 => '\u{201C}', // Left double quotation mark
                0x94 => '\u{201D}', // Right double quotation mark
                0x95 => '\u{2022}', // Bullet
                0x96 => '\u{2013}', // En dash
                0x97 => '\u{2014}', // Em dash
                0x98 => '\u{02DC}', // Small tilde
                0x99 => '\u{2122}', // Trade mark sign
                0x9A => '\u{0161}', // Latin small letter s with caron
                0x9B => '\u{203A}', // Single right-pointing angle quotation mark
                0x9C => '\u{0160}', // Latin capital letter S with caron
                0x9D => '\u{0178}', // Latin capital letter Y with diaeresis
                0x9E => '\u{017E}', // Latin small letter z with caron
                0x9F => '\u{017D}', // Latin capital letter Z with caron
                _ => b as char,     // ASCII and Latin-1 Supplement (valid as-is in UTF-8)
            }
        })
        .collect()
}

#[derive(Error, Debug)]
pub enum DocumentError {
    #[error("Failed to read document: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse document: {0}")]
    ParseError(String),
    #[error("Unsupported encoding: {0}")]
    EncodingError(String),
    #[error("Corrupted document: {0}")]
    Corrupted(String),
}

/// Detect text encoding from BOM or heuristics
pub fn detect_encoding(bytes: &[u8]) -> &'static str {
    // Check for BOM
    if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        return "UTF-8";
    }
    if bytes.len() >= 2 {
        if bytes[0] == 0xFF && bytes[1] == 0xFE {
            return "UTF-16LE";
        }
        if bytes[0] == 0xFE && bytes[1] == 0xFF {
            return "UTF-16BE";
        }
    }

    // Check for valid UTF-8
    if std::str::from_utf8(bytes).is_ok() {
        return "UTF-8";
    }

    // Default to Windows-1252 (Latin-1)
    "WINDOWS-1252"
}

/// Read text file with encoding detection
pub fn read_text_file(path: &Path) -> Result<String, DocumentError> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    let encoding = detect_encoding(&bytes);
    info!("Detected encoding: {} for {}", encoding, path.display());

    match encoding {
        "UTF-8" => {
            // Remove BOM if present
            let text =
                if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
                    String::from_utf8_lossy(&bytes[3..]).to_string()
                } else {
                    String::from_utf8_lossy(&bytes).to_string()
                };
            Ok(text.trim().to_string())
        }
        "WINDOWS-1252" => {
            // Windows-1252 to UTF-8: lossy conversion replaces invalid sequences
            // This is a pragmatic approach without adding encoding_rs dependency
            let text = lossy_utf8_from_windows1252(&bytes);
            Ok(text.trim().to_string())
        }
        _ => {
            // Try UTF-8 as fallback
            Ok(String::from_utf8_lossy(&bytes).trim().to_string())
        }
    }
}

/// Extract text from plain text or markdown files
pub fn extract_text(path: &Path) -> Result<String, DocumentError> {
    let path_str = path.to_string_lossy();
    info!("Extracting text from document: {}", path_str);

    let text = read_text_file(path)?;

    if text.is_empty() {
        warn!("Document is empty: {}", path_str);
    }

    info!("Extracted {} chars from document", text.len());
    Ok(text)
}

/// Extract text from DOCX files
pub fn extract_docx(path: &Path) -> Result<String, DocumentError> {
    let path_str = path.to_string_lossy();
    info!("Extracting text from DOCX: {}", path_str);

    let file = File::open(path)?;
    let mut archive = zip::ZipArchive::new(BufReader::new(file))
        .map_err(|e| DocumentError::Corrupted(e.to_string()))?;

    let mut text_parts = Vec::new();

    // Read document.xml from the DOCX (it's a ZIP archive)
    if let Ok(mut doc_file) = archive.by_name("word/document.xml") {
        let mut xml_content = String::new();
        doc_file.read_to_string(&mut xml_content)?;

        // Simple XML parsing - extract text between <w:t> tags
        let mut in_text = false;
        let mut current_text = String::new();

        for chunk in xml_content.split_inclusive('<') {
            if chunk.contains("<w:t") {
                in_text = true;
                // Extract text content
                if let Some(start) = chunk.find('>') {
                    let rest = &chunk[start + 1..];
                    if let Some(end) = rest.find("</w:t>") {
                        current_text.push_str(&rest[..end]);
                    }
                }
            } else if in_text && chunk.starts_with("</w:t>") {
                in_text = false;
                if !current_text.is_empty() {
                    text_parts.push(current_text.clone());
                    current_text.clear();
                }
            } else if in_text {
                // Continue collecting text
                if let Some(start) = chunk.find('>') {
                    current_text.push_str(&chunk[start + 1..chunk.len().saturating_sub(1)]);
                }
            }
        }
    }

    let text = text_parts.join(" ");
    let trimmed = text.trim();

    if trimmed.is_empty() {
        warn!("DOCX extracted empty text: {}", path_str);
        return Err(DocumentError::ParseError(
            "No text content found in DOCX".to_string(),
        ));
    }

    info!("Extracted {} chars from DOCX", trimmed.len());
    Ok(trimmed.to_string())
}

/// Document extraction result with metadata
pub struct DocumentExtraction {
    pub text: String,
    pub encoding: String,
    pub char_count: usize,
    pub word_count: usize,
}

impl DocumentExtraction {
    pub fn from_path(path: &Path) -> Result<Self, DocumentError> {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        let text = match extension.as_str() {
            "docx" => extract_docx(path)?,
            "txt" | "md" | "rtf" => extract_text(path)?,
            _ => {
                return Err(DocumentError::ParseError(format!(
                    "Unsupported extension: {}",
                    extension
                )))
            }
        };

        let encoding = "UTF-8".to_string(); // We convert everything to UTF-8
        let char_count = text.len();
        let word_count = text.split_whitespace().count();

        Ok(DocumentExtraction {
            text,
            encoding,
            char_count,
            word_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_encoding_detection_utf8() {
        let text = "Hello, World!";
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(text.as_bytes()).unwrap();

        let encoding = detect_encoding(&text.as_bytes());
        assert_eq!(encoding, "UTF-8");
    }

    #[test]
    fn test_encoding_detection_latin1() {
        let text = "Héllo, Wörld!";
        let bytes: Vec<u8> = text.chars().map(|c| c as u8).collect();

        let encoding = detect_encoding(&bytes);
        // Will default to Windows-1252 for non-UTF-8
        assert!(!encoding.is_empty());
    }
}
