# Unit Tests

## Overview

Rust unit tests are defined using the standard `#[cfg(test)]` module pattern.
Tests are co-located with the source files they test.

> **Note**: As of the rapid-mlx swap completion, no Rust unit tests currently
> exist in the codebase. The `cargo test` run reports 0 tests. See
> [E2E Tests](e2e.md) for the Playwright end-to-end test suite.

## Test Structure

Tests are co-located with the source files they test:

```
src-tauri/src/
├── core/
│   └── database.rs     # Tests for database operations (none currently)
├── extractors/
│   ├── pdf.rs          # Tests for PDF extraction (none currently)
│   ├── docx.rs         # Tests for DOCX extraction (none currently)
│   ├── image.rs        # Tests for image extraction (none currently)
│   └── audio.rs        # Tests for audio extraction (none currently)
├── inference/
│   ├── mlx_pipeline.rs # Tests for MLX pipeline (none currently)
│   └── reasoner.rs     # Tests for reasoner (none currently)
└── commands/mod.rs     # Tests for command handlers (none currently)
```

## Running Tests

### Using mise (recommended)

```bash
mise run ci_test       # Run all Rust tests
```

### Using cargo directly

```bash
cd src-tauri && cargo test
cargo test -- --nocapture  # Show output
cargo test -- test_name    # Single test
cargo test -- --test-threads=1  # Serialise database tests
```

## Test Categories

### Database Tests

- Schema initialization (migrations)
- CRUD operations via `Pool` methods
- Query operations (fact search, entity relationships)

### Extractor Tests

- File type routing in `extract_metadata_from_path`
- PDF, DOCX, image, audio extraction
- Error handling (missing files, unsupported formats)

### Inference Tests

- MlxPipeline startup and health-checking
- Reasoner fact extraction
- Pipeline execution (when tests are added)

## Integration Tests

Integration tests are not currently present. If added, they would be located
in `src-tauri/tests/` and test cross-module functionality.

## E2E Tests

The project maintains 18 Playwright end-to-end tests in the `e2e/` directory.
See [E2E Tests](e2e.md) for details.
