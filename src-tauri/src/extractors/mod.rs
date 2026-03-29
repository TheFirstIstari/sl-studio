pub mod audio;
pub mod deconstructor;
pub mod document;
pub mod ocr;
pub mod pdf;

pub use deconstructor::{Deconstructor, ExtractionResult, ExtractorConfig};
pub use document::{
    detect_encoding, extract_docx, extract_text, DocumentError, DocumentExtraction,
};
