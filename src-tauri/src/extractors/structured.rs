//! FR-STRUCT: Structured-data extraction.
//!
//! Two narrowly-scoped capabilities live here:
//!
//! 1. [`extract_pdf_form_fields`] — pull AcroForm field name/value pairs out
//!    of fillable PDFs (intake forms, declarations, receipts).
//! 2. [`extract_key_value_pairs`] — heuristic regex over already-extracted
//!    plain text to recover `Key: value` patterns.
//!
//! True PDF *table* extraction without ML is unreliable, so it is explicitly
//! out of scope here.

use crate::extractors::pdf::PdfError;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::OnceLock;

/// A single AcroForm widget extracted from a fillable PDF.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FormField {
    pub name: String,
    pub value: String,
    /// Field type — `"text"`, `"checkbox"`, `"choice"`, `"signature"`, etc.
    pub field_type: String,
    pub page: u32,
}

/// A `Key: value` pair recovered from free text via regex.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyValuePair {
    pub key: String,
    pub value: String,
    /// 0-based line index within the input text.
    pub line: usize,
}

/// Extract AcroForm field name/value pairs from a fillable PDF.
///
/// # Current status
///
/// `mupdf-rs` 0.6 (our pinned version) only exposes `Page::run_widgets`, which
/// *renders* widgets to a device — it does not give us iterable access to
/// AcroForm field metadata (name, value, type). The underlying MuPDF C API
/// supports it (`pdf_first_widget` / `pdf_next_widget`), but no safe Rust
/// binding exists in this version.
///
/// Rather than pull in `lopdf` as a parallel parser (it ships as a transitive
/// dep but adding it as a direct dep would split parsing across two stacks
/// for a feature with no current consumer), we keep the API surface stable
/// and return an empty `Vec`. Frontend callers can wire to this command
/// today; when we upgrade `mupdf-rs` (or vendor a small extension over
/// `mupdf-sys`) the implementation will fill in without an API break.
///
/// Returns `Ok(vec![])` — never an error — when no fields are found or when
/// the PDF has no AcroForm. A genuine open/parse failure surfaces as
/// [`PdfError`].
pub fn extract_pdf_form_fields(path: &Path) -> Result<Vec<FormField>, PdfError> {
    // Validate that the file at least opens cleanly so a caller passing a
    // bad path still gets a useful error rather than a silent empty Vec.
    let _doc = mupdf::Document::open(path.to_string_lossy().as_ref())
        .map_err(|e| PdfError::ExtractionError(format!("Failed to open PDF: {}", e)))?;

    // mupdf-rs 0.6: no widget iteration API. See module docs.
    Ok(Vec::new())
}

fn kv_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Key: starts with uppercase letter, then letters/spaces/slash/hyphen,
        // 3-41 chars total. Value: non-whitespace start, up to 200 more chars,
        // anchored to end of line.
        Regex::new(r"^(?P<key>[A-Z][A-Za-z\s/-]{2,40}):\s*(?P<value>\S.{0,200})$").unwrap()
    })
}

/// Extract `Key: value` pairs from arbitrary text, line by line.
///
/// Heuristic: a key must start with an uppercase letter, be 3–41 characters
/// long, and contain only letters, spaces, slashes, or hyphens. The value
/// must start with a non-whitespace character and be ≤ 200 characters.
/// Lines that don't match are silently skipped.
pub fn extract_key_value_pairs(text: &str) -> Vec<KeyValuePair> {
    let re = kv_regex();
    text.lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            let trimmed = line.trim_end();
            let caps = re.captures(trimmed)?;
            let key = caps.name("key")?.as_str().trim().to_string();
            let value = caps.name("value")?.as_str().trim().to_string();
            if key.len() > 40 || value.len() > 200 || key.is_empty() || value.is_empty() {
                return None;
            }
            Some(KeyValuePair {
                key,
                value,
                line: idx,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn kv_extracts_multiple_pairs() {
        let text = "\
Name: Jane Doe
Date: 2024-01-15
Case Number: 2024-CR-0042
Officer: Det. Smith
";
        let pairs = extract_key_value_pairs(text);
        assert_eq!(pairs.len(), 4);
        assert_eq!(pairs[0].key, "Name");
        assert_eq!(pairs[0].value, "Jane Doe");
        assert_eq!(pairs[0].line, 0);
        assert_eq!(pairs[1].key, "Date");
        assert_eq!(pairs[1].value, "2024-01-15");
        assert_eq!(pairs[2].key, "Case Number");
        assert_eq!(pairs[2].value, "2024-CR-0042");
        assert_eq!(pairs[3].key, "Officer");
        assert_eq!(pairs[3].value, "Det. Smith");
    }

    #[test]
    fn kv_returns_empty_when_no_pairs() {
        let text = "this is just prose\nwith no key value pairs at all\nor: lowercase keys";
        let pairs = extract_key_value_pairs(text);
        assert!(pairs.is_empty());
    }

    #[test]
    fn kv_extracts_only_matching_lines_from_mixed_prose() {
        let text = "\
Some introductory paragraph that should be ignored entirely.
Name: Alice
This line is prose and should not match anything.
Location: 123 Main St
Another sentence with no colon at all
Status: Active
";
        let pairs = extract_key_value_pairs(text);
        assert_eq!(pairs.len(), 3);
        assert_eq!(pairs[0].key, "Name");
        assert_eq!(pairs[0].line, 1);
        assert_eq!(pairs[1].key, "Location");
        assert_eq!(pairs[1].line, 3);
        assert_eq!(pairs[2].key, "Status");
        assert_eq!(pairs[2].line, 5);
    }

    #[test]
    fn kv_skips_overly_long_keys() {
        // Key here is > 40 characters and should be rejected.
        let long_key = "A".to_string() + &"b".repeat(50);
        let text = format!("{}: value\nName: Bob", long_key);
        let pairs = extract_key_value_pairs(&text);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].key, "Name");
    }

    #[test]
    fn kv_skips_empty_values() {
        let text = "Name:   \nDate: 2024-01-01";
        let pairs = extract_key_value_pairs(text);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].key, "Date");
    }

    #[test]
    #[ignore = "requires AcroForm PDF fixture; mupdf 0.6 has no widget iteration API"]
    fn form_fields_returns_empty_for_non_form_pdf() {
        // Placeholder: when a real fillable PDF fixture is added under
        // tests/fixtures/, point this at it. For now the function always
        // returns Ok(vec![]) so we cannot meaningfully test extraction.
        let path = PathBuf::from("tests/fixtures/sample_form.pdf");
        let fields = extract_pdf_form_fields(&path).expect("open ok");
        assert!(fields.is_empty());
    }
}
