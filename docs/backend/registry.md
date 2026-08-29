# Registry Scanner

## Overview

The registry scanner is implemented as the `start_registry` Tauri command in
`commands/mod.rs`. There is no separate `core/registry.rs` module — the
scanning logic uses `std::fs::read_dir` to walk the evidence root directory and
inserts records into the `registry` table via the shared SQLite pool.

## Registry Commands

| Command                | Parameters | Returns              | Description                          |
| ---------------------- | ---------- | -------------------- | ------------------------------------ |
| `start_registry`       | None       | `i64`                | Scan evidence dir, return file count |
| `get_extraction_queue` | `limit`    | `Vec<RegistryFile>`  | Files needing extraction             |
| `get_analysis_queue`   | `limit`    | `Vec<RegistryFile>`  | Files needing LLM analysis           |
| `get_registry_files`   | `limit`    | `Vec<RegistryEntry>` | Paginated registry listing           |

## Process

```
Evidence Directory
       │
       ▼
┌─────────────┐
│ Walk Dir    │ ← std::fs::read_dir (one level)
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Fingerprint │ ← Format hash of path string
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Batch Insert│ ← INSERT OR IGNORE into registry table
└──────┬──────┘
       │
       ▼
   File Count (i64)
```

## Registry Table Schema

| Column                | Type     | Description                     |
| --------------------- | -------- | ------------------------------- |
| `id`                  | INTEGER  | Primary key (autoincrement)     |
| `fingerprint`         | TEXT     | Unique hash (path-based)        |
| `path`                | TEXT     | Full file path                  |
| `file_size`           | INTEGER  | File size in bytes              |
| `file_type`           | TEXT     | File extension                  |
| `file_name`           | TEXT     | Filename                        |
| `last_modified`       | DATETIME | Last modified timestamp         |
| `has_extracted_text`  | BOOLEAN  | Whether text extraction is done |
| `extracted_at`        | DATETIME | Extraction timestamp            |
| `processed`           | BOOLEAN  | Whether LLM analysis is done    |
| `processing_priority` | INTEGER  | Priority (0 = default)          |
| `retry_count`         | INTEGER  | Extraction retry count          |
| `extraction_quality`  | REAL     | Quality score                   |
| `created_at`          | DATETIME | Record creation timestamp       |

## Queue Queries

### get_extraction_queue

Selects files where `has_extracted_text = FALSE`, ordered by `processing_priority DESC, created_at ASC`.

### get_analysis_queue

Selects files where `processed = FALSE`, ordered by priority.
