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
use lopdf::Document as LopdfDocument;
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
/// Uses `lopdf` to walk the AcroForm field tree. Returns `Ok(vec![])` when
/// the PDF has no AcroForm or no fields. A genuine open/parse failure
/// surfaces as [`PdfError`].
pub fn extract_pdf_form_fields(path: &Path) -> Result<Vec<FormField>, PdfError> {
    let doc = LopdfDocument::load(path)
        .map_err(|e| PdfError::ExtractionError(format!("Failed to open PDF: {}", e)))?;

    // Walk the AcroForm fields array from the document catalog.
    let fields_array = {
        let catalog = doc.catalog()
            .map_err(|e| PdfError::ExtractionError(format!("No catalog: {}", e)))?;
        match catalog.get(b"AcroForm") {
            Ok(acroform_obj) => {
                let acroform_id = acroform_obj.as_reference()
                    .map_err(|e| PdfError::ExtractionError(format!("AcroForm ref: {}", e)))?;
                let acroform_dict = doc.get_dictionary(acroform_id)
                    .map_err(|e| PdfError::ExtractionError(format!("AcroForm dict: {}", e)))?;
                match acroform_dict.get(b"Fields") {
                    Ok(f) => f.as_array()
                        .map_err(|e| PdfError::ExtractionError(format!("Fields array: {}", e)))?
                        .clone(),
                    Err(_) => return Ok(Vec::new()),
                }
            }
            Err(_) => return Ok(Vec::new()),
        }
    };

    let mut results = Vec::new();
    for field_obj in &fields_array {
        let field_id = match field_obj.as_reference() {
            Ok(id) => id,
            Err(_) => continue,
        };
        collect_fields(&doc, field_id, None, &mut results);
    }

    Ok(results)
}

/// Recursively walk a field node (and its Kids) collecting terminal fields.
fn collect_fields(
    doc: &LopdfDocument,
    field_id: lopdf::ObjectId,
    parent_name: Option<&str>,
    out: &mut Vec<FormField>,
) {
    let dict = match doc.get_dictionary(field_id) {
        Ok(d) => d,
        Err(_) => return,
    };

    // Build full dotted name: parent.T
    let partial = dict
        .get(b"T")
        .ok()
        .and_then(|o| o.as_string().ok().map(|s| s.into_owned()));
    let full_name = match (parent_name, partial.as_deref()) {
        (Some(p), Some(n)) => format!("{}.{}", p, n),
        (None, Some(n)) => n.to_owned(),
        (Some(p), None) => p.to_owned(),
        (None, None) => String::new(),
    };

    // If it has Kids, recurse — this is an intermediate node.
    if let Ok(kids) = dict.get(b"Kids").and_then(|o| o.as_array()) {
        let kids = kids.clone();
        for kid in &kids {
            if let Ok(kid_id) = kid.as_reference() {
                collect_fields(doc, kid_id, Some(&full_name), out);
            }
        }
        return;
    }

    // Terminal field — extract FT, V, and page number.
    let field_type = dict
        .get(b"FT")
        .ok()
        .and_then(|o| o.as_name_str().ok().map(|s| s.to_lowercase()))
        .unwrap_or_else(|| "text".to_string());

    let value = dict
        .get(b"V")
        .ok()
        .and_then(|o| match o {
            lopdf::Object::String(bytes, _) => Some(String::from_utf8_lossy(bytes).into_owned()),
            lopdf::Object::Name(n) => Some(String::from_utf8_lossy(n).into_owned()),
            lopdf::Object::Boolean(b) => Some(if *b { "true" } else { "false" }.to_string()),
            _ => None,
        })
        .unwrap_or_default();

    // Best-effort page number from the widget's /P reference.
    let page = dict
        .get(b"P")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .and_then(|page_id| {
            doc.page_iter()
                .position(|id| id == page_id)
                .map(|i| i as u32 + 1)
        })
        .unwrap_or(0);

    out.push(FormField {
        name: full_name,
        value,
        field_type,
        page,
    });
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
    #[ignore = "requires AcroForm PDF fixture; add tests/fixtures/sample_form.pdf to enable"]
    fn form_fields_returns_empty_for_non_form_pdf() {
        // Point at a real fillable PDF fixture to test field extraction.
        let path = PathBuf::from("tests/fixtures/sample_form.pdf");
        let fields = extract_pdf_form_fields(&path).expect("open ok");
        assert!(!fields.is_empty());
    }
}
