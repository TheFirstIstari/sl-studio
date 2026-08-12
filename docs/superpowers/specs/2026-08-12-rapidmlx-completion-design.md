# Design Spec: Complete the rapid-mlx Swap (Bug Fixes & Audit)

## Overview

SL Studio is a macOS-only Tauri 2 + SvelteKit 5 desktop app. The initial
rapid-mlx swap (commits `f2c2bab6`–`810a8673`) replaced `llama.rs` with
`MlxPipeline`, updated `ModelInfo`/`DownloadedModel` for MLX fields, and
switched frontend commands to use model names instead of GGUF paths.

However, the swap is **buggy and incomplete**: the reasoner is never actually
used, config serialization mismatches cause failures, field mismatches break
model selection, tests are stale, and ~30 doc files still reference GGUF/llama.
This spec covers the **completion and bug-fix phase**.

## Scope

### In scope

- Fix `init_reasoner` so it actually loads the pipeline and stores it in `AppState`
- Fix `analyze_batch` to use the stored `Reasoner` for real LLM inference
- Align backend `ModelConfig` with frontend (replace `quantization` with `mlx_model_name`/`dtype`)
- Remove `recommended_gpu_layers` from `HardwareInfo` (backend)
- Fix frontend field mismatches (`LocalInfo` interface, `downloadSelectedModel`, dashboard, analysis)
- Fix stale e2e tests
- Clean up dead code (`ModelRegistry`, backend `ModelInfo`)
- Update `AGENTS.md`, docs, `SPEC.md`, `README.md`, `CHANGELOG.md`
- Commit 117 dirty files (reorganization) in logical segments

### Out of scope

- Switching away from rapid-mlx (it IS used directly as a subprocess — confirmed correct)
- Database schema changes
- Training or model conversion

## Current State Assessment

### Backend (`src-tauri/src/`)

| File                            | Status                                                                 |
| ------------------------------- | ---------------------------------------------------------------------- |
| `inference/mlx_pipeline.rs`     | ✅ Implemented — `MlxPipeline::new/load/infer/Drop` works correctly    |
| `inference/reasoner.rs`         | ⚠️ Structurally correct but **never instantiated**                     |
| `inference/model_registry.rs`   | ❌ **Dead code** — `ModelRegistry` and backend `ModelInfo` never used  |
| `commands/mod.rs:init_reasoner` | ❌ **Stub** — creates pipeline, never calls `load()`, never stores it  |
| `commands/mod.rs:analyze_batch` | ❌ **Hardcoded** — inserts "Unknown fact" without any LLM inference    |
| `lib.rs:ModelConfig`            | ❌ **Mismatch** — has `quantization`, missing `mlx_model_name`/`dtype` |
| `lib.rs:HardwareInfo`           | ❌ Still has `recommended_gpu_layers: 32`                              |
| `lib.rs:AppState`               | ❌ No pipeline holder field                                            |
| `lib.rs:11` comment             | ❌ Still says "llama.cpp pipeline"                                     |

### Frontend (`src/`)

| File                           | Status                                                                                                                                                |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `lib/stores/app.ts`            | ✅ `ModelConfig` has `mlx_model_name`/`dtype`, `HardwareInfo` has no `gpu_layers`                                                                     |
| `routes/settings/+page.svelte` | ❌ `ModelInfo` interface missing `mlx_model_name`; `downloadSelectedModel` uses `result.path` (always empty); downloaded model list uses `model.path` |
| `routes/analysis/+page.svelte` | ❌ Checks `local_path` (always empty); uses `models[0].path` (always empty)                                                                           |
| `routes/+page.svelte`          | ❌ Dashboard displays `local_path` (always empty)                                                                                                     |
| `e2e/settings.test.ts`         | ❌ References `#modelPath` (should be `#mlxModelName`)                                                                                                |

### Documentation

`AGENTS.md`, `SPEC.md`, `docs/backend/`, `docs/architecture/`, `docs/README.md`,
`docs/project-overview.md`, `CHANGELOG.md`, `README.md` — all contain stale
GGUF/llama.cpp references.

### Git

- Remote: `https://github.com/TheFirstIstari/sl-studio.git`
- Branch: `main`
- Working tree: 117 dirty files (101 deletions from file reorganization, 12 modifications, 4 untracked)
- `gh` CLI is authenticated

## Architecture

### AppState extension

Add a pipeline holder so `init_reasoner` can store the loaded `Reasoner` and
`analyze_batch` can retrieve it:

