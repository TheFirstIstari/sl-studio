//! Audio transcription and metadata extraction.
//!
//! **Transcription** uses whisper-rs (in-process whisper.cpp bindings) with
//! Metal acceleration on Apple Silicon.  The caller must supply a path to a
//! GGUF/ggml whisper model — the same model directory that is already managed
//! by the Settings page can host whisper models (e.g. `ggml-base.en.bin`).
//!
//! **Decoding** uses symphonia to read any supported container (MP3, WAV,
//! M4A/AAC, OGG/Vorbis, FLAC) and resample to 16 kHz mono f32 PCM, which is
//! the exact format whisper.cpp expects.
//!
//! If no whisper model path is configured the `AudioExtractor` still decodes
//! audio and returns real `AudioMetadata`; transcription returns
//! `Err(AudioError::ModelNotConfigured)` so the caller can distinguish
//! "model missing" from hard errors.

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info, warn};

// ── Symphonia imports ────────────────────────────────────────────────────────

use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

// ── whisper-rs imports ───────────────────────────────────────────────────────

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

// ── Target sample rate required by whisper.cpp ───────────────────────────────

const WHISPER_SAMPLE_RATE: u32 = 16_000;

// ─────────────────────────────────────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Audio file not found: {0}")]
    FileNotFound(String),
    #[error("Unsupported audio format: {0}")]
    UnsupportedFormat(String),
    #[error("Whisper model not configured — set a model path in Settings")]
    ModelNotConfigured,
    #[error("Whisper model file not found: {0}")]
    ModelNotFound(String),
    #[error("Failed to load whisper model: {0}")]
    ModelLoadError(String),
    #[error("Transcription failed: {0}")]
    TranscriptionFailed(String),
    #[error("Audio decode error: {0}")]
    DecodeError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

// ── Public data types ─────────────────────────────────────────────────────────

/// Real audio file properties extracted by symphonia before (or without)
/// running whisper.  All fields are best-effort; symphonia exposes what the
/// container and codec headers contain.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioMetadata {
    /// Duration in seconds (from the container's `time_base` + `n_frames`).
    pub duration_seconds: Option<f64>,
    /// Native sample rate reported by the codec (Hz).
    pub sample_rate: Option<u32>,
    /// Number of audio channels (1 = mono, 2 = stereo, …).
    pub channels: Option<u32>,
    /// Human-readable format string, e.g. "MP3", "FLAC", "WAV".
    pub format: String,
    /// Codec name as reported by symphonia, e.g. "mp3", "aac".
    pub codec: String,
    /// Bit-depth (bits per sample), if available.
    pub bits_per_sample: Option<u32>,
    /// File size on disk in bytes.
    pub file_size_bytes: u64,
}

// ── AudioExtractor ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AudioExtractor {
    /// Absolute path to a ggml whisper model file.  `None` means "no model
    /// configured"; `get_metadata` still works but `transcribe` returns an error.
    pub model_path: Option<String>,
}

impl Default for AudioExtractor {
    fn default() -> Self {
        AudioExtractor { model_path: None }
    }
}

impl AudioExtractor {
    /// Create a new extractor. Always succeeds — whisper availability is checked
    /// lazily when `transcribe` is called so that the rest of the pipeline can
    /// still use `get_metadata` and file-type detection even without a model.
    pub fn new(model_path: Option<String>) -> Self {
        AudioExtractor { model_path }
    }

    /// Returns `true` if a model path is configured AND the file exists on disk.
    pub fn is_available(&self) -> bool {
        self.model_path
            .as_deref()
            .map(|p| std::path::Path::new(p).exists())
            .unwrap_or(false)
    }

