# Backend Overview

## Module Structure

The Rust backend is organized into 7 core modules:

```
src-tauri/src/
├── main.rs          # Entry point (Windows subsystem guard)
├── lib.rs           # Library root + Tauri command hub (60+ commands)
├── core/            # Database + Registry
│   ├── mod.rs
│   ├── database.rs  # SQLite operations (3397 lines)
│   └── registry.rs  # File fingerprinting (223 lines)
├── extractors/      # Text extraction
│   ├── mod.rs
│   ├── pdf.rs       # PDF extraction (278 lines)
│   ├── ocr.rs       # OCR extraction (310 lines)
│   ├── audio.rs     # Audio transcription (163 lines)
│   ├── document.rs  # DOCX/TXT parsing (214 lines)
│   └── deconstructor.rs # Unified orchestrator (209 lines)
├── inference/       # LLM reasoning
│   ├── mod.rs
│   ├── llama.rs     # LLM wrapper (103 lines)
│   ├── pipeline.rs  # Multi-pass pipeline (355 lines)
│   ├── reasoner.rs  # Neural reasoner (380 lines)
│   ├── prompts/     # Prompt templates
│   └── schemas/     # JSON output schemas
├── inference/quality/
│   ├── mod.rs
│   ├── scoring.rs   # Quality metrics (131 lines)
│   └── deduplication.rs # Fact dedup (296 lines)
├── gpu/             # Hardware detection
│   ├── mod.rs
│   ├── detect.rs    # Hardware detection (200 lines)
│   └── backend.rs   # GPU backend enum (33 lines)
├── config/          # Configuration
│   ├── mod.rs
│   ├── model.rs     # App config (154 lines)
│   ├── project.rs   # Project file (149 lines)
│   └── settings.rs  # Settings helpers (13 lines)
├── models/          # Model management
│   └── mod.rs       # ModelManager (208 lines)
└── utils/           # Utilities
    ├── mod.rs
    ├── files.rs     # File utilities (204 lines)
    ├── logging.rs   # Structured logging (53 lines)
    └── paths.rs     # Path helpers (30 lines)
```

## Entry Points

### main.rs

Windows subsystem entry point. Calls `steinline_lib::run()` to start the Tauri application.

### lib.rs

The main library and Tauri command hub (~1572 lines). Responsibilities:

- Declares all 7 modules
- Defines `AppState` (config, database, registry worker, reasoner)
- Registers 60+ Tauri commands for frontend communication
- Sets up the Tauri application builder

## AppState

```rust
struct AppState {
    config: Mutex<AppConfig>,
    db: Mutex<Database>,
    registry_worker: Mutex<Option<RegistryWorker>>,
    reasoner: Mutex<Option<Reasoner>>,
}
```

The `AppState` is shared across all Tauri commands via `Arc<Mutex<>>` and provides access to:

- Configuration management
- Database connections
- Registry scanning
- LLM reasoning

## Tauri Commands

The backend exposes 60+ commands to the frontend. See [Tauri Commands](../api/tauri-commands.md) for the complete list.

### Command Categories

| Category             | Count | Examples                                             |
| -------------------- | ----- | ---------------------------------------------------- |
| Config/Project       | 7     | `load_config`, `create_project`, `save_project`      |
| Hardware             | 3     | `detect_hardware`, `get_system_monitor`              |
| Registry             | 4     | `init_project`, `start_registry`, `get_stats`        |
| Search               | 4     | `search_facts`, `search_entities`, `search_combined` |
| Analysis             | 12    | `get_timeline_events`, `detect_anomalies`            |
| Tags/Annotations     | 7     | `add_tag`, `add_annotation`                          |
| Export               | 8     | `export_facts_json`, `export_pdf_report`             |
| Comparison           | 2     | `compare_projects`, `get_project_summary`            |
| Backup               | 2     | `create_backup`, `restore_backup`                    |
| Models               | 4     | `download_model`, `list_downloaded_models`           |
| Extraction/Reasoning | 5     | `extract_file`, `analyze_file`                       |
| Notifications        | 1     | `send_notification`                                  |
