# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- **Tauri `frontendDist` path** (`tauri.conf.json`): corrected from `../dist` to
  `../build` so the Tauri bundler can locate SvelteKit adapter-static output
- **Windows bundle icon** (`tauri.conf.json`): added `icons/icon.ico` to the
  `bundle.icon` array — Windows build target requires an `.ico` file
- **Self-hosted runner CI jobs**: added `timeout-minutes: 15` and `continue-on-error: true`
  so offline self-hosted runners (Fedora, NixOS) don't block CI; added fallback shell
  scripts that use `mise run` when available, falling back to direct `cargo` commands
- **Vestigial `local_path` field** (`lib.rs`, `commands/mod.rs`, `app.ts`,
  `settings/+page.svelte`): removed the dead `local_path` field from `ModelConfig`
  — it was a leftover from the old GGUF/llama.cpp architecture and was always
  empty. Model selection now uses `mlx_model_name` exclusively
- **Vestigial `path` field** (`lib.rs`, `commands/mod.rs`,
  `settings/+page.svelte`): removed dead `path` field from `DownloadedModel`
  — was always `String::new()` and never read by any frontend logic
- **`validate_model` parameter mismatch** (`commands/mod.rs`): renamed
  `model_path` → `model_name` to match the frontend's `invoke('validate_model',
  { modelName })` call — the old parameter name caused Tauri to silently drop the
  argument since camelCase→snake_case mapping didn't match

### Changed

- Migrated to rapid-mlx as the sole inference backend — replaced all GGUF/llama.cpp
  references with `rapid-mlx serve` subprocess + OpenAI-compatible HTTP API
- CI workflow now requires status checks for all GitHub-hosted runner jobs before merge
  (Frontend, Rust + Tests for Linux, macOS ARM, macOS Intel, Windows)

### Documented

- Audit of backend documentation (`docs/backend/`, `docs/api/`, `docs/architecture/`,
  `docs/testing/`, `docs/project-overview.md`): corrected module structure, file paths,
  line counts, struct definitions, and command signatures to match the actual codebase
- `docs/backend/inference.md`: removed references to non-existent `PipelineRunner`,
  `Deconstructor`, `inference/prompts/`, and `inference/schemas/`; updated to reflect
  `MloxPipeline` + `Reasoner` implementation
- `docs/backend/extractors.md`: removed references to non-existent `Deconstructor`,
  `ocr.rs`, `document.rs`, `metadata.rs`, `structured.rs` extractors; updated to match
  actual `extract_pdf`/`extract_image`/`extract_docx`/`extract_audio` functions
- `docs/backend/overview.md`: updated module structure from 7+ modules to actual 4-module
  layout (core, extractors, inference, commands); corrected `AppState` definition
- `docs/api/tauri-commands.md`: removed `get_builtin_pipelines` (internal function, not a
  registered command), corrected command count from 68 to 69, fixed `validate_model`
  parameter name (`model_path` → `model_name`)
- `docs/backend/database.md`: corrected from dual-DB (3397 lines) to single shared
  SQLite pool (317 lines) with embedded migrations
- `docs/backend/config.md`: fixed `source` field description to match TypeScript interface
  (`"huggingface"` or `"local"`); removed `local_path` row
- `docs/backend/overview.md`: corrected command count from "60+" to "69"
- `docs/backend/inference.md`: corrected `extract_facts` description (no text chunking,
  JSON parsing, or deduplication — prompt + single infer() call + Fact wrapping)
- `docs/backend/extractors.md`: corrected dispatch claim — `extract_metadata_from_path`
  does not route to extractors; dispatching is via private `extract_file` helper in
  `commands/mod.rs`
- `docs/architecture/pipeline.md`: fixed Stage 2 diagram (removed chunk/dedup/score steps),
  corrected Fact Structure table to match actual `Fact` struct fields, fixed `MloxPipeline`
  typo, corrected system prompt description

## [0.3.2] - 2026-04-30

### Added

- AGENTS.md developer reference (build/test commands, architecture notes, key patterns)
- `src/lib/utils.ts` shared utility module (`getSeverityColor`, `getCategoryIcon`,
  `getQualityBadgeColor`, `formatFileSize`) — all using CSS custom properties

### Changed

- `aria-current="page"` added to active nav items for accessibility (F-MED-003)
- Svelte 5 `$state` Set mutation in metadata page now uses reassignment for
  correct reactivity (F-MED-001)
- Removed duplicate local `getSeverityColor` / `getCategoryIcon` /
  `getQualityBadgeColor` functions from `results`, `quality`, `timeline`, and
  `maps` pages — all now import from `$lib/utils`
- Removed redundant `setInterval(refreshWorkflow, 2000)` from results page;
  `analysis_progress` event listener is sufficient (F-HIGH-003)

### Fixed

- **Quality score bug** (`extraction.rs`): quality field was always 0/1 (cast
  from `is_partial`); now computed as a real heuristic (word-density × 0.6 +
  length score × 0.4, capped at 0.8 for partial chunks) (B-CRIT-003)
- **Llama lock unwrap** (`llama.rs`): replaced `.lock().unwrap()` in hot
  inference path with `map_err` to avoid poisoning panics (B-CRIT-002)
- **Pipeline naked unwrap** (`pipeline.rs`): `parsed.as_array().unwrap()`
  replaced with a safe `if let` (B-CRIT-001)
- **Analytics cache lock unwraps** (`queries/analytics.rs`): all four
  `.lock().unwrap()` calls replaced with `if let Ok` guards (B-HIGH-003)
- **GPS timeline unwrap** (`queries/timeline.rs`): regex capture `.unwrap()`
  replaced with destructured `if let` (B-HIGH-006)
- **`require_db()` helper** (`commands/mod.rs`): extracted to replace ~72
  instances of manual lock/check boilerplate across 14 command files (B-HIGH-002)

---

## [0.3.1] - 2026-04-28

### Added

- FR-META: Metadata extraction (EXIF from images, PDF document properties)
- Metadata UI page: file selector, cached/live extraction toggle, raw/parsed view
- FR-LANG: Language detection with whatlang crate
- FR-STRUCT: Structured data extraction (key-value pairs, PDF form fields)

### Changed

- Performance improvements: Streaming PDF extraction for large files, parallel app initialization

### Fixed

- Inference improvements: Proper n_threads_batch wiring

---

## [0.3.0] - 2026-04-23

### Added

- Gemma 3 model support with enhanced fact extraction
- Audio transcription with whisper CLI integration
- Hardware auto-scaling with sysinfo detection
- Metal GPU acceleration for Apple Silicon
- Two-stage pipeline (extract then analyze)
- Extraction statistics panel
- Workflow state persistence across page navigation

### Fixed

- tokio runtime panic (converted blocking HTTP to async)
- Critical parallel processing race conditions
- JSON parsing in LLM responses
- Timeout and hash issues
- Model selection persistence
- Settings page HTML structure
- Gemma 3 prompt and stop button

### Changed

- Remove emojis from UI for professional appearance
- Simplify Settings page (removed manual performance parameters)
- Update mise config for GitHub workflow compatibility

### Security

- Remove unused shell:default Tauri permission

### Dependencies

- Update to Tauri 2.x stable
- Update SvelteKit to 2.x

### Infrastructure

- CI: platform-specific pipelines (Linux, macOS ARM/Intel, Windows)
- CI: self-hosted runners (Fedora, NixOS)
- Automated multi-platform release artifacts

---

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
