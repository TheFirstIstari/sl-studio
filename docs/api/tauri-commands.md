# Tauri Commands

## Overview

The backend exposes 60+ commands to the frontend via Tauri's IPC mechanism. All commands are registered in `lib.rs`.

## Config/Project Commands

| Command               | Parameters    | Returns               | Description                    |
| --------------------- | ------------- | --------------------- | ------------------------------ |
| `load_config`         | None          | `AppConfig`           | Load application configuration |
| `save_config`         | `AppConfig`   | `Result<()>`          | Save application configuration |
| `validate_config`     | `AppConfig`   | `Result<()>`          | Validate configuration values  |
| `create_project`      | `ProjectFile` | `Result<()>`          | Create new project             |
| `load_project`        | `PathBuf`     | `Result<ProjectFile>` | Load existing project          |
| `save_project`        | `ProjectFile` | `Result<()>`          | Save project file              |
| `get_default_project` | None          | `ProjectFile`         | Get default project template   |

## Hardware Commands

| Command                    | Parameters | Returns               | Description                       |
| -------------------------- | ---------- | --------------------- | --------------------------------- |
| `detect_hardware`          | None       | `HardwareInfo`        | Detect CPU/GPU/memory             |
| `get_system_monitor`       | None       | `SystemMetrics`       | Real-time system metrics          |
| `get_processing_stats`     | None       | `ProcessingStats`     | Current processing statistics     |
| `get_hardware_info`        | None       | `HardwareInfo`        | Get current hardware capabilities |
| `get_recommended_settings` | None       | `RecommendedSettings` | Get auto-scaled processing params |

## Registry Commands

| Command                 | Parameters | Returns              | Description                          |
| ----------------------- | ---------- | -------------------- | ------------------------------------ |
| `init_project`          | `PathBuf`  | `Result<()>`         | Initialize project with evidence dir |
| `start_registry`        | None       | `Result<()>`         | Start registry scanning              |
| `get_stats`             | None       | `RegistryStats`      | Get registry statistics              |
| `get_unprocessed_files` | None       | `Vec<RegistryEntry>` | List files needing processing        |
| `mark_processed`        | `String`   | `Result<()>`         | Mark file as processed               |

## Search Commands

| Command           | Parameters       | Returns               | Description                      |
| ----------------- | ---------------- | --------------------- | -------------------------------- |
| `search_facts`    | `query, filters` | `Vec<SearchResult>`   | Search facts with FTS5           |
| `search_entities` | `query, filters` | `Vec<EntityResult>`   | Search entities with FTS5        |
| `search_combined` | `query`          | `Vec<CombinedResult>` | Combined facts + entities search |
| `search_by_tags`  | `tags`           | `Vec<SearchResult>`   | Filter facts by tags             |

## Analysis Commands

| Command                     | Parameters  | Returns                 | Description                |
| --------------------------- | ----------- | ----------------------- | -------------------------- |
| `get_timeline_events`       | None        | `Vec<TimelineEvent>`    | Chronological fact list    |
| `get_overall_statistics`    | None        | `Statistics`            | Overall stats summary      |
| `get_category_distribution` | None        | `Vec<CategoryCount>`    | Facts by category          |
| `get_severity_distribution` | None        | `Vec<SeverityCount>`    | Facts by severity          |
| `get_entity_centrality`     | None        | `Vec<EntityCentrality>` | Network centrality scores  |
| `detect_anomalies`          | `threshold` | `Vec<Anomaly>`          | Z-score anomaly detection  |
| `get_weighted_evidence`     | None        | `Vec<WeightedEvidence>` | Confidence-ranked evidence |
| `get_entity_relationships`  | None        | `Vec<Relationship>`     | Entity relationship graph  |
| `get_connected_entities`    | `entity_id` | `Vec<Entity>`           | Connected entities         |
| `get_location_entities`     | None        | `Vec<LocationEntity>`   | Geographic entities        |

## Tags/Annotations Commands

