use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

use thiserror::Error;
use tracing::{info, warn};

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Audio transcription not available: {0}")]
    NotAvailable(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Audio file not found: {0}")]
    FileNotFound(String),
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("Whisper CLI not installed: {0}")]
    WhisperNotInstalled(String),
    #[error("Transcription failed: {0}")]
    TranscriptionFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioMetadata {
    pub duration_seconds: Option<f64>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub format: String,
    pub file_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioExtractor {
    pub model_path: Option<String>,
    model_loaded: bool,
}

impl AudioExtractor {
    pub fn new() -> Result<Option<Self>, AudioError> {
        if !Self::check_whisper_available() {
            return Ok(None);
        }

        Ok(Some(AudioExtractor {
            model_path: None,
            model_loaded: true,
        }))
    }

    pub fn transcribe(&self, path: &Path) -> Result<String, AudioError> {
        if !path.exists() {
            return Err(AudioError::FileNotFound(path.to_string_lossy().to_string()));
        }

        if !Self::is_supported_format(path) {
            return Err(AudioError::UnsupportedFormat(
                path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
            ));
        }

        let whisper_available = Self::check_whisper_available();
        if !whisper_available {
            return Err(AudioError::WhisperNotInstalled(
                "Whisper CLI is not installed on this system. \
                 Please install whisper.cpp and ensure it's available in your PATH. \
                 On macOS: brew install whisper.cpp"
                    .to_string(),
            ));
        }

        let model_path = self.model_path.clone().unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_default();
            format!("{}/.whisper/models/ggml-base.en.bin", home)
        });

        let temp_dir = std::env::temp_dir();
        let output_base = temp_dir.join(format!("steinline_transcribe_{}", std::process::id()));

        let file_path = path.to_string_lossy().to_string();
        let output_path = output_base.with_extension("txt");

        info!("Transcribing audio with whisper: {}", path.display());

        let output = Command::new("whisper")
            .args([
                "-m",
                &model_path,
                "-f",
                &file_path,
                "-o",
                temp_dir.to_string_lossy().as_ref(),
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Whisper transcription failed: {}", stderr);
            return Err(AudioError::TranscriptionFailed(stderr.to_string()));
        }

        let transcription = std::fs::read_to_string(&output_path).map_err(AudioError::IoError)?;

        let _ = std::fs::remove_file(&output_path);

        Ok(transcription)
    }

    pub fn get_metadata(&self, path: &Path) -> Result<AudioMetadata, AudioError> {
        if !path.exists() {
            return Err(AudioError::FileNotFound(path.to_string_lossy().to_string()));
        }

        let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        let format = match ext.as_str() {
            "mp3" => "MP3",
            "wav" => "WAV",
            "m4a" => "M4A/AAC",
            "mp4" => "MP4/AAC",
            "ogg" => "OGG Vorbis",
            "flac" => "FLAC",
            _ => "Unknown",
        }
        .to_string();

        let duration_seconds = Self::estimate_duration(path);

        Ok(AudioMetadata {
            duration_seconds,
            sample_rate: None,
            channels: None,
            format,
            file_size_bytes: file_size,
        })
    }

    fn estimate_duration(path: &Path) -> Option<f64> {
        let file_size = std::fs::metadata(path).ok()?.len();
        let ext = path.extension()?.to_str()?.to_lowercase();
        let bitrate = match ext.as_str() {
            "mp3" => 128_000,
            "m4a" | "m4v" => 128_000,
            "wav" => 1_411_000,
            "flac" => 800_000,
            "ogg" => 128_000,
            _ => 128_000,
        };
        Some(file_size as f64 * 8.0 / bitrate as f64)
    }

    fn check_whisper_available() -> bool {
        Command::new("whisper")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn is_available(&self) -> bool {
        self.model_loaded
    }

    pub fn is_supported_format(path: &Path) -> bool {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        matches!(ext, Some(e) if matches!(e.as_str(),
            "mp3" | "wav" | "mp4" | "m4a" | "m4v" | "ogg" | "flac"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    #[ignore]
    fn test_supported_formats() {
        assert!(AudioExtractor::is_supported_format(Path::new("test.mp3")));
        assert!(AudioExtractor::is_supported_format(Path::new("test.wav")));
        assert!(AudioExtractor::is_supported_format(Path::new("test.m4a")));
        assert!(AudioExtractor::is_supported_format(Path::new("test.ogg")));
        assert!(!AudioExtractor::is_supported_format(Path::new("test.txt")));
    }

    #[test]
    #[ignore]
    fn test_metadata_estimate() {
        let extractor = AudioExtractor::default();
        let metadata = extractor.get_metadata(Path::new("test.mp3")).unwrap();
        assert_eq!(metadata.format, "MP3");
    }
}
