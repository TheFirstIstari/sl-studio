# Backend Overview

## Module Structure

The Rust backend is organized into 4 core modules under `src-tauri/src/`:

```
src-tauri/src/
├── main.rs          # Entry point (6 lines)
├── lib.rs           # Public types + Tauri command hub (~767 lines)
├── app.rs           # Tauri setup callback (~13 lines)
├── commands/mod.rs  # All #[tauri::command] handlers (~1853 lines)
├── core/            # Database (SQLite pool + migrations)
│   ├── mod.rs       # Module re-export
│   └── database.rs  # SQLite connection pool + run_migrations (~317 lines)
├── extractors/      # Text extraction (PDF, DOCX, OCR images, audio)
│   ├── mod.rs       # extract_metadata_from_path + module declarations (~70 lines)
│   ├── audio.rs     # Audio metadata extraction (~22 lines)
│   ├── docx.rs      # DOCX text extraction (~22 lines)
│   ├── image.rs     # OCR image text extraction (~22 lines)
│   └── pdf.rs       # PDF text extraction (~22 lines)
└── inference/       # MLX pipeline + reasoner
    ├── mod.rs       # get_builtin_pipelines (~80 lines)
    ├── mlx_pipeline.rs # rapid-mlx subprocess wrapper (~91 lines)
    └── reasoner.rs  # Fact extraction via LLM (~38 lines)
```

## Entry Points

### main.rs

Windows subsystem entry point. Calls `steinline_lib::run()` to start the Tauri application.

### lib.rs

The main library root (~767 lines). Responsibilities:

- Declares all 4 modules (`commands`, `core`, `extractors`, `inference`)
- Defines public types (`AppConfig`, `ModelConfig`, `Fact`, `Chain`, etc.)
- Defines `AppState` with database pool, in-memory stores, and MLX reasoner
- Defines `require_db()` singleton accessor for the SQLite pool
- Registers 60+ Tauri commands for frontend communication via `tauri::generate_handler![...]`

## AppState

```rust
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<core::database::Pool>,
    pub metadata: HashMap<String, Metadata>,
    pub facts: HashMap<String, Fact>,
    pub chains: HashMap<String, Chain>,
    pub file_results: HashMap<String, FileResult>,
    pub reasoner: Arc<Mutex<Option<inference::reasoner::Reasoner>>>,
}
```

The `AppState` is shared across all Tauri commands via `tauri::State` and provides access to:

- Database connections (SQLite pool)
- In-memory caches for metadata, facts, chains, and file results
- The MLX inference reasoner (initialized on demand via `init_reasoner`)

## Tauri Commands

The backend exposes 60+ commands to the frontend, all located in `commands/mod.rs`. See [Tauri Commands](../api/tauri-commands.md) for the complete list.

### Command Categories

| Category             | Commands                                              | Description                          |
| -------------------- | ----------------------------------------------------- | ------------------------------------ |
| Config/Project       | `load_config`, `save_config`, `init_project`          | Application configuration            |
| Registry             | `start_registry`, `get_extraction_queue`              | File fingerprinting & registry       |
| Extraction           | `extract_batch`, `get_extraction_statistics`, `get_analysis_queue` | Text extraction pipeline |
| Facts                | `search_facts`, `export_facts_json`, `delete_facts`   | Fact search & export                 |
| Entities             | `suggest_entity_matches`, `get_entity_relationships`  | Entity management & relationships    |
| Evidence Chains      | `list_evidence_chains`, `create_evidence_chain`        | Chain management                     |
| Facets               | `list_facet_presets`, `save_facet_preset`             | Search facet presets                 |
| Pipelines            | `list_pipelines`, `save_pipeline`, `delete_pipeline`  | Analysis pipeline management         |
| Quality              | `find_duplicate_facts`, `merge_duplicate_facts`       | Deduplication & validation           |
| Timeline             | `get_timeline_events`                                 | Chronological fact ordering          |
| Metadata             | `get_registry_files`, `get_cached_metadata`, `extract_metadata` | File & metadata management |
| Stats                | `get_stats`, `get_overall_statistics`                 | Aggregate statistics                 |
| Hardware/Model       | `detect_hardware`, `download_model`, `init_reasoner`  | System detection & inference setup   |
| Analysis             | `analyze_batch`, `set_cancel_flag`, `get_workflow_state` | LLM analysis & workflow     |
| Compare              | `get_project_summary`, `compare_projects`             | Cross-project comparison             |
| Utility              | `write_file`, `create_backup`, `restore_backup`       | File I/O & backup/restore            |
