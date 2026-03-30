# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-03-30

### Added

#### Phase 1: Foundation

- Tauri 2 + SvelteKit project structure
- SQLite database with rusqlite
- File system walker with fingerprinting
- Config management system

#### Phase 2: Text Extraction

- PDF text extraction (pdf-extract)
- Image OCR (ocrs with preprocessing)
- Audio transcription support
- Document parsing (DOCX, TXT, MD)

#### Phase 3: LLM Integration

- llama.cpp bindings for local inference
- Model download from HuggingFace
- Multi-pass pipeline framework
- Built-in pipelines: Basic Facts, Financial Crimes, Document Analysis, Image OCR, Audio Transcription

#### Phase 4: Data Management

- Intelligence database with FTS5
- Quality scoring system
- Fact deduplication
- Incremental processing

#### Phase 5: Search & Analysis

- Full-text search (FTS5)
- Temporal analysis
- Network analysis (degree, betweenness centrality)
- Anomaly detection
- Evidence weighting

#### Phase 6: User Interface

- Dashboard with statistics
- Analysis configuration page
- Results viewer with filtering
- Timeline visualization
- Network graph (Cytoscape.js)
- Maps integration (Leaflet.js)
- Statistics charts (Chart.js)
- Anomaly dashboard
- Keyboard shortcuts
- Bulk operations
- Undo/redo
- Annotation system
- Tagging system

#### Phase 7: Export & Reporting

- JSON export
- CSV export
- PDF report generation
- Excel data export
- Cross-project case comparison
- Entity overlap detection
- Timeline correlation

#### Phase 8: System Integration

- Real-time system monitoring (CPU, memory, GPU)
- Batch size tuning
- Hardware auto-detection
- Backup/restore functionality
- Notification system

### Technical Details

- Built with Tauri 2 (Rust backend + SvelteKit frontend)
- SQLite databases for data storage
- Local-only processing (no cloud dependencies)
- GPU-accelerated inference (optional)
- Supports Windows, macOS, and Linux

---

## Template

## [version] - YYYY-MM-DD

### Added

- New features

### Changed

- Changes to existing functionality

### Deprecated

- Features that will be removed in future versions

### Removed

- Removed features

### Fixed

- Bug fixes

### Security

- Vulnerability fixes
