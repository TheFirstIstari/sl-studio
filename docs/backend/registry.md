# Registry Scanner

## Overview

The registry scanner (`core/registry.rs`, ~223 lines) walks an evidence directory, fingerprints files with SHA-256, and batch-inserts records into the database.

## RegistryWorker

```rust
struct RegistryWorker {
    db: Database,
    fingerprint_cache: HashSet<String>,
}
```

## Process

```
Evidence Directory
       │
       ▼
┌─────────────┐
│ Walk Dir    │ ← Recursive file discovery (walkdir)
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Filter      │ ← Supported extensions only
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Fingerprint │ ← SHA-256 hash (parallel via rayon)
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Dedup Check │ ← Skip if fingerprint in cache
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Batch Insert│ ← Insert into registry DB
└──────┬──────┘
       │
       ▼
   Progress Report (via channels)
```

## Key Features

- **Parallel fingerprinting**: Uses rayon for CPU-bound hashing
- **In-memory cache**: Avoids re-hashing known files
- **Progress reporting**: Channels for real-time UI updates
- **Batch insertion**: Efficient database writes

## Priority Queue

Files are assigned processing priorities:

| Priority  | Description                         |
| --------- | ----------------------------------- |
| New       | Never processed before              |
| Modified  | Fingerprint changed since last scan |
| Extracted | Text extracted but not yet analyzed |
| Rerun     | Explicitly requested reprocessing   |
