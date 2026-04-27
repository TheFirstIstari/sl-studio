//! Language detection for extracted text (FR-LANG-002).
//!
//! Thin wrapper around the `whatlang` crate. Returns ISO 639-3 codes
//! (e.g. "eng", "spa", "fra", "deu", "cmn", "jpn", "kor", "ara") so
//! they can be stored directly in the `intelligence.source_language`
//! column without any further normalization.

/// Minimum text length (chars) we'll attempt to classify. Below this,
/// `whatlang` produces near-random guesses, so we explicitly bail.
const MIN_LEN: usize = 25;

/// Maximum text length (chars) passed to the detector. `whatlang`
/// scales linearly in input size; trigram statistics stabilize well
/// before 4 KiB, so truncating keeps us O(1) on huge documents.
const MAX_LEN: usize = 4096;

/// Below this confidence, whatlang's pick is too unreliable to store.
const MIN_CONFIDENCE: f64 = 0.5;

/// Detect the dominant language of a piece of extracted text.
///
/// Returns the ISO 639-3 code (e.g. `"eng"`, `"spa"`, `"cmn"`) when
/// `whatlang` is reasonably confident; otherwise `None`. Texts shorter
/// than [`MIN_LEN`] characters are not classified.
pub fn detect_language(text: &str) -> Option<&'static str> {
    detect_language_with_confidence(text).map(|(code, _)| code)
}

/// Same as [`detect_language`] but returns the detector's confidence
/// in `[0.0, 1.0]` alongside the ISO 639-3 code.
pub fn detect_language_with_confidence(text: &str) -> Option<(&'static str, f64)> {
    let trimmed = text.trim();
    if trimmed.chars().count() < MIN_LEN {
        return None;
    }

    // Truncate by char boundary, not byte index, so multi-byte scripts
    // (CJK, Arabic, etc.) don't panic on slice bounds.
    let truncated: String = trimmed.chars().take(MAX_LEN).collect();

    let info = whatlang::detect(&truncated)?;
    let confidence = info.confidence();
    if confidence < MIN_CONFIDENCE {
        return None;
    }
    Some((info.lang().code(), confidence))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_english_paragraph() {
        let text = "The quick brown fox jumps over the lazy dog. \
                    This sentence contains every letter of the English alphabet \
                    and is commonly used to test fonts and language detectors.";
        assert_eq!(detect_language(text), Some("eng"));
    }

    #[test]
    fn detects_spanish_paragraph() {
        let text = "El veloz murciélago hindú comía feliz cardillo y kiwi. \
                    La cigüeña tocaba el saxofón detrás del palenque de paja. \
                    Esta es una frase en español con suficiente contexto.";
        assert_eq!(detect_language(text), Some("spa"));
    }

    #[test]
    fn rejects_empty_or_short_text() {
        assert_eq!(detect_language(""), None);
        assert_eq!(detect_language("   "), None);
        assert_eq!(detect_language("hi"), None);
        assert_eq!(detect_language("short text"), None);
    }

    #[test]
    fn rejects_garbage_or_single_char() {
        assert_eq!(detect_language("a"), None);
        assert_eq!(detect_language("xxxxxxxxxxxxxxxxxxxxxxxxxxxxx"), None);
        assert_eq!(detect_language("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"), None);
    }
}
