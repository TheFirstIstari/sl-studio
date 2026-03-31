# Quality & Deduplication

## Quality Scoring

`inference/quality/scoring.rs` (~131 lines)

### ExtractionQuality

```rust
struct ExtractionQuality {
    confidence: f64,
    text_coverage: f64,
    entity_density: f64,
    quote_quality: f64,
    overall: f64,
    retry_recommended: bool,
    issues: Vec<QualityIssue>,
}
```

### Metrics

| Metric           | Description                         |
| ---------------- | ----------------------------------- |
| `confidence`     | LLM confidence in extracted facts   |
| `text_coverage`  | Ratio of extracted text to original |
| `entity_density` | Entities per unit of text           |
| `quote_quality`  | Quality of direct quotes            |
| `overall`        | Weighted composite score            |

### Quality Levels

| Level    | Threshold | Color  |
| -------- | --------- | ------ |
| Good     | >= 0.8    | Green  |
| Marginal | 0.5 - 0.8 | Yellow |
| Poor     | < 0.5     | Red    |

### Quality Issues

```rust
enum QualityIssue {
    LowConfidence,
    ShortQuote,
    PoorCoverage,
    EntityMismatch,
}
```

## Fact Deduplication

`inference/quality/deduplication.rs` (~296 lines)

### DeduplicationConfig

```rust
struct DeduplicationConfig {
    similarity_threshold: f64, // Default: 0.85
    match_on_fields: Vec<String>,
    merge_strategy: MergeStrategy,
}
```

### Merge Strategies

```rust
enum MergeStrategy {
    KeepHighestConfidence,
    KeepMostSevere,
    MergeAll,
}
```

### Similarity Algorithm

Uses Jaccard word-set similarity:

```
similarity = |A ∩ B| / |A ∪ B|
```

Where A and B are word sets of two facts' summaries.

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
┌─────────────┐
│ Merge       │ ← Apply merge strategy
└──────┬──────┘
       │
       ▼
   Deduplicated Facts
```