| Command             | Parameters            | Returns           | Description              |
| ------------------- | --------------------- | ----------------- | ------------------------ |
| `add_tag`           | `fact_id, tag`        | `Result<()>`      | Add tag to fact          |
| `remove_tag`        | `fact_id, tag`        | `Result<()>`      | Remove tag from fact     |
| `get_all_tags`      | None                  | `Vec<String>`     | List all tags            |
| `add_annotation`    | `fact_id, annotation` | `Result<()>`      | Add annotation           |
| `update_annotation` | `id, annotation`      | `Result<()>`      | Update annotation        |
| `delete_annotation` | `id`                  | `Result<()>`      | Delete annotation        |
| `get_annotations`   | `fact_id`             | `Vec<Annotation>` | Get annotations for fact |

## Export Commands

| Command                   | Parameters      | Returns           | Description             |
| ------------------------- | --------------- | ----------------- | ----------------------- |
| `export_facts_json`       | `filters`       | `Result<String>`  | Export facts as JSON    |
| `export_facts_csv`        | `filters`       | `Result<String>`  | Export facts as CSV     |
| `export_entities_csv`     | None            | `Result<String>`  | Export entities as CSV  |
| `export_timeline_json`    | None            | `Result<String>`  | Export timeline as JSON |
| `export_full_report_json` | None            | `Result<String>`  | Full report as JSON     |
| `export_pdf_report`       | `filters`       | `Result<PathBuf>` | Export PDF report       |
| `export_excel_data`       | `filters`       | `Result<PathBuf>` | Export Excel workbook   |
| `write_file`              | `path, content` | `Result<()>`      | Write file to disk      |

## Comparison Commands

| Command               | Parameters             | Returns            | Description          |
| --------------------- | ---------------------- | ------------------ | -------------------- |
| `compare_projects`    | `project_a, project_b` | `ComparisonResult` | Compare two projects |
| `get_project_summary` | `project_path`         | `ProjectSummary`   | Get project summary  |

## Backup Commands

| Command          | Parameters         | Returns           | Description         |
| ---------------- | ------------------ | ----------------- | ------------------- |
| `create_backup`  | `include_evidence` | `Result<PathBuf>` | Create ZIP backup   |
| `restore_backup` | `backup_path`      | `Result<()>`      | Restore from backup |

## Model Commands

| Command                  | Parameters               | Returns          | Description               |
| ------------------------ | ------------------------ | ---------------- | ------------------------- |
| `get_models_dir`         | None                     | `PathBuf`        | Get models directory path |
| `get_huggingface_models` | None                     | `Vec<ModelInfo>` | List available HF models  |
| `download_model`         | `model_id, quantization` | `Result<()>`     | Download model from HF    |
| `list_downloaded_models` | None                     | `Vec<ModelInfo>` | List local models         |

## Extraction/Reasoning Commands

| Command                    | Parameters   | Returns                    | Description               |
| -------------------------- | ------------ | -------------------------- | ------------------------- |
| `extract_file`             | `file_path`  | `Result<ExtractionResult>` | Extract text from file    |
| `extract_batch`            | `file_paths` | `Vec<ExtractionResult>`    | Extract multiple files    |
| `get_supported_extensions` | None         | `Vec<String>`              | List supported file types |
| `init_reasoner`            | `model_path` | `Result<()>`               | Initialize LLM reasoner   |
| `analyze_file`             | `file_path`  | `Result<AnalysisResult>`   | Full file analysis        |
| `analyze_batch`            | `file_paths` | `Vec<AnalysisResult>`      | Analyze multiple files    |
| `get_extraction_queue`     | None         | `QueueStatus`              | Get extraction queue      |
| `get_analysis_queue`       | None         | `QueueStatus`              | Get analysis queue        |
| `is_model_loaded`          | None         | `bool`                     | Check if model is loaded  |
| `get_reasoner_config`      | None         | `ReasonerConfig`           | Get reasoner settings     |

## Notification Commands

| Command             | Parameters    | Returns      | Description              |
| ------------------- | ------------- | ------------ | ------------------------ |
| `send_notification` | `title, body` | `Result<()>` | Send system notification |
