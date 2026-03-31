# Database Schema Reference

## Overview

SL Studio uses two SQLite databases:

- **Registry DB**: Tracks files, extracted text, and metadata
- **Intelligence DB**: Stores facts, entities, annotations, and analysis results

## Registry Database

### registry

Tracks all evidence files and their processing status.

| Column        | Type                 | Description                         |
| ------------- | -------------------- | ----------------------------------- |
| `id`          | INTEGER PRIMARY KEY  | Auto-increment ID                   |
| `path`        | TEXT NOT NULL        | File path                           |
| `fingerprint` | TEXT NOT NULL UNIQUE | SHA-256 hash                        |
| `file_type`   | TEXT                 | File type (pdf, image, audio, etc.) |
| `size`        | INTEGER              | File size in bytes                  |
| `modified_at` | INTEGER              | Last modified timestamp             |
| `processed`   | BOOLEAN              | Whether file has been processed     |
| `priority`    | INTEGER              | Processing priority (0-3)           |
| `retry_count` | INTEGER              | Number of retry attempts            |
| `quality`     | REAL                 | Extraction quality score            |

### text_cache

Stores extracted text for each file.

| Column      | Type                | Description             |
| ----------- | ------------------- | ----------------------- |
| `id`        | INTEGER PRIMARY KEY | Auto-increment ID       |
| `file_id`   | INTEGER NOT NULL    | FK to registry          |
| `text`      | TEXT                | Extracted text content  |
| `text_hash` | TEXT                | Hash of extracted text  |
| `quality`   | REAL                | Text extraction quality |
| `pages`     | INTEGER             | Number of pages         |

### metadata_cache

Stores file metadata.

| Column    | Type                | Description       |
| --------- | ------------------- | ----------------- |
| `id`      | INTEGER PRIMARY KEY | Auto-increment ID |
| `file_id` | INTEGER NOT NULL    | FK to registry    |
| `key`     | TEXT NOT NULL       | Metadata key      |
| `value`   | TEXT                | Metadata value    |

## Intelligence Database

### intelligence

Core facts extracted from evidence files.

| Column        | Type                | Description                |
| ------------- | ------------------- | -------------------------- |
| `id`          | INTEGER PRIMARY KEY | Auto-increment ID          |
| `fingerprint` | TEXT NOT NULL       | Fact fingerprint           |
| `source_file` | TEXT                | Source file path           |
| `page`        | INTEGER             | Page number                |
| `quote`       | TEXT                | Direct quote from source   |
| `summary`     | TEXT                | Concise fact statement     |
| `category`    | TEXT                | Crime/fact category        |
| `date`        | TEXT                | Associated date            |
| `severity`    | TEXT                | Critical/High/Medium/Low   |
| `confidence`  | REAL                | Confidence score (0.0-1.0) |
| `quality`     | REAL                | Extraction quality score   |
| `deleted`     | BOOLEAN             | Soft-delete flag           |

### entities

Named entities extracted from facts.

| Column       | Type                | Description                                |
| ------------ | ------------------- | ------------------------------------------ |
| `id`         | INTEGER PRIMARY KEY | Auto-increment ID                          |
| `name`       | TEXT NOT NULL       | Entity name                                |
| `type`       | TEXT                | Person/Location/Date/Money/Org/Phone/Email |
| `normalized` | TEXT                | Normalized form                            |
| `fact_id`    | INTEGER             | FK to intelligence                         |

### annotations

User annotations on facts.

| Column       | Type                | Description           |
| ------------ | ------------------- | --------------------- |
| `id`         | INTEGER PRIMARY KEY | Auto-increment ID     |
| `fact_id`    | INTEGER NOT NULL    | FK to intelligence    |
| `content`    | TEXT                | Annotation text       |
| `created_at` | INTEGER             | Creation timestamp    |
| `updated_at` | INTEGER             | Last update timestamp |

### entity_aliases

Alias resolution for entity deduplication.

| Column      | Type                | Description       |
| ----------- | ------------------- | ----------------- |
| `id`        | INTEGER PRIMARY KEY | Auto-increment ID |
| `entity_id` | INTEGER NOT NULL    | FK to entities    |
| `alias`     | TEXT NOT NULL       | Alternative name  |

### evidence_chains

Evidence relationship groups.

| Column        | Type                | Description        |
| ------------- | ------------------- | ------------------ |
| `id`          | INTEGER PRIMARY KEY | Auto-increment ID  |
| `name`        | TEXT NOT NULL       | Chain name         |
| `description` | TEXT                | Chain description  |
| `created_at`  | INTEGER             | Creation timestamp |

### evidence_chain_links

Links between facts and chains.

| Column         | Type                | Description           |
| -------------- | ------------------- | --------------------- |
| `chain_id`     | INTEGER NOT NULL    | FK to evidence_chains |
| `fact_id`      | INTEGER NOT NULL    | FK to intelligence    |
| `relationship` | TEXT                | Relationship type     |
| PRIMARY KEY    | (chain_id, fact_id) |                       |

### checkpoints

Job resumption state.

| Column     | Type                | Description              |
| ---------- | ------------------- | ------------------------ |
| `id`       | INTEGER PRIMARY KEY | Auto-increment ID        |
| `job_id`   | TEXT NOT NULL       | Job identifier           |
| `stage`    | TEXT                | Current processing stage |
| `progress` | INTEGER             | Progress percentage      |
| `data`     | TEXT                | Checkpoint data (JSON)   |

### audit_log

Action audit trail.

| Column      | Type                | Description           |
| ----------- | ------------------- | --------------------- |
| `id`        | INTEGER PRIMARY KEY | Auto-increment ID     |
| `action`    | TEXT NOT NULL       | Action performed      |
| `timestamp` | INTEGER             | Action timestamp      |
| `details`   | TEXT                | Action details (JSON) |

### error_queue

Failed jobs with retry logic.

| Column        | Type                | Description            |
| ------------- | ------------------- | ---------------------- |
| `id`          | INTEGER PRIMARY KEY | Auto-increment ID      |
| `job_type`    | TEXT NOT NULL       | Job type               |
| `error`       | TEXT                | Error message          |
| `retry_count` | INTEGER             | Number of retries      |
| `max_retries` | INTEGER             | Maximum retry attempts |
| `data`        | TEXT                | Job data (JSON)        |

### facts_fts (FTS5)

Full-text search on facts.

| Column     | Type | Description  |
| ---------- | ---- | ------------ |
| `summary`  | TEXT | Fact summary |
| `quote`    | TEXT | Direct quote |
| `category` | TEXT | Category     |

### entities_fts (FTS5)

Full-text search on entities.

| Column       | Type | Description     |
| ------------ | ---- | --------------- |
| `name`       | TEXT | Entity name     |
| `normalized` | TEXT | Normalized form |
| `type`       | TEXT | Entity type     |
