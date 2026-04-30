pub mod audio;
pub mod deconstructor;
pub mod document;
pub mod language;
pub mod metadata;
pub mod ocr;
pub mod pdf;
pub mod structured;

pub use deconstructor::{Deconstructor, ExtractionResult, ExtractorConfig};
pub use document::{
    detect_encoding, extract_docx, extract_text, DocumentError, DocumentExtraction,
};
