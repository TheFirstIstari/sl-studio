# Database Layer

## Overview

The database layer (`core/database.rs`, ~3397 lines) manages two SQLite connections:

- **Registry DB**: Tracks files, extracted text, and metadata
- **Intelligence DB**: Stores facts, entities, annotations, and analysis results

## Database Structure

```rust
struct Database {
    registry_conn: Connection,
    intelligence_conn: Connection,
    cache: Cache, // In-memory TTL cache for aggregate queries
}
```

## Tables

### Registry Database

| Table            | Purpose                                                             |
| ---------------- | ------------------------------------------------------------------- |
| `registry`       | File tracking with fingerprint, path, type, size, processing status |
| `text_cache`     | Extracted text storage with hash and quality score                  |
| `metadata_cache` | File metadata storage                                               |

### Intelligence Database

| Table                  | Purpose                                                             |
| ---------------------- | ------------------------------------------------------------------- |
| `intelligence`         | Extracted facts with quote, category, severity, confidence, quality |
| `entities`             | Named entities (person, location, date, money, org, phone, email)   |
| `annotations`          | User annotations on facts                                           |
| `entity_aliases`       | Alias resolution for entity deduplication                           |
| `evidence_chains`      | Evidence relationship groups                                        |
| `evidence_chain_links` | Links between facts and chains                                      |
| `checkpoints`          | Job resumption state                                                |
| `audit_log`            | Action audit trail                                                  |
| `error_queue`          | Failed jobs with retry logic                                        |
| `facts_fts`            | FTS5 full-text search on facts                                      |
| `entities_fts`         | FTS5 full-text search on entities                                   |

## Key Operations

### CRUD Operations

- Insert/update/delete for all tables
- Batch operations for performance
- Soft-delete support for facts

### Search

- **FTS5 full-text search** with Boolean operators (AND, OR, NOT, phrases)
- **Combined search** across facts and entities
- **Tag-based filtering**
- **Faceted search** by category, severity, date range

### Analysis Queries

- **Timeline events**: Chronological fact ordering
- **Statistics**: Aggregates by severity, category, entity
- **Network analysis**: Entity relationships, centrality calculations
- **Anomaly detection**: Z-score based outlier detection
- **Evidence weighting**: Confidence-based fact ranking
- **Chain detection**: Related fact grouping

### Caching

In-memory TTL cache for frequently accessed aggregate queries:

- Category distribution
- Severity distribution
- Top entities
- Overall statistics

Cache TTL: 30-60 seconds depending on query type.

## Data Structures

| Struct              | Purpose                                 |
| ------------------- | --------------------------------------- |
| `RegistryEntry`     | File record with fingerprint and status |
| `IntelligenceEntry` | Fact record with all metadata           |
| `SearchResult`      | Combined search result                  |
| `TimelineEvent`     | Chronological fact with date            |
| `EntityCentrality`  | Network analysis result                 |
| `Anomaly`           | Detected outlier with z-score           |
| `WeightedEvidence`  | Confidence-ranked evidence              |
