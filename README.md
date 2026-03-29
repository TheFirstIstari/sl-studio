# SL Studio - Forensic Document Analysis Platform

A high-performance desktop application for forensic document analysis, built with Tauri + Rust + SvelteKit.

## Overview

SL Studio processes evidence files (PDFs, images, audio/video) through a pipeline of extraction and AI-powered reasoning to extract structured facts for forensic investigations.

### Key Features

- **Local Processing**: All processing runs locally (privacy-first, no cloud dependencies)
- **Rust Backend**: Native performance with parallel file hashing and extraction
- **LLM Integration**: Local GGUF model inference for fact extraction via llama.cpp
- **SQLite Storage**: Registry and Intelligence databases for evidence tracking
- **Auto-scaling**: Automatically configures batch sizes and workers based on hardware
- **Project Files**: Save/load investigation configurations as `.sls` files
- **HuggingFace Integration**: Download GGUF models directly from HuggingFace

## Architecture

```
+-------------------------------------------------------------+
|                   SvelteKit Frontend                         |
|   (Dashboard, Settings, Analysis, Results)                   |
+----------------------------+--------------------------------+
                             | Tauri Commands (IPC)
+----------------------------v--------------------------------+
|                      Rust Backend                            |
|  +-------------+  +-------------+  +--------------------+   |
|  | Deconstructor|  | LLM Reasoner|  | Database Manager  |   |
|  |(PDF/OCR/Audio)|| (llama.cpp) |  | (rusqlite)        |   |
|  +-------------+  +-------------+  +--------------------+   |
|  +-------------+  +-------------+                           |
|  |GPU Detection|  |Auto-scaling |                           |
|  +-------------+  +-------------+                           |
+-------------------------------------------------------------+
```

## Tech Stack

| Layer | Technology |
|-------|-------------|
| Frontend | SvelteKit + TypeScript + SVG Icons |
| Desktop | Tauri 2 |
| Backend | Rust |
| Database | SQLite (rusqlite) |
| OCR | ocrs |
| PDF | pdf-extract |
| LLM | llama.cpp (GGUF models) |
| Audio | whisper.cpp (optional) |

## Getting Started

### Prerequisites

- Node.js 22+
- Rust stable
- macOS (primary), Linux, or Windows

### Installation

```bash
# Install dependencies
mise run setup

# Or manually
npm install
cargo fetch --manifest-path src-tauri/Cargo.toml
```

### Development

```bash
# Terminal 1: Watch Rust with bacon (recommended)
mise run watch_rust

# Terminal 2: Run Tauri dev server
mise run dev
```

### Testing

```bash
# All tests
mise run test

# Rust only
mise run test_rust

# With bacon (watch mode)
mise run watch_tests
```

### Building

```bash
# Release build
mise run release

# Or specific bundles
mise run release_dmg   # macOS DMG
mise run release_app   # macOS app
```

## Configuration

### Mise Tasks

| Task | Description |
|------|-------------|
| `mise run dev` | Start Tauri dev server |
| `mise run test` | Run all tests |
| `mise run build` | Build Tauri app |
| `mise run check` | Quick compile check |
| `mise run watch_rust` | Watch + build with bacon |
| `mise run watch_tests` | Watch + test with bacon |
| `mise run lint_rust` | Run clippy |
| `mise run release` | Production build |
| `mise run clean` | Clean build artifacts |

## Project File Format

SL Studio uses `.sls` JSON project files to store investigation settings:

