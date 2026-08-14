# Tauri Commands

## Overview

The backend exposes 68 commands to the frontend via Tauri's IPC mechanism. All
commands are registered in `lib.rs` via `tauri::generate_handler![...]` and
implemented in `commands/mod.rs`.

Commands use `snake_case` and return `Result<T>` (the `crate::Result` type
backed by `AppError` for serializable errors).

## Config / Project Commands

| Command         | Parameters   | Returns       | Description                    |
| --------------- | ------------ | ------------- | ------------------------------ |
| `load_config`   | None         | `AppConfig`   | Load config from `sl-studio-config.json` |
| `save_config`   | `AppConfig`  | `Result<()>`  | Persist config to disk         |
| `init_project`  | `AppConfig`  | `Result<()>`  | Initialize project with evidence dir |

## Registry / Extraction Commands

| Command                     | Parameters        | Returns                    | Description                          |
| --------------------------- | ----------------- | -------------------------- | ------------------------------------ |
| `start_registry`            | None              | `Result<i64>`              | Scan evidence dir, return file count |
| `get_extraction_queue`      | `limit: usize`    | `Result<Vec<RegistryFile>>` | Files needing extraction             |
| `get_analysis_queue`        | `limit: usize`    | `Result<Vec<RegistryFile>>` | Files needing LLM analysis           |
| `extract_batch`             | `fingerprints: Vec<String>, cpu_workers: usize` | `Result<Vec<ExtractionResult>>` | Extract text from files |
| `get_extraction_statistics` | None              | `Result<ExtractionStats>`  | Extraction stats summary             |

## Fact Commands

| Command                | Parameters                              | Returns             | Description                |
| ---------------------- | --------------------------------------- | ------------------- | -------------------------- |
| `search_facts`         | `query: String, limit: usize`           | `Result<Vec<Fact>>` | Search facts (LIKE query)  |
| `export_facts_json`    | `min_weight, limit, categories, start_date, end_date` | `Result<String>` | Export facts as JSON       |
| `export_facts_csv`     | `min_weight: f64, limit: usize`         | `Result<String>`    | Export facts as CSV        |
| `export_entities_csv`  | `min_weight: f64, limit: usize`         | `Result<String>`    | Export entities as CSV     |
| `export_timeline_json` | `min_weight, limit, categories, start_date, end_date` | `Result<String>` | Export timeline as JSON    |
| `export_full_report_json` | None                                  | `Result<String>`    | Full report as JSON        |
| `export_pdf_report`    | None                                    | `Result<Vec<u8>>`   | Export PDF report          |
| `export_excel_data`    | None                                    | `Result<String>`    | Export Excel data          |
| `delete_facts`         | `ids: Vec<u64>`                         | `Result<()>`        | Soft-delete facts          |
| `update_fact_verification` | `id: i64, status: String, review_notes: Option<String>` | `Result<()>` | Update verification status |

## Entity Commands

| Command                     | Parameters                                      | Returns                          | Description                    |
| --------------------------- | ----------------------------------------------- | -------------------------------- | ------------------------------ |
| `suggest_entity_matches`    | `canonical_id: i64, threshold: f64`            | `Result<Vec<EntityMatchSuggestion>>` | Find entity aliases     |
| `add_entity_alias`          | `canonical_id: i64, alias_id: i64`              | `Result<()>`                     | Link entity alias              |
| `get_entity_relationships`  | `min_cooccurrence: i64, limit: usize`          | `Result<Vec<EntityRelationship>>` | Entity co-occurrence graph |
| `get_connected_entities`    | `entity_id: i64, max_depth: i64`               | `Result<Vec<ConnectedEntity>>`  | BFS entity traversal          |
| `detect_entity_communities` | None                                            | `Result<Vec<EntityCommunity>>`  | Community detection          |
| `compute_betweenness_centrality` | None                                         | `Result<Vec<EntityBetweenness>>` | Betweenness centrality       |
| `get_location_entities`     | `min_confidence: f64`                           | `Result<Vec<LocationEntity>>`   | Geographic entities for maps  |
| `get_entity_centrality`     | `limit: usize`                                  | `Result<Vec<EntityCentrality>>` | Entity network centrality     |

## Evidence Chain Commands

| Command                   | Parameters                                      | Returns             | Description                    |
| ------------------------- | ----------------------------------------------- | ------------------- | ------------------------------ |
| `list_evidence_chains`    | `limit: usize, offset: usize`                   | `Result<Vec<ChainSummary>>` | List all chains          |
| `create_evidence_chain`   | `chain_name, chain_type, description`           | `Result<i64>`       | Create a new chain             |
| `get_evidence_chain`      | `chain_id: i64`                                 | `Result<Option<EvidenceChain>>` | Get full chain detail  |
| `delete_evidence_chain`   | `chain_id: i64`                                 | `Result<()>`        | Delete a chain                 |
| `add_to_evidence_chain`   | `chain_id, intelligence_id, relationship_type, notes` | `Result<()>` | Add fact to chain   |
| `remove_from_evidence_chain` | `chain_id: i64, intelligence_id: i64`         | `Result<()>`        | Remove fact from chain         |

## Facet Commands

| Command              | Parameters                    | Returns                 | Description                    |
| -------------------- | ----------------------------- | ----------------------- | ------------------------------ |
| `list_facet_presets` | `page: String`                | `Result<Vec<FacetPreset>>` | List saved facet presets  |
| `save_facet_preset`  | `page, name, state_json`      | `Result<()>`            | Save a facet preset            |
| `delete_facet_preset`| `preset_id: i64`              | `Result<()>`            | Delete a facet preset          |

