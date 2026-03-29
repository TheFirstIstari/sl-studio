pub mod pdf;
pub mod ocr;
pub mod audio;
pub mod document;
pub mod deconstructor;

pub use deconstructor::{Deconstructor, ExtractorConfig, ExtractionResult};
pub use document::{extract_text, extract_docx, detect_encoding, DocumentExtraction, DocumentError};
