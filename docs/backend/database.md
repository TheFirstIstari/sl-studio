# Database Layer

## Overview

The database layer (`core/database.rs`, ~317 lines) manages a single SQLite
connection pool shared across all Tauri commands. There is no separate
"Registry DB" and "Intelligence DB" — all tables live in one database file,
created and migrated on connection.

## Structure

```rust
pub struct Pool {
    conn: Arc<Mutex<Connection>>,
}
```

The `Pool` is wrapped in `Arc<Mutex<Connection>>` to allow safe sharing across
async Tauri command handlers. It exposes `execute` and `query_map` methods.

## Connection

The pool is initialised once via `OnceLock` in `lib.rs` (`require_db()`),
connecting to `sl-studio.db` in the current working directory. Migrations run
on first connection via `run_migrations`.

## Tables

All tables are created with `CREATE TABLE IF NOT EXISTS`:

### Intelligence

| Column              | Type    | Description                          |
| ------------------- | ------- | ------------------------------------ |
| `id`                | INTEGER | Primary key (autoincrement)          |
| `fingerprint`       | TEXT    | File fingerprint                     |
| `filename`          | TEXT    | Source file name                     |
| `fact_summary`      | TEXT    | Extracted fact                       |
| `category`          | TEXT    | Crime/fact category                  |
| `identified_crime`  | TEXT    | Specific crime type                  |
| `severity_score`    | INTEGER | 0–10 severity rating                 |
| `confidence`        | REAL    | Confidence (0.0–1.0)                 |
| `quality_score`     | REAL    | Extraction quality                   |
| `source_quote`      | TEXT    | Direct quote from source             |
| `associated_date`   | TEXT    | Associated date                      |
| `is_deleted`        | BOOLEAN | Soft-delete flag                     |
| `deleted_at`        | DATETIME| Deletion timestamp                   |
| `verification_status` | TEXT  | `unverified` / verified              |
| `review_notes`      | TEXT    | User review notes                    |
| `created_at`        | DATETIME| Creation timestamp                   |
| `updated_at`        | DATETIME| Last update timestamp                |

### Registry

Tracks scanned files with fingerprint, path, type, size, and processing status.

### Text Cache

Stores extracted text keyed by fingerprint, with quality scoring and timestamps.

### Evidence Chains & Chain Items

Evidence chain groups with many-to-many links to intelligence facts via
`chain_items`.

### Entities & Aliases

Named entity extraction results with alias resolution for deduplication.

### Facet Presets

User-saved search facet configurations per page.

### Pipelines

Configurable multi-pass analysis pipelines stored as JSON.

### System Tables

`file_metadata_cache`, `fact_validations`, `evidence_weights`, `audit_log`.

## Migrations

Defined inline in `get_migrations()`, executed sequentially on connection.
Each migration uses `CREATE TABLE IF NOT EXISTS` for idempotency.