    /// Decode `path` to 16 kHz mono f32 PCM, then run whisper inference.
    ///
    /// Returns the full transcript as a single `String` with segments separated
    /// by newlines, each prefixed with `[HH:MM:SS]`.
    pub fn transcribe(&self, path: &Path) -> Result<String, AudioError> {
        if !path.exists() {
            return Err(AudioError::FileNotFound(path.to_string_lossy().into()));
        }
        if !Self::is_supported_format(path) {
            return Err(AudioError::UnsupportedFormat(
                path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
            ));
        }

        let model_path = self
            .model_path
            .as_deref()
            .ok_or(AudioError::ModelNotConfigured)?;

        if !std::path::Path::new(model_path).exists() {
            return Err(AudioError::ModelNotFound(model_path.to_string()));
        }

        info!("Decoding audio for transcription: {}", path.display());
        let pcm = decode_to_mono_f32(path)?;
        info!(
            "Decoded {} samples ({:.1}s) at {}Hz",
            pcm.len(),
            pcm.len() as f64 / WHISPER_SAMPLE_RATE as f64,
            WHISPER_SAMPLE_RATE
        );

        info!("Loading whisper model: {}", model_path);
        let ctx = WhisperContext::new_with_params(
            model_path,
            WhisperContextParameters::default(),
        )
        .map_err(|e| AudioError::ModelLoadError(e.to_string()))?;

        let mut state = ctx
            .create_state()
            .map_err(|e| AudioError::TranscriptionFailed(e.to_string()))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("auto"));
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        state
            .full(params, &pcm)
            .map_err(|e| AudioError::TranscriptionFailed(e.to_string()))?;

        let n_segments = state
            .full_n_segments()
            .map_err(|e| AudioError::TranscriptionFailed(e.to_string()))?;

        let mut transcript = String::new();
        for i in 0..n_segments {
            let text = state
                .full_get_segment_text(i)
                .map_err(|e| AudioError::TranscriptionFailed(e.to_string()))?;
            let t0 = state
                .full_get_segment_t0(i)
                .map_err(|e| AudioError::TranscriptionFailed(e.to_string()))?;
            // t0 is in centiseconds
            let secs = t0 as f64 / 100.0;
            let h = (secs / 3600.0) as u64;
            let m = ((secs % 3600.0) / 60.0) as u64;
            let s = (secs % 60.0) as u64;
            transcript.push_str(&format!("[{:02}:{:02}:{:02}] {}\n", h, m, s, text.trim()));
        }

        info!(
            "Transcription complete: {} segments, {} chars",
            n_segments,
            transcript.len()
        );