```rust
pub struct AppState {
    pub db: Arc<core::database::Pool>,
    pub metadata: HashMap<String, Metadata>,
    pub facts: HashMap<String, Fact>,
    pub chains: HashMap<String, Chain>,
    pub file_results: HashMap<String, FileResult>,
    pub reasoner: Arc<Mutex<Option<Reasoner>>>,  // NEW
}
```

`Arc<Mutex<Option<Reasoner>>>` is `Clone` (Arc is always Clone), so `AppState`
keeps its `#[derive(Clone)]`.

### init_reasoner command

```rust
#[tauri::command]
pub async fn init_reasoner(
    state: tauri::State<'_, AppState>,
    model_name: String,
    context_size: usize,
) -> Result<()> {
    let mut pipeline = MlxPipeline::new(model_name, context_size);
    pipeline.load()?;                    // spawn rapid-mlx serve + wait for health
    let reasoner = Reasoner::new(pipeline);
    *state.reasoner.lock().unwrap() = Some(reasoner);
    Ok(())
}
```

Requires adding `tauri::State<'_, AppState>` to the command signature and
registering it in the `invoke_handler` (Tauri auto-handles state injection).

### analyze_batch command

```rust
#[tauri::command]
pub async fn analyze_batch(
    state: tauri::State<'_, AppState>,
    fingerprints: Vec<String>,
) -> Result<()> {
    let db = require_db()?;
    let reasoner = state.reasoner.lock().unwrap()
        .as_ref()
        .ok_or_else(|| AppError("Reasoner not initialized. Run init_reasoner first.".into()))?;

    for fp in &fingerprints {
        // Get extracted text + filename from text_cache
        let row: (String, String) = db.query_row(
            "SELECT file_name, extracted_text FROM text_cache WHERE fingerprint = ?1",
            params![fp],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let facts = reasoner.extract_facts(&row.1)?;
        for fact in facts {
            db.execute(
                "INSERT INTO intelligence (fingerprint, filename, fact_summary, category,
                 identified_crime, severity_score, confidence, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))",
                params![fp, row.0, fact.fact_summary, fact.category, fact.identified_crime,
                        fact.severity_score, fact.confidence],
            )?;
        }
    }
    Ok(())
}
```

### Reasoner::extract_facts

Update from stub to actually call `infer()`:

```rust
pub fn extract_facts(&self, text: &str) -> Result<Vec<crate::Fact>> {
    let prompt = format!("Extract facts and entities from: {}", text);
    let content = self.pipeline.infer(&prompt, 2048)?;
    Ok(vec![crate::Fact {
        id: 0,
        fingerprint: "generated".to_string(),
        filename: "unknown".to_string(),
        fact_summary: content,
        category: Some("Unknown".to_string()),
        identified_crime: None,
        severity_score: 5,
        confidence: Some(0.8),
        created_at: chrono::Utc::now().to_rfc3339(),
    }])
}
```

### Backend ModelConfig

Replace `quantization` with `mlx_model_name` + `dtype`:

```rust
pub struct ModelConfig {
    pub source: String,
    pub id: String,
    pub mlx_model_name: String,
    pub dtype: String,
    pub context_length: usize,
    pub downloaded: bool,
    pub local_path: String,
}
```

### HardwareInfo

Remove `recommended_gpu_layers`:

```rust
pub struct HardwareInfo {
    pub recommended_context: usize,
    pub recommended_batch_size: usize,
    pub worker_count: usize,
    pub backend: String,
}
```

## Error Handling

- If `rapid-mlx` is not installed: `spawn()` returns `io::Error` → converted to `AppError`
- If `rapid-mlx serve` doesn't start: polling times out → `AppError("rapid-mlx serve did not become ready...")`
- If `analyze_batch` called without `init_reasoner`: explicit `AppError` message
- If `text_cache` row missing: `query_row` returns `rusqlite::Error::QueryReturnedNoRows` → `AppError`

## Test Strategy

- `cargo clippy -- -D warnings` — zero warnings (including dead code lint on `model_registry.rs`)
- `cargo test` — existing database unit tests
- `npm run check` — SvelteKit type checking (frontend types must align with backend)
- `npm test` — Playwright e2e (requires built app; settings.test.ts selector fix is critical)

## Commit Strategy

GitHub issues created for each logical group. Commits in these segments:

1. **Backend MLX fix** — ModelConfig, HardwareInfo, AppState, init_reasoner, analyze_batch, reasoner
2. **Frontend MLX fix** — app.ts, settings, analysis, dashboard
3. **Tests** — e2e settings.test.ts
4. **Docs** — AGENTS.md, docs/\*, SPEC.md, README, CHANGELOG
5. **Reorganization** — 117-file working tree (file consolidation)
