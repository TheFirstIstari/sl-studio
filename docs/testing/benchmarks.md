# Benchmarks

## Overview

Criterion benchmarks measure performance of critical backend operations.

## Benchmark File

`src-tauri/benches/database_bench.rs` (~334 lines)

## Benchmark Categories

### Search Query Parsing

Measures FTS5 query parsing performance:

| Query Type           | Length   | Time    |
| -------------------- | -------- | ------- |
| Simple search        | 13 chars | ~215 ns |
| AND/OR/NOT operators | 19 chars | ~289 ns |
| Phrase with OR       | 20 chars | ~333 ns |
| Complex nested       | 30 chars | ~480 ns |
| Long query           | 60 chars | ~895 ns |

**Finding**: Query parsing scales linearly (~15ns per character).

### Entity Overlap Computation

Compares HashSet vs Vec for entity set operations:

| Size         | HashSet | Vec     | Winner                |
| ------------ | ------- | ------- | --------------------- |
| 10 entities  | ~293 ns | ~31 ns  | Vec (9x faster)       |
| 50 entities  | ~1.5 µs | ~155 µs | Vec (10x faster)      |
| 100 entities | ~4.5 µs | ~618 µs | Vec (7x faster)       |
| 200 entities | ~3 µs   | ~2.4 ms | HashSet (800x faster) |

**Finding**: Use Vec for small sets (<50), HashSet for large sets (>100).

### String Operations

| Operation               | Time   |
| ----------------------- | ------ |
| String format           | ~38 ns |
| String clone (40 chars) | ~17 ns |
| to_lowercase (53 chars) | ~24 ns |
| contains check          | ~27 ns |

### Hashing

| Operation          | Time    |
| ------------------ | ------- |
| Hash single string | ~22 ns  |
| Hash 100 strings   | ~2.0 µs |

### Collections

| Operation            | Size         | Time    |
| -------------------- | ------------ | ------- |
| HashMap insert       | 1000 items   | ~139 µs |
| HashMap lookup       | 1000 lookups | ~53 µs  |
| Vec sort by severity | 1000 items   | ~385 ns |

### Category Icon Lookup

| Method                | Time    |
| --------------------- | ------- |
| With HashMap cache    | ~314 ns |
| Without cache (match) | ~33 ns  |

**Finding**: Direct match is 10x faster for small fixed sets.

## Running Benchmarks

### Using mise (recommended)

```bash
mise run benchmark          # Run all benchmarks
mise run benchmark_quick    # Quick run with minimal iterations
mise run benchmark_search   # Search query parsing only
mise run benchmark_entities # Entity overlap only
mise run benchmark_strings  # String operations only
mise run benchmark_collections # Collection operations only
mise run benchmark_save     # Save results to file
```

### Using cargo directly

```bash
cargo bench --manifest-path src-tauri/Cargo.toml
cargo bench --manifest-path src-tauri/Cargo.toml -- --quick
cargo bench --manifest-path src-tauri/Cargo.toml -- --warm-up-time 1
```

## Optimization Recommendations

### High Priority

1. **Remove HashMap caching for icon lookups** - Match is 10x faster
2. **Use Vec for small entity sets, HashSet for large** - Adaptive strategy
3. **Keep optimized `detect_chains()`** - Avoids N+1 queries

### Medium Priority

1. Pre-compute lowercase strings when possible
2. Batch database operations where applicable
3. Use `String::with_capacity()` for known-size strings

### Low Priority

1. String hashing is already optimal
2. HashMap operations are efficient
3. Sorting is appropriately O(n log n)
