use std::fs;
use steinline_lib::extractors::{Deconstructor, ExtractionResult, ExtractorConfig};
use tempfile::TempDir;

#[test]
fn test_extractor_config_default() {
    let config = ExtractorConfig::default();
    assert_eq!(config.use_gpu_ocr, false);
    assert_eq!(config.whisper_model_path.is_none(), true);
}

#[test]
fn test_deconstructor_with_nonexistent_file() {
    let config = ExtractorConfig::default();
    let deconstructor = Deconstructor::new(config).unwrap();

    let result = deconstructor.extract(std::path::Path::new("/nonexistent/file.pdf"));
    assert!(result.is_err());
}

#[test]
fn test_deconstructor_supported_extensions() {
    let extensions = Deconstructor::supported_extensions();

    assert!(extensions.contains(&"pdf"));
    assert!(extensions.contains(&"txt"));
    assert!(extensions.contains(&"mp3"));
    assert!(extensions.contains(&"png"));
}

#[test]
fn test_extraction_result_struct() {
    let result = ExtractionResult {
        text: "Test text".to_string(),
        source_file: "test.pdf".to_string(),
        file_type: "pdf".to_string(),
        char_count: 9,
        is_partial: false,
    };

    assert_eq!(result.char_count, 9);
    assert_eq!(result.file_type, "pdf");
}

#[test]
fn test_text_file_extraction() {
    let tmp_dir = TempDir::new().unwrap();
    let file_path = tmp_dir.path().join("test.txt");
    fs::write(&file_path, "Hello, World!").unwrap();

    let config = ExtractorConfig::default();
    let deconstructor = Deconstructor::new(config).unwrap();

    let result = deconstructor.extract(&file_path);
    assert!(result.is_ok());

    let extracted = result.unwrap();
    assert!(extracted.text.contains("Hello"));
}
