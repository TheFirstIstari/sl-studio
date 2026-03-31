# Analysis Queries

## Network Analysis

### Entity Relationships

Builds a graph of entity co-occurrence within facts:

```rust
fn get_entity_relationships(&self) -> Vec<Relationship>
```

Returns edges with source, target, and weight (co-occurrence count).

### Centrality Calculations

**Degree Centrality**: Count of direct connections per entity.

```rust
fn calculate_degree_centrality(&self) -> Vec<EntityCentrality>
```

**Betweenness Centrality** (Brandes Algorithm): Measures how often an entity lies on shortest paths between other entities.

```rust
fn calculate_betweenness(&self) -> Vec<EntityCentrality>
fn bfs_brandes(&self, source: &str) -> HashMap<String, f64>
```

**Community Detection** (Louvain Algorithm): Groups entities into communities based on connection density.

```rust
fn detect_communities(&self) -> Vec<EntityCommunity>
fn calculate_modularity(&self) -> f64
```

## Anomaly Detection

### Z-Score Based Detection

Detects outliers in severity, confidence, and quality metrics:

```rust
fn detect_anomalies(&self, threshold: f64) -> Vec<Anomaly>
```

| Metric     | Anomaly Condition    |
| ---------- | -------------------- |
| Severity   | Z-score > threshold  |
| Confidence | Z-score < -threshold |
| Quality    | Z-score < -threshold |

### File Size Anomaly Detection

Detects files with unusual sizes compared to the dataset:

```rust
fn detect_file_size_anomalies(&self) -> Vec<FileAnomaly>
```

### Fact Outlier Detection

Detects facts with unusual patterns:

```rust
fn detect_fact_outliers(&self) -> Vec<FactOutlier>
```

### Suspicious Pattern Detection

Identifies suspicious patterns in entity co-occurrences:

```rust
fn detect_suspicious_patterns(&self) -> Vec<SuspiciousPattern>
fn count_entity_cooccurrences(&self) -> HashMap<(String, String), usize>
```

## Temporal Analysis

### Timeline Events

Extracts facts with dates and orders them chronologically:

```rust
fn get_timeline_events(&self) -> Vec<TimelineEvent>
```

### Date Range Filtering

Supports filtering facts by date range for focused analysis.

## Evidence Weighting

Ranks evidence by confidence and severity:

```rust
fn get_weighted_evidence(&self) -> Vec<WeightedEvidence>
```

Weight calculation combines:

- Confidence score
- Severity level
- Quality score
- Source reliability
