use std::path::Path;
use thiserror::Error;
use serde::{Deserialize, Serialize};

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Audio transcription not available: {0}")]
    NotAvailable(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Audio file not found: {0}")]
    FileNotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioExtractor {
    pub model_path: Option<String>,
}

impl AudioExtractor {
    pub fn new(_model_path: &Path) -> Result<Self, AudioError> {
        Err(AudioError::NotAvailable("Whisper not built (cmake required)".to_string()))
    }

    pub fn transcribe(&self, _path: &Path) -> Result<String, AudioError> {
        Err(AudioError::NotAvailable("Whisper not built (cmake required)".to_string()))
    }

    pub fn is_available(&self) -> bool {
        self.model_path.is_some()
    }

    pub fn is_supported_format(path: &Path) -> bool {
        let ext = path.extension()
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
    fn test_supported_formats() {
        assert!(AudioExtractor::is_supported_format(Path::new("test.mp3")));
        assert!(AudioExtractor::is_supported_format(Path::new("test.wav")));
        assert!(AudioExtractor::is_supported_format(Path::new("test.m4a")));
        assert!(!AudioExtractor::is_supported_format(Path::new("test.txt")));
    }
}
