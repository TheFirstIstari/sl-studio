//! FR-META: extract document metadata.
//!
//! - **Images** (jpeg/png/tiff/heic/webp): EXIF tags (camera model,
//!   capture date, GPS, software, original dimensions, etc.) via the
//!   `kamadak-exif` crate.
//! - **PDFs**: Document Information Dictionary (Title, Author, Subject,
//!   Keywords, Creator, Producer, CreationDate, ModDate) via `lopdf`.
//! - **Audio** (mp3/wav/m4a/ogg/flac): duration, sample rate, channels,
//!   and codec via symphonia (see `extractors::audio`).
//!
//! Returns a `DocumentMetadata` struct with both raw key/value pairs
//! and a small set of normalized fields (author, created_at, etc.) so
//! the UI doesn't have to interpret tag-name dialects.

use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, warn};

use crate::extractors::audio::AudioExtractor;

#[derive(Error, Debug)]
pub enum MetadataError {
    #[error("file not found: {0}")]
    FileNotFound(String),
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("exif parse error: {0}")]
    Exif(String),
    #[error("pdf parse error: {0}")]
    Pdf(String),
    #[error("audio metadata error: {0}")]
    Audio(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentMetadata {
    /// Type of metadata: "exif", "pdf", "audio", or "none".
    pub source: String,
    /// Normalized fields (best-effort across formats).
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub keywords: Option<String>,
    pub camera_model: Option<String>,
    pub gps_latitude: Option<f64>,
    pub gps_longitude: Option<f64>,
    // ── Audio-specific fields (populated when source == "audio") ────────────
    /// Duration in seconds.
    pub audio_duration_seconds: Option<f64>,
    /// Native sample rate in Hz (e.g. 44100, 48000).
    pub audio_sample_rate: Option<u32>,
    /// Number of audio channels (1 = mono, 2 = stereo).
    pub audio_channels: Option<u32>,
    /// Human-readable container format (e.g. "MP3", "FLAC", "WAV").
    pub audio_format: Option<String>,
    /// Codec identifier as reported by symphonia (e.g. "mp3", "aac").
    pub audio_codec: Option<String>,
    /// Bit-depth, if available (e.g. 16, 24).
    pub audio_bits_per_sample: Option<u32>,
    /// All raw key/value pairs as detected. The keys are
    /// implementation-specific (e.g. `Image.Make`, `pdf:Author`).
    pub raw: std::collections::BTreeMap<String, String>,
}

/// Dispatch to the right metadata reader based on file extension.
///
/// For unsupported extensions returns a `DocumentMetadata { source: "none", .. }`
/// rather than an error — a forensic pipeline shouldn't choke on plain
/// text files that simply have no metadata to surface.
pub fn extract_metadata(path: &Path) -> Result<DocumentMetadata, MetadataError> {
    if !path.exists() {
        return Err(MetadataError::FileNotFound(
            path.to_string_lossy().to_string(),
        ));
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "jpg" | "jpeg" | "tiff" | "tif" | "heic" | "heif" | "webp" | "png" => {
            extract_image_metadata(path)
        }
        "pdf" => extract_pdf_metadata(path),
        "mp3" | "wav" | "m4a" | "m4v" | "mp4" | "ogg" | "flac" => {
            extract_audio_metadata(path)
        }
        _ => Ok(DocumentMetadata {
            source: "none".to_string(),
            ..Default::default()
        }),
    }
}

/// Extract EXIF tags from an image.
pub fn extract_image_metadata(path: &Path) -> Result<DocumentMetadata, MetadataError> {
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(&file);
    let exif = exif::Reader::new()
        .read_from_container(&mut reader)
        .map_err(|e| MetadataError::Exif(e.to_string()))?;

    let mut meta = DocumentMetadata {
        source: "exif".to_string(),
        ..Default::default()
    };

    let mut camera_make: Option<String> = None;
    let mut camera_model_raw: Option<String> = None;
    let mut gps_lat: Option<f64> = None;
    let mut gps_lat_ref: Option<String> = None;
    let mut gps_lon: Option<f64> = None;
    let mut gps_lon_ref: Option<String> = None;

    for field in exif.fields() {
        let tag = field.tag.to_string();
        let value = field.display_value().with_unit(&exif).to_string();
        // Strip surrounding quotes that exif crate adds for string values.
        let trimmed = value.trim_matches('"').to_string();

        match field.tag {
            exif::Tag::Make => camera_make = Some(trimmed.clone()),
            exif::Tag::Model => camera_model_raw = Some(trimmed.clone()),
            exif::Tag::Software => {
                meta.creator.get_or_insert(trimmed.clone());
            }
            exif::Tag::Artist => {
                meta.author.get_or_insert(trimmed.clone());
            }
            exif::Tag::ImageDescription => {
                meta.subject.get_or_insert(trimmed.clone());
            }
            exif::Tag::DateTimeOriginal | exif::Tag::DateTime => {
                meta.created_at.get_or_insert(trimmed.clone());
            }
            exif::Tag::DateTimeDigitized => {
                meta.modified_at.get_or_insert(trimmed.clone());
            }
            exif::Tag::GPSLatitude => {
                gps_lat = parse_gps_decimal(&trimmed);
            }
            exif::Tag::GPSLatitudeRef => gps_lat_ref = Some(trimmed.clone()),
            exif::Tag::GPSLongitude => {
                gps_lon = parse_gps_decimal(&trimmed);
            }
            exif::Tag::GPSLongitudeRef => gps_lon_ref = Some(trimmed.clone()),
            _ => {}
        }

        meta.raw.insert(tag, trimmed);
    }

    meta.camera_model = match (camera_make, camera_model_raw) {
        (Some(make), Some(model)) => Some(format!("{} {}", make, model)),
        (None, Some(model)) => Some(model),
        (Some(make), None) => Some(make),
        (None, None) => None,
    };

    if let (Some(lat), Some(refc)) = (gps_lat, &gps_lat_ref) {
        meta.gps_latitude = Some(if refc.starts_with('S') { -lat } else { lat });
    }
    if let (Some(lon), Some(refc)) = (gps_lon, &gps_lon_ref) {
        meta.gps_longitude = Some(if refc.starts_with('W') { -lon } else { lon });
    }

    debug!(
        "EXIF extracted: {} raw tags, camera={:?}",
        meta.raw.len(),
        meta.camera_model
    );
    Ok(meta)
}

/// Extract Document Information Dictionary entries from a PDF via lopdf.
pub fn extract_pdf_metadata(path: &Path) -> Result<DocumentMetadata, MetadataError> {
    let doc = lopdf::Document::load(path).map_err(|e| MetadataError::Pdf(e.to_string()))?;

    let mut meta = DocumentMetadata {
        source: "pdf".to_string(),
        ..Default::default()
    };

    if let Ok(info_id) = doc.trailer.get(b"Info") {
        if let Ok(info_ref) = info_id.as_reference() {
            if let Ok(obj) = doc.get_object(info_ref) {
                if let Ok(dict) = obj.as_dict() {
                    for (key, value) in dict.iter() {
                        let key_str = String::from_utf8_lossy(key).to_string();
                        let val_str = pdf_string_value(value).unwrap_or_default();
                        if val_str.is_empty() {
                            continue;
                        }

                        match key_str.as_str() {
                            "Title" => meta.title = Some(val_str.clone()),
                            "Author" => meta.author = Some(val_str.clone()),
                            "Subject" => meta.subject = Some(val_str.clone()),
                            "Creator" => meta.creator = Some(val_str.clone()),
                            "Producer" => meta.producer = Some(val_str.clone()),
                            "Keywords" => meta.keywords = Some(val_str.clone()),
                            "CreationDate" => meta.created_at = Some(normalize_pdf_date(&val_str)),
                            "ModDate" => meta.modified_at = Some(normalize_pdf_date(&val_str)),
                            _ => {}
                        }
                        meta.raw.insert(key_str, val_str);
                    }
                }
            }
        }
    } else {
        debug!("PDF has no Info dictionary");
    }

    Ok(meta)
}

/// Extract audio metadata (duration, sample rate, channels, codec) via symphonia.
pub fn extract_audio_metadata(path: &Path) -> Result<DocumentMetadata, MetadataError> {
    let extractor = AudioExtractor::default();
    let audio = extractor
        .get_metadata(path)
        .map_err(|e| MetadataError::Audio(e.to_string()))?;

    let mut raw = std::collections::BTreeMap::new();
    if let Some(sr) = audio.sample_rate {
        raw.insert("sample_rate".to_string(), format!("{} Hz", sr));
    }
    if let Some(ch) = audio.channels {
        raw.insert(
            "channels".to_string(),
            match ch {
                1 => "Mono".to_string(),
                2 => "Stereo".to_string(),
                n => format!("{} channels", n),
            },
        );
    }
    if let Some(bps) = audio.bits_per_sample {
        raw.insert("bits_per_sample".to_string(), format!("{}-bit", bps));
    }
    if let Some(dur) = audio.duration_seconds {
        let h = (dur / 3600.0) as u64;
        let m = ((dur % 3600.0) / 60.0) as u64;
        let s = (dur % 60.0) as u64;
        raw.insert(
            "duration".to_string(),
            if h > 0 {
                format!("{:02}:{:02}:{:02}", h, m, s)
            } else {
                format!("{:02}:{:02}", m, s)
            },
        );
    }
    raw.insert("format".to_string(), audio.format.clone());
    if !audio.codec.is_empty() {
        raw.insert("codec".to_string(), audio.codec.clone());
    }
    raw.insert(
        "file_size".to_string(),
        format!("{} bytes", audio.file_size_bytes),
    );

    Ok(DocumentMetadata {
        source: "audio".to_string(),
        audio_duration_seconds: audio.duration_seconds,
        audio_sample_rate: audio.sample_rate,
        audio_channels: audio.channels,
        audio_format: Some(audio.format),
        audio_codec: if audio.codec.is_empty() {
            None
        } else {
            Some(audio.codec)
        },
        audio_bits_per_sample: audio.bits_per_sample,
        raw,
        ..Default::default()
    })
}

/// Pull a UTF-8 string out of a lopdf::Object that might be a String or
/// HexString. Returns None for non-string types.
fn pdf_string_value(obj: &lopdf::Object) -> Option<String> {
    use lopdf::Object;
    match obj {
        Object::String(bytes, _format) => {
            // Try UTF-8 first; PDF strings can also be PDFDocEncoding or
            // UTF-16BE (with BOM). Best-effort: prefer UTF-8.
            if let Ok(s) = std::str::from_utf8(bytes) {
                return Some(s.trim().to_string());
            }
            // UTF-16BE with BOM: 0xFEFF.
            if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
                let utf16: Vec<u16> = bytes[2..]
                    .chunks(2)
                    .filter(|c| c.len() == 2)
                    .map(|c| u16::from_be_bytes([c[0], c[1]]))
                    .collect();
                return String::from_utf16(&utf16)
                    .ok()
                    .map(|s| s.trim().to_string());
            }
            // Fallback: lossy UTF-8.
            Some(String::from_utf8_lossy(bytes).trim().to_string())
        }
        _ => None,
    }
}