## Pipeline Commands

| Command             | Parameters        | Returns              | Description                    |
| ------------------- | ----------------- | -------------------- | ------------------------------ |
| `list_pipelines`    | None              | `Result<Vec<Pipeline>>` | List all pipelines           |
| `save_pipeline`     | `Pipeline`        | `Result<()>`         | Save or update a pipeline      |
| `delete_pipeline`   | `pipeline_id: String` | `Result<()>`      | Delete a pipeline              |
| `get_builtin_pipelines` | None          | `Vec<Pipeline>`      | Built-in pipeline definitions  |

## Quality Commands

| Command                 | Parameters                                     | Returns                   | Description                    |
| ----------------------- | ---------------------------------------------- | ------------------------- | ------------------------------ |
| `find_duplicate_facts`  | `threshold, require_same_category, require_same_date` | `Result<Vec<DuplicateGroup>>` | Find similar facts  |
| `merge_duplicate_facts` | `keeper_id, member_ids`                        | `Result<i64>`             | Merge duplicates               |
| `cross_validate_fact`   | `intelligence_id, threshold`                  | `Result<CrossValidationResult>` | Cross-validate a fact |
| `get_evidence_weight`   | `intelligence_id`                              | `Result<f64>`             | Weighted evidence score        |
| `detect_anomalies`      | `metric, threshold_std`                        | `Result<Vec<Anomaly>>`    | Z-score anomaly detection      |

## Timeline Commands

| Command            | Parameters                              | Returns                  | Description                    |
| ------------------ | --------------------------------------- | ------------------------ | ------------------------------ |
| `get_timeline_events` | `min_weight, limit, categories, start_date, end_date` | `Result<Vec<TimelineEvent>>` | Chronological facts |

## Metadata Commands

| Command               | Parameters              | Returns                              | Description                          |
| --------------------- | ----------------------- | ------------------------------------ | ------------------------------------ |
| `get_registry_files`  | `limit: usize`          | `Result<Vec<RegistryEntry>>`         | Paginated registry listing           |
| `get_cached_metadata` | `fingerprint: String`   | `Result<Option<DocumentMetadata>>`   | Get cached metadata from DB          |
| `extract_metadata`    | `path: String`          | `Result<DocumentMetadata>`           | Extract metadata from file (no DB)   |
| `cache_metadata`      | `fingerprint, path`     | `Result<DocumentMetadata>`           | Extract and cache metadata           |

## Statistics Commands

| Command                      | Parameters | Returns                          | Description                    |
| ---------------------------- | ---------- | -------------------------------- | ------------------------------ |
| `get_stats`                  | None       | `Result<ProjectStats>`           | Registry & intelligence stats  |
| `get_overall_statistics`     | None       | `Result<OverallStats>`           | Aggregate statistics summary   |
| `get_category_distribution`  | None       | `Result<Vec<CategoryStat>>`      | Facts by category              |
| `get_severity_distribution`  | None       | `Result<Vec<SeverityStat>>`      | Facts by severity              |

## Hardware / Model Commands

| Command                  | Parameters                    | Returns                   | Description                    |
| ------------------------ | ----------------------------- | ------------------------- | ------------------------------ |
| `detect_hardware`        | None                          | `Result<HardwareStatus>`  | Detect CPU/RAM/GPU             |
| `get_hardware_info`      | None                          | `Result<HardwareInfoExt>` | Detailed info for settings     |
| `get_recommended_settings` | None                        | `Result<HardwareInfo>`    | Auto-scaled LLM params         |
| `get_system_monitor`     | None                          | `Result<SystemMonitor>`   | Real-time CPU/memory snapshot  |
| `list_downloaded_models` | None                          | `Result<Vec<DownloadedModel>>` | List MLX models      |
| `download_model`         | `repo_id, filename`           | `Result<DownloadedModel>` | Pull model via rapid-mlx       |
| `is_model_loaded`        | None                          | `Result<bool>`            | Check if model is loaded       |
| `validate_model`         | `model_path`                  | `Result<bool>`            | Validate model name or path    |

## Analysis Commands

| Command             | Parameters                              | Returns         | Description                    |
| ------------------- | --------------------------------------- | --------------- | ------------------------------ |
| `init_reasoner`     | `model_name, context_size`              | `Result<()>`    | Start rapid-mlx subprocess + store Reasoner |
| `analyze_batch`     | `fingerprints: Vec<String>`             | `Result<()>`    | Run LLM inference on extracted text |
| `set_cancel_flag`   | `cancel: bool`                          | `Result<()>`    | Cancel ongoing analysis        |

## Workflow Commands

| Command            | Parameters | Returns            | Description                    |
| ------------------ | ---------- | ------------------ | ------------------------------ |
| `get_workflow_state` | None     | `Result<WorkflowState>` | Current workflow progress  |

## Compare Commands

| Command              | Parameters             | Returns                | Description                    |
| -------------------- | ---------------------- | ---------------------- | ------------------------------ |
| `get_project_summary`| None                   | `Result<ProjectSummary>` | Project summary             |
| `compare_projects`   | `project2_path: String` | `Result<ProjectComparison>` | Cross-project comparison |

## Utility Commands

| Command            | Parameters          | Returns           | Description                    |
| ------------------ | ------------------- | ----------------- | ------------------------------ |
| `write_file`       | `path, contents`    | `Result<()>`      | Write file to disk             |
| `create_backup`    | `include_evidence`  | `Result<BackupResult>` | Create ZIP backup           |
| `restore_backup`   | `backup_path`       | `Result<()>`      | Restore from backup            |
