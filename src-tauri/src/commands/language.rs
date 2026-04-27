//! Frontend-facing language detection command (FR-LANG).
//!
//! Pure-CPU wrapper around [`crate::extractors::language`]. Has no DB
//! or Tauri state requirements, so we expose it as a free function
//! that the UI can call against any text snippet.

use crate::extractors::language::detect_language_with_confidence;

/// Detect the dominant language of `text`.
///
/// Returns `Some((iso_639_3_code, confidence))` when whatlang is
/// reasonably confident, otherwise `None`. See
/// [`crate::extractors::language`] for thresholds.
#[tauri::command]
pub fn detect_text_language(text: String) -> Option<(String, f64)> {
    detect_language_with_confidence(&text).map(|(code, conf)| (code.to_string(), conf))
}