/// PDF date strings look like `D:20240115093000+02'00'`. Normalize to
/// ISO 8601 ish: `2024-01-15T09:30:00+0200`. Best-effort; if parsing
/// fails return the input unchanged.
fn normalize_pdf_date(input: &str) -> String {
    let s = input.trim_start_matches("D:").trim_end_matches('\'');
    if s.len() < 14 {
        return input.to_string();
    }
    let year = &s[0..4];
    let month = &s[4..6];
    let day = &s[6..8];
    let hour = &s[8..10];
    let minute = &s[10..12];
    let second = &s[12..14];
    let tz = if s.len() > 14 {
        s[14..].replace('\'', "")
    } else {
        String::new()
    };
    format!("{year}-{month}-{day}T{hour}:{minute}:{second}{tz}")
}

/// Parse a decimal-degrees value out of an EXIF GPS tag like
/// `40 deg 26' 46.302"`. Returns None on parse failure.
fn parse_gps_decimal(value: &str) -> Option<f64> {
    // Try simple decimal first.
    if let Ok(d) = value.parse::<f64>() {
        return Some(d);
    }
    // Otherwise try DMS format.
    let cleaned = value.replace(['°', '\'', '"', ',', '/'], " ");
    let nums: Vec<f64> = cleaned
        .split_whitespace()
        .filter_map(|t| t.parse::<f64>().ok())
        .collect();
    match nums.as_slice() {
        [d] => Some(*d),
        [d, m] => Some(d + m / 60.0),
        [d, m, s] => Some(d + m / 60.0 + s / 3600.0),
        // 4 numbers with the last pair being seconds_num/seconds_den
        [d, m, s_num, s_den] if *s_den != 0.0 => Some(d + m / 60.0 + (s_num / s_den) / 3600.0),
        _ => {
            warn!("Could not parse GPS value: {}", value);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_date_normalization() {
        assert_eq!(
            normalize_pdf_date("D:20240115093000+02'00'"),
            "2024-01-15T09:30:00+0200"
        );
        assert_eq!(
            normalize_pdf_date("D:20231225120000Z"),
            "2023-12-25T12:00:00Z"
        );
        // Too short -> returned unchanged.
        assert_eq!(normalize_pdf_date("D:2024"), "D:2024");
    }

    #[test]
    fn gps_decimal_parsing() {
        assert_eq!(parse_gps_decimal("40.4456"), Some(40.4456));
        // 40° 26' 46.302" → 40.44619...
        let v = parse_gps_decimal("40 26 46.302").unwrap();
        assert!((v - 40.4462).abs() < 0.001);
        assert!(parse_gps_decimal("not a coord").is_none());
    }

    #[test]
    fn extract_metadata_missing_file() {
        let r = extract_metadata(std::path::Path::new("/no/such/file.jpg"));
        assert!(matches!(r, Err(MetadataError::FileNotFound(_))));
    }

    #[test]
    fn extract_metadata_unknown_extension_returns_none() {
        // Use this very file as a stand-in for "any file with no metadata
        // extractor" — extension .rs is not supported.
        let path = std::env::current_exe().unwrap();
        // current_exe is OS specific; use a guaranteed-existing path with
        // an unsupported extension instead by writing a temp file.
        let tmp = tempfile::Builder::new().suffix(".log").tempfile().unwrap();
        let r = extract_metadata(tmp.path()).unwrap();
        assert_eq!(r.source, "none");
        // Avoid unused var warning on path.
        let _ = path;
    }
}
