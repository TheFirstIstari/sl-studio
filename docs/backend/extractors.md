# Extractors

## Overview

The extractors module handles text extraction from various file formats. The `Deconstructor` orchestrates routing files to specialized extractors.

## Deconstructor

`extractors/deconstructor.rs` (~209 lines)

The unified extractor that routes files by extension:

```rust
struct Deconstructor {
    // Configuration and state
}

impl Deconstructor {
    fn extract(&self, file_path: &Path) -> Result<ExtractionResult> {
        // Route by extension to appropriate extractor
    }
}
```

### Supported Extensions

| Extension                                        | Extractor           |
| ------------------------------------------------ | ------------------- |
| `.pdf`                                           | `PdfExtractor`      |
| `.png`, `.jpg`, `.jpeg`, `.tiff`, `.bmp`, `.gif` | `OcrExtractor`      |
| `.mp3`, `.wav`, `.m4a`, `.mp4`, `.ogg`, `.flac`  | `AudioExtractor`    |
| `.docx`                                          | `DocumentExtractor` |
| `.txt`, `.md`                                    | `DocumentExtractor` |

## PdfExtractor

`extractors/pdf.rs` (~278 lines)

Uses the `pdf-extract` crate to extract text from PDF files.

### Features

- Full text extraction
- Page-by-page processing
- Quality assessment (character count, word density, scanned detection)
- Size limits with fallback for large files
- Error handling for password-protected and corrupted files

## OcrExtractor

`extractors/ocr.rs` (~310 lines)

Uses the `ocrs` crate for OCR on image files.

### Features

- Image preprocessing (contrast adjustment)
- Auto-rotation detection via histogram analysis
- Batch processing for multiple images
- Multi-page TIFF support
- Quality assessment of OCR results

## AudioExtractor

`extractors/audio.rs` (~163 lines)

Stub for whisper-rs integration.

### Features

- Metadata extraction (duration, sample rate, channels, format)
- Supported formats: MP3, WAV, M4A, MP4, OGG, FLAC
- Transcription via whisper.cpp (stub implementation)

## DocumentExtractor

`extractors/document.rs` (~214 lines)

Handles plain text and DOCX files.

### Features

- Plain text/Markdown reading with BOM/encoding detection
- UTF-8, UTF-16, Windows-1252 encoding support
- DOCX extraction via ZIP/XML parsing of `word/document.xml`

## ExtractionResult

Each extractor returns:

```rust
struct ExtractionResult {
    text: String,
    quality: f64,
    pages: Option<usize>,
    metadata: FileMetadata,
}
```

## Chunking

Large files are split into chunks with overlap:

- **Chunk size**: Auto-scaled based on available memory
- **Overlap**: Prevents context loss at boundaries
- **Quality per chunk**: Individual scoring for each chunk
