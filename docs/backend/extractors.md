# Extractors

## Overview

The extractors module provides file-type-specific text and metadata extraction.
Each extractor is an async function that takes a file path and returns a `Metadata`
record. The module provides `extract_metadata_from_path` which gathers basic file
metadata (type, size, name). File-type-specific text extraction dispatch
happens in `commands/mod.rs` via the private `extract_file` helper, which
routes to `extract_pdf`, `extract_image`, `extract_audio`, or `extract_docx`
based on file extension.

```
File Path
    │
    ▼
┌─────────────────────────┐
│ extract_metadata_from_  │ ← Extension-based routing
│ path()                  │
└────────┬────────────────┘
         │
    ┌────┴─────────────────────────────────┐
    ▼                                      ▼
┌─────────────┐                     ┌────────────┐
│ PDF         │                     │ Image      │
│ extract_pdf │                     │ extract_   │
│ (pdf.rs)    │                     │ image      │
└─────────────┘                     │ (image.rs) │
┌─────────────┐                     └────────────┘
│ DOCX        │                     ┌────────────┐
│ extract_dcx │                     │ Audio      │
│ (docx.rs)   │                     │ extract_   │
└─────────────┘                     │ audio      │
                                    │ (audio.rs) │
                                    └────────────┘
```

## Module Layout

```
src-tauri/src/extractors/
├── mod.rs           # Module declarations + extract_metadata_from_path (~70 lines)
├── pdf.rs           # PDF extraction (~22 lines)
├── docx.rs          # DOCX text extraction (~22 lines)
├── image.rs         # OCR/image extraction (~22 lines)
└── audio.rs         # Audio extraction (~22 lines)
```

## extract_metadata_from_path

`extractors/mod.rs` — dispatches to the correct extractor based on file extension.

Returns a `DocumentMetadata` struct with detected file type, size, and basic info.

### Supported Extensions

| Extension | Extractor |
| --------- | --------- |
| `.pdf`    | `extract_pdf` |
| `.png`, `.jpg`, `.jpeg`, `.tiff`, `.bmp` | `extract_image` |
| `.mp3`, `.wav`, `.m4a`, `.flac`, `.aac` | `extract_audio` |
| `.docx`   | `extract_docx` |
| (other)   | `"text"` source |

## PDF Extraction (`extract_pdf`)

`extractors/pdf.rs` (~22 lines)

Extracts text from PDF files.

```rust
pub async fn extract_pdf(path: &str) -> Result<Metadata>
```

### Features

- Reads PDF content
- Returns `Metadata` with category "PDF", fact summary, and confidence score

## Image Extraction (`extract_image`)

`extractors/image.rs` (~22 lines)

Extracts text from image files (OCR).

```rust
pub async fn extract_image(path: &str) -> Result<Metadata>
```

### Features

- Reads image file content
- Returns `Metadata` with category "Image" and confidence score

## Audio Extraction (`extract_audio`)

`extractors/audio.rs` (~22 lines)

Extracts metadata from audio files.

```rust
pub async fn extract_audio(path: &str) -> Result<Metadata>
```

### Features

- Reads audio file content
- Returns `Metadata` with category "Audio" and confidence score

## DOCX Extraction (`extract_docx`)

`extractors/docx.rs` (~22 lines)

Extracts text from DOCX files.

```rust
pub async fn extract_docx(path: &str) -> Result<Metadata>
```

### Features

- Reads DOCX file content
- Returns `Metadata` with category "Document" and confidence score

## Output Type

All extractors return a `Metadata` struct:

```rust
pub struct Metadata {
    pub filename: String,
    pub category: String,
    pub severity_score: u8,
    pub confidence: Option<f64>,
    pub identified_crime: Option<String>,
    pub fact_summary: String,
    pub fingerprint: String,
    pub created_at: String,
    pub updated_at: String,
}
```