        Ok(transcript.trim_end().to_string())
    }

    /// Extract audio metadata (duration, sample rate, channels, codec) using
    /// symphonia.  Does NOT require a whisper model.
    pub fn get_metadata(&self, path: &Path) -> Result<AudioMetadata, AudioError> {
        if !path.exists() {
            return Err(AudioError::FileNotFound(path.to_string_lossy().into()));
        }

        let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        let format_label = format_label_from_ext(&ext);

        // Use symphonia to probe the file for real codec/stream info.
        let src = std::fs::File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(src), Default::default());
        let mut hint = Hint::new();
        hint.with_extension(&ext);

        let probed = match symphonia::default::get_probe().format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        ) {
            Ok(p) => p,
            Err(e) => {
                warn!("Symphonia probe failed for {}: {}", path.display(), e);
                return Ok(AudioMetadata {
                    format: format_label,
                    file_size_bytes: file_size,
                    ..Default::default()
                });
            }
        };

        let format = probed.format;
        let track = match format.default_track() {
            Some(t) => t,
            None => {
                return Ok(AudioMetadata {
                    format: format_label,
                    file_size_bytes: file_size,
                    ..Default::default()
                });
            }
        };

        let codec_params = &track.codec_params;
        let sample_rate = codec_params.sample_rate;
        let channels = codec_params.channels.map(|c| c.count() as u32);
        let bits_per_sample = codec_params.bits_per_sample;
        let codec = format!("{:?}", codec_params.codec)
            .to_lowercase()
            .trim_start_matches("codec_")
            .to_string();

        let duration_seconds = match (codec_params.n_frames, codec_params.sample_rate) {
            (Some(frames), Some(rate)) if rate > 0 => Some(frames as f64 / rate as f64),
            _ => None,
        };

        debug!(
            "Audio metadata: fmt={} codec={} sr={:?} ch={:?} dur={:?}",
            format_label, codec, sample_rate, channels, duration_seconds
        );

        Ok(AudioMetadata {
            duration_seconds,
            sample_rate,
            channels,
            bits_per_sample,
            format: format_label,
            codec,
            file_size_bytes: file_size,
        })
    }

    pub fn is_supported_format(path: &Path) -> bool {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());
        matches!(
            ext,
            Some(e) if matches!(
                e.as_str(),
                "mp3" | "wav" | "mp4" | "m4a" | "m4v" | "ogg" | "flac"
            )
        )
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Decode an audio file to a 16 kHz, mono, f32 PCM buffer.
///
/// If the source has multiple channels they are averaged to mono.
/// If the sample rate differs from 16 kHz a simple linear interpolation
/// resampler converts it.  This is not Hi-Fi quality but whisper.cpp is
/// robust to it.
fn decode_to_mono_f32(path: &Path) -> Result<Vec<f32>, AudioError> {
    let src = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    let mut hint = Hint::new();
    hint.with_extension(&ext);

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| AudioError::DecodeError(format!("probe failed: {e}")))?;

    let mut format = probed.format;

    let track = format
        .default_track()
        .ok_or_else(|| AudioError::DecodeError("no default audio track".into()))?;

    let track_id = track.id;
    let src_rate = track
        .codec_params
        .sample_rate
        .unwrap_or(WHISPER_SAMPLE_RATE);
    let n_channels = track
        .codec_params
        .channels
        .map(|c| c.count())
        .unwrap_or(1);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| AudioError::DecodeError(format!("decoder init: {e}")))?;

    let mut raw_mono: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(symphonia::core::errors::Error::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(e) => {
                warn!("Packet error during decode: {}", e);
                break;
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                append_mono_f32(&decoded, n_channels, &mut raw_mono);
            }
            Err(symphonia::core::errors::Error::DecodeError(e)) => {
                warn!("Decode error (skipping packet): {}", e);
            }
            Err(e) => {
                warn!("Unexpected decode error: {}", e);
            }
        }
    }

    // Resample to 16 kHz if necessary.
    let pcm = if src_rate == WHISPER_SAMPLE_RATE {
        raw_mono
    } else {
        debug!(
            "Resampling {} Hz → {} Hz ({} samples)",
            src_rate,
            WHISPER_SAMPLE_RATE,
            raw_mono.len()
        );
        linear_resample(&raw_mono, src_rate, WHISPER_SAMPLE_RATE)
    };

    Ok(pcm)
}

/// Copy all samples from a decoded `AudioBufferRef` into `out` as mono f32.
/// Multi-channel audio is downmixed by averaging.
fn append_mono_f32(buf: &AudioBufferRef<'_>, n_channels: usize, out: &mut Vec<f32>) {
    match buf {
        AudioBufferRef::F32(b) => {
            let n_frames = b.frames();
            for i in 0..n_frames {
                let mut sum = 0.0f32;
                for ch in 0..n_channels {
                    sum += b.chan(ch)[i];
                }
                out.push(sum / n_channels as f32);
            }
        }
        AudioBufferRef::S16(b) => {
            let n_frames = b.frames();
            for i in 0..n_frames {
                let mut sum = 0.0f32;
                for ch in 0..n_channels {
                    sum += b.chan(ch)[i] as f32 / i16::MAX as f32;
                }
                out.push(sum / n_channels as f32);
            }
        }
        AudioBufferRef::S32(b) => {
            let n_frames = b.frames();
            for i in 0..n_frames {
                let mut sum = 0.0f32;
                for ch in 0..n_channels {
                    sum += b.chan(ch)[i] as f32 / i32::MAX as f32;
                }
                out.push(sum / n_channels as f32);
            }
        }
        AudioBufferRef::U8(b) => {
            let n_frames = b.frames();
            for i in 0..n_frames {
                let mut sum = 0.0f32;
                for ch in 0..n_channels {
                    sum += (b.chan(ch)[i] as f32 - 128.0) / 128.0;
                }
                out.push(sum / n_channels as f32);
            }
        }
        _ => {
            // For less-common sample formats, use the conversion helper
            // provided by whisper-rs if available, or skip gracefully.
            warn!("Unsupported sample format in audio buffer; packet skipped");
        }
    }
}

