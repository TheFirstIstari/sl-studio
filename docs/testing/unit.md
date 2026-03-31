# Unit Tests

## Overview

Rust unit tests are defined throughout the codebase using the standard `#[cfg(test)]` module pattern.

## Test Structure

Tests are co-located with the code they test:

```
src-tauri/src/
├── core/
│   └── database.rs      # Tests for database operations
├── extractors/
│   ├── pdf.rs           # Tests for PDF extraction
│   ├── ocr.rs           # Tests for OCR
│   └── document.rs      # Tests for document parsing
├── inference/
│   ├── pipeline.rs      # Tests for pipeline execution
│   └── reasoner.rs      # Tests for reasoner
└── inference/quality/
    ├── scoring.rs       # Tests for quality scoring
    └── deduplication.rs # Tests for deduplication
```

## Running Tests

### Using mise (recommended)

```bash
mise run test              # Run all tests
mise run test_quick        # Quick test run
```

### Using cargo directly

```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture  # Show output
cargo test --manifest-path src-tauri/Cargo.toml -- test_name    # Single test
```

## Test Categories

### Database Tests

- Schema initialization
- CRUD operations
- Search queries
- FTS5 functionality

### Extractor Tests

- PDF text extraction
- OCR accuracy
- Document parsing
- Error handling (corrupted files, password-protected)

### Inference Tests

- Pipeline execution
- Fact parsing
- JSON extraction
- Deduplication logic

### Quality Tests

- Scoring calculations
- Threshold validation
- Merge strategies

## Integration Tests

Located in `src-tauri/tests/` for cross-module testing.