```json
{
  "version": "1.0.0",
  "created_at": "2024-01-15T10:30:00Z",
  "modified_at": "2024-01-15T12:45:00Z",
  "investigator": {
    "name": "John Doe",
    "agency": "FBI",
    "case_number": "CASE-2024-001",
    "notes": ""
  },
  "paths": {
    "evidence_root": "/path/to/evidence",
    "registry_db": "./data/registry.db",
    "intelligence_db": "./data/intelligence.db",
    "export_dir": "./exports",
    "models_dir": "./models"
  },
  "model": {
    "source": "huggingface",
    "model_id": "TheBloke/Mistral-7B-Instruct-v0.2-GGUF",
    "quantization": "Q4_K_M",
    "context_length": 16384,
    "local_path": "./models/mistral-7b.gguf"
  },
  "hardware": {
    "gpu_backend": "metal",
    "gpu_memory_fraction": 0.45,
    "cpu_workers": 8,
    "ocr_provider": "onnx",
    "whisper_size": "base"
  },
  "processing": {
    "batch_size": 24,
    "max_image_resolution": 2048,
    "enable_ocr": true,
    "enable_audio": true,
    "enable_pdf_extraction": true
  },
  "metadata": {
    "total_files": 150,
    "processed_files": 45,
    "facts_extracted": 127,
    "last_scan_date": "2024-01-15T10:30:00Z",
    "last_analysis_date": "2024-01-15T12:45:00Z",
    "tags": ["financial", "suspect-xyz"]
  }
}
```

## Database Schema

### Registry DB
```sql
CREATE TABLE registry (
    id INTEGER PRIMARY KEY,
    fingerprint TEXT UNIQUE,
    path TEXT,
    file_size INTEGER,
    file_type TEXT,
    file_name TEXT,
    processed INTEGER DEFAULT 0,
    processed_at DATETIME
);
-- Indexes
CREATE INDEX idx_registry_fingerprint ON registry(fingerprint);
CREATE INDEX idx_registry_processed ON registry(processed);
CREATE INDEX idx_registry_processed_id ON registry(processed, id);
```

### Intelligence DB
```sql
CREATE TABLE intelligence (
    id INTEGER PRIMARY KEY,
    registry_id INTEGER,
    fingerprint TEXT,
    filename TEXT,
    evidence_quote TEXT,
    fact_summary TEXT,
    category TEXT,
    identified_crime TEXT,
    severity_score INTEGER,
    confidence REAL
);
-- Indexes
CREATE INDEX idx_intelligence_severity ON intelligence(severity_score DESC);
CREATE UNIQUE INDEX idx_intelligence_unique ON intelligence(fingerprint, filename, fact_summary);
```

## Performance Optimizations

The application includes several optimizations for large-scale evidence processing:

- **Batch Database Inserts**: Fingerprints inserted in batches of 100
- **Fingerprint Cache**: Pre-loads existing fingerprints to skip duplicates
- **64KB Hash Buffer**: 8x larger than default for faster I/O
- **Composite Indexes**: Optimized database queries
- **Auto-scaling**: Hardware detection configures optimal batch sizes and workers
- **Model Caching**: LLM stays loaded in memory between analyses

## Testing

- **Unit Tests**: Inline in source files
- **Integration Tests**: `src-tauri/tests/`
- **Test Coverage**: 45+ tests

Run tests:
```bash
mise run test_rust    # 45+ tests
mise run test_ci      # CI mode
```

## Dependencies

### Required Crates

- `tauri` - Desktop framework
- `rusqlite` - SQLite database
- `pdf-extract` - PDF text extraction
- `ocrs` - OCR engine
- `llama_cpp` - LLM inference
- `sysinfo` - Hardware detection
- `rayon` - Parallel processing
- `sha2` - File hashing

### Optional (Feature-gated)

- `whisper-cpp-plus` - Audio transcription (requires cmake)

## Migration from Qt

This project is a Rust migration of the original Python/Qt `Project-SteinLine`.

### Changes from Qt Version

| Component | Qt (Python) | Rust/Tauri |
|-----------|-------------|------------|
| UI | Qt Widgets | SvelteKit |
| Database | sqlite3 | rusqlite |
| OCR | EasyOCR | ocrs |
| Audio | Faster Whisper | whisper.cpp |
| LLM | vLLM (HTTP) | llama.cpp (local) |
| Hardware | psutil + pynvml | sysinfo |

## License

MIT

## Contributing

1. Fork the repository
2. Create a feature branch
3. Run tests: `mise run test_rust`
4. Submit a pull request
