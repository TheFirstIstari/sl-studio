# Extractors

## Overview

The extractors module provides comprehensive file analysis capabilities:

1. **Text Extraction**: The `Deconstructor` orchestrates routing files to specialized extractors for text content extraction.
2. **Metadata Extraction**: The `MetadataExtractor` extracts EXIF metadata from images and document metadata from PDFs.
3. **Structured Data Extraction**: Extract form fields from fillable PDFs and key-value pairs from plain text.

These extractors work together to provide both content and contextual information about files.

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

| Extension                                                | Extractor           |
| -------------------------------------------------------- | ------------------- |
| `.pdf`                                                   | `PdfExtractor`      |
| `.png`, `.jpg`, `.jpeg`, `.tiff`, `.tif`, `.bmp`, `.gif` | `OcrExtractor`      |
| `.heic`, `.heif`, `.webp`                                | Metadata only\*     |
| `.mp3`, `.wav`, `.m4a`, `.mp4`, `.m4v`, `.ogg`, `.flac`  | `AudioExtractor`    |
| `.docx`                                                  | `DocumentExtractor` |
| `.txt`, `.md`, `.json`, `.xml`, `.csv`                   | `DocumentExtractor` |

\* HEIC/HEIF/WebP files support metadata extraction (EXIF) but not text extraction.

## PdfExtractor

`extractors/pdf.rs` (~278 lines)

Uses the `pdf-extract` crate to extract text from PDF files.

### Features

- Full text extraction
- Page-by-page processing
- Quality assessment (character count, word density, scanned detection)
- Size limits with fallback for large files
- Error handling for password-protected and corrupted files
- Panic handling with descriptive error messages

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

## MetadataExtractor

`extractors/metadata.rs` (~333 lines)

Extracts metadata from images (EXIF) and PDFs without requiring database access. This extractor focuses purely on metadata — not text content.

### Supported Formats

| Format                      | Metadata Type                   | Library        |
| --------------------------- | ------------------------------- | -------------- |
| JPEG, PNG, TIFF, HEIC, WebP | EXIF tags                       | `kamadak-exif` |
| PDF                         | Document Information Dictionary | `lopdf`        |

### Features

**EXIF Metadata (Images)**

- Camera information (make, model)
- Capture date/time (`DateTimeOriginal`, `DateTime`, `DateTimeDigitized`)
- GPS coordinates (latitude/longitude with hemisphere handling)
- Software/Artist/ImageDescription tags
- All raw EXIF tags preserved in `raw` map

**PDF Metadata**

- Document Information Dictionary fields: Title, Author, Subject, Keywords, Creator, Producer
- Dates normalized to ISO 8601 format (`CreationDate`, `ModDate`)
- Handles both UTF-8 and UTF-16BE (with BOM) encoded strings

**Output Structure**

```rust
struct DocumentMetadata {
    source: String,              // "exif", "pdf", or "none"
    title: Option<String>,
    author: Option<String>,
    subject: Option<String>,
    creator: Option<String>,
    producer: Option<String>,
    created_at: Option<String>,
    modified_at: Option<String>,
    keywords: Option<String>,
    camera_model: Option<String>,
    gps_latitude: Option<f64>,
    gps_longitude: Option<f64>,
    raw: BTreeMap<String, String>,  // All raw key/value pairs
}
```

### Error Handling

- Returns `DocumentMetadata { source: "none" }` for unsupported file types (not an error)
- Returns `MetadataError::FileNotFound` for missing files
- Individual parse errors (EXIF/PDF) wrapped in `MetadataError::Exif` or `MetadataError::Pdf`

## StructuredExtractor

`extractors/structured.rs` (~186 lines)

Extracts structured data from documents — form fields from fillable PDFs and key-value pairs from plain text.

### Features

**PDF Form Fields** (`extract_pdf_form_fields`)

- Extracts AcroForm field name/value pairs from fillable PDFs
- Returns field type (`text`, `checkbox`, `choice`, `signature`, etc.) and page number
- Current status: API stable but returns empty vec (mupdf-rs 0.6 lacks widget iteration API)

**Key-Value Pair Extraction** (`extract_key_value_pairs`)

- Heuristic regex extraction from plain text
- Key pattern: Starts with uppercase letter, 3-41 characters, letters/spaces/slash/hyphen
- Value pattern: Non-whitespace start, up to 200 characters, anchored to end of line
- Returns line index for each extracted pair

### Output Structures

```rust
struct FormField {
    name: String,
    value: String,
    field_type: String,  // "text", "checkbox", "choice", "signature", etc.
    page: u32,
}

struct KeyValuePair {
    key: String,
    value: String,
    line: usize,  // 0-based line index in source text
}
```

### Usage Example

```rust
use crate::extractors::structured::{extract_pdf_form_fields, extract_key_value_pairs};

// Extract form fields from a fillable PDF
let fields = extract_pdf_form_fields(path)?;

// Extract key-value pairs from plain text
let text = "Name: John Doe\nDate: 2024-01-15\nCase Number: 2024-CR-0042";
let pairs = extract_key_value_pairs(text);
```

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
