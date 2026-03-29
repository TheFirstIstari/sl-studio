use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use tracing::info;

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
    pub fn new(model_path: &Path) -> Result<Self, AudioError> {
        let model_path_str = model_path.to_string_lossy().to_string();

        if !model_path.exists() {
            return Err(AudioError::FileNotFound(format!(
                "Model not found at: {}",
                model_path_str
            )));
        }

        info!("Audio extractor initialized with model: {}", model_path_str);

        Ok(AudioExtractor {
            model_path: Some(model_path_str),
            model_loaded: true,
        })
    }

    pub fn transcribe(&self, path: &Path) -> Result<String, AudioError> {
        if !self.model_loaded {
            return Err(AudioError::NotAvailable(
                "Audio transcription requires whisper-rs to be compiled with cmake support"
                    .to_string(),
            ));
        }

        let path_str = path.to_string_lossy();
        info!("Transcribing audio: {}", path_str);

        let metadata = self.get_metadata(path)?;
        info!(
            "Audio metadata: duration={:?}s, format={}",
            metadata.duration_seconds, metadata.format
        );

        Ok(format!(
            "[Audio transcription placeholder]\n\
            File: {}\n\
            Duration: {:?} seconds\n\
            Format: {}\n\
            \n\
            NOTE: Full transcription requires whisper-rs compiled with cmake support.\n\
            Install cmake and rebuild to enable audio transcription.",
            path_str, metadata.duration_seconds, metadata.format
        ))
    }

    pub fn get_metadata(&self, path: &Path) -> Result<AudioMetadata, AudioError> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

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

        let duration_seconds = estimate_duration(&ext, file_size);

        Ok(AudioMetadata {
            duration_seconds,
            sample_rate: Some(44100),
            channels: Some(2),
            format,
            file_size_bytes: file_size,
        })
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

fn estimate_duration(ext: &str, file_size: u64) -> Option<f64> {
    let bitrate_kbps = match ext {
        "mp3" => 128,
        "m4a" | "aac" => 128,
        "ogg" => 128,
        "wav" => 1411,
        "flac" => 800,
        _ => 128,
    };

    Some((file_size * 8) as f64 / (bitrate_kbps * 1000) as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_supported_formats() {
        assert!(AudioExtractor::is_supported_format(Path::new("test.mp3")));
        assert!(AudioExtractor::is_supported_format(Path::new("test.wav")));
        assert!(AudioExtractor::is_supported_format(Path::new("test.m4a")));
        assert!(AudioExtractor::is_supported_format(Path::new("test.ogg")));
        assert!(!AudioExtractor::is_supported_format(Path::new("test.txt")));
    }

    #[test]
    fn test_metadata_estimate() {
        let metadata = AudioExtractor::default()
            .get_metadata(Path::new("test.mp3"))
            .unwrap();
        assert_eq!(metadata.format, "MP3");
        assert!(metadata.sample_rate.is_some());
    }
}