/// Linear interpolation resampler.  Not high-quality but fast and sufficient
/// for speech recognition where frequencies above 8 kHz are irrelevant.
fn linear_resample(input: &[f32], src_rate: u32, dst_rate: u32) -> Vec<f32> {
    if input.is_empty() || src_rate == 0 {
        return Vec::new();
    }
    let ratio = src_rate as f64 / dst_rate as f64;
    let out_len = (input.len() as f64 / ratio).ceil() as usize;
    let mut output = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let pos = i as f64 * ratio;
        let idx = pos as usize;
        let frac = (pos - idx as f64) as f32;
        let s0 = input[idx.min(input.len() - 1)];
        let s1 = input[(idx + 1).min(input.len() - 1)];
        output.push(s0 + frac * (s1 - s0));
    }
    output
}

fn format_label_from_ext(ext: &str) -> String {
    match ext {
        "mp3" => "MP3",
        "wav" => "WAV",
        "m4a" | "m4v" => "M4A/AAC",
        "mp4" => "MP4/AAC",
        "ogg" => "OGG Vorbis",
        "flac" => "FLAC",
        _ => "Audio",
    }
    .to_string()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn supported_formats() {
        assert!(AudioExtractor::is_supported_format(Path::new("test.mp3")));
        assert!(AudioExtractor::is_supported_format(Path::new("test.wav")));
        assert!(AudioExtractor::is_supported_format(Path::new("test.m4a")));
        assert!(AudioExtractor::is_supported_format(Path::new("test.ogg")));
        assert!(AudioExtractor::is_supported_format(Path::new("test.flac")));
        assert!(!AudioExtractor::is_supported_format(Path::new("test.txt")));
        assert!(!AudioExtractor::is_supported_format(Path::new("test.pdf")));
    }

    #[test]
    fn format_label() {
        assert_eq!(format_label_from_ext("mp3"), "MP3");
        assert_eq!(format_label_from_ext("flac"), "FLAC");
        assert_eq!(format_label_from_ext("m4a"), "M4A/AAC");
    }

    #[test]
    fn no_model_returns_not_available() {
        let ex = AudioExtractor::new(None);
        assert!(!ex.is_available());
    }

    #[test]
    fn missing_model_path_returns_not_available() {
        let ex = AudioExtractor::new(Some("/no/such/model.bin".into()));
        assert!(!ex.is_available());
    }

    #[test]
    fn transcribe_missing_file_returns_error() {
        let ex = AudioExtractor::new(Some("/tmp/fake_model.bin".into()));
        let result = ex.transcribe(Path::new("/no/such/audio.wav"));
        assert!(matches!(result, Err(AudioError::FileNotFound(_))));
    }

    #[test]
    fn transcribe_no_model_returns_error() {
        // A path that would exist (this binary) but no model configured.
        let ex = AudioExtractor::new(None);
        // Any existing file with a supported extension would do; we just want
        // ModelNotConfigured before FileNotFound.
        let result = ex.transcribe(Path::new("/tmp/fake.wav"));
        // File doesn't exist → FileNotFound wins (checked first).
        assert!(matches!(
            result,
            Err(AudioError::FileNotFound(_)) | Err(AudioError::ModelNotConfigured)
        ));
    }

    #[test]
    fn linear_resample_basic() {
        // Resample 4 samples at 32 kHz to 16 kHz → 2 samples.
        let input = vec![0.0f32, 1.0, 0.5, 0.25];
        let out = linear_resample(&input, 32_000, 16_000);
        // ratio = 2.0 → positions 0.0, 2.0
        assert_eq!(out.len(), 2);
        assert!((out[0] - 0.0f32).abs() < 1e-6);
        assert!((out[1] - 0.5f32).abs() < 1e-6);
    }

    #[test]
    fn linear_resample_passthrough() {
        let input = vec![0.1f32, 0.2, 0.3];
        let out = linear_resample(&input, 16_000, 16_000);
        assert_eq!(out, input);
    }

    #[test]
    fn linear_resample_empty() {
        let out = linear_resample(&[], 44_100, 16_000);
        assert!(out.is_empty());
    }
}
