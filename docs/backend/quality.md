# Quality & Deduplication

## Overview

Quality assessment, deduplication, and anomaly detection are implemented as
Tauri commands in `commands/mod.rs`. There is no separate `inference/quality/`
module — all quality logic lives in the command handlers.

## Deduplication Commands

| Command                | Parameters                                  | Returns            | Description                          |
| ---------------------- | ------------------------------------------- | ------------------ | ------------------------------------ |
| `find_duplicate_facts` | `threshold, require_same_category, require_same_date` | `Vec<DuplicateGroup>` | Find similar facts |
| `merge_duplicate_facts` | `keeper_id, member_ids`                    | `i64`              | Soft-delete members into keeper      |

### DuplicateGroup

| Field        | Type     | Description                    |
| ------------ | -------- | ------------------------------ |
| `keeper_id`  | i64      | ID of the fact to keep         |
| `member_ids` | Vec<i64> | IDs of duplicate facts         |
| `similarity` | f64      | Similarity score (0.0–1.0)     |

### Deduplication Process

```
Facts List
    │
    ▼
┌─────────────┐
│ Compare All │ ← Pairwise similarity check
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Group Dups  │ ← Cluster similar facts
└──────┬──────┘
       │
       ▼
│ Merge       │ ← Soft-delete duplicates, keep highest
└──────┬──────┘
       │
       ▼
   Deduplicated Facts
```

> **Note**: `find_duplicate_facts` is currently a stub — a real implementation
> would use Jaro-Winkler or n-gram similarity. `merge_duplicate_facts` performs
> soft deletes via `is_deleted` flag.

## Cross-Validation

| Command              | Parameters           | Returns                  | Description                    |
| -------------------- | -------------------- | ------------------------ | ------------------------------ |
| `cross_validate_fact` | `intelligence_id, threshold` | `CrossValidationResult` | Validate fact against sources |

### CrossValidationResult

| Field               | Type                    | Description                |
| ------------------- | ----------------------- | -------------------------- |
| `intelligence_id`   | i64                     | Fact being validated       |
| `source_filename`   | String                  | Source file name           |
| `matches`           | Vec<CorroborationMatch> | Matching facts             |
| `consensus_score`   | f64                     | Agreement score (0.0–1.0)  |

## Evidence Weighting

| Command             | Parameters        | Returns | Description                       |
| ------------------- | ----------------- | ------- | --------------------------------- |
| `get_evidence_weight` | `intelligence_id` | `f64`   | Weighted confidence for a fact    |

Calculates weighted confidence from the `evidence_weights` table, falling back
to `reliability_score * confidence` from the `intelligence` table.

## Anomaly Detection

| Command              | Parameters          | Returns      | Description                    |
| -------------------- | ------------------- | ------------ | ------------------------------ |
| `detect_anomalies`   | `metric, threshold_std` | `Vec<Anomaly>` | Z-score outlier detection  |

Supports `severity` and `confidence` metrics. Returns all facts with their
values (deviation calculation is a stub returning 0.0).
