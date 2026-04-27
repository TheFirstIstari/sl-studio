//! FR-STRUCT Tauri commands.

use crate::extractors::structured::{
    extract_key_value_pairs as extract_kv, extract_pdf_form_fields as extract_fields, FormField,
    KeyValuePair,
};
use std::path::PathBuf;

/// Extract AcroForm field name/value pairs from a fillable PDF.
///
/// Currently returns an empty list for any valid PDF — see
/// [`crate::extractors::structured::extract_pdf_form_fields`] for why. The
/// command is exposed now so the frontend can wire up the call site.
#[tauri::command]
pub fn extract_pdf_form_fields(path: String) -> Result<Vec<FormField>, String> {
    extract_fields(&PathBuf::from(path)).map_err(|e| e.to_string())
}

/// Extract `Key: value` pairs from arbitrary text via heuristic regex.
#[tauri::command]
pub fn extract_key_value_pairs(text: String) -> Vec<KeyValuePair> {
    extract_kv(&text)
}
