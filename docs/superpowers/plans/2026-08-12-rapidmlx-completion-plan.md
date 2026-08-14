# RapidMLX Completion Plan

> **For agentic workers:** Use subagent-driven-development or executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Complete the rapid-mlx swap by fixing bugs, aligning types, wiring up the reasoner pipeline, cleaning dead code, updating docs, and committing all work to GitHub in logical segments.

**Architecture:** Backend `MlxPipeline` wraps `rapid-mlx serve` subprocess + HTTP client. `Reasoner` wraps `MlxPipeline` and is stored in `AppState` via `Arc<Mutex<Option<Reasoner>>>`. `analyze_batch` retrieves the reasoner and runs real LLM inference. Frontend types align with backend structs.

**Tech Stack:** Rust (Tauri 2), `reqwest` blocking HTTP, `rapid-mlx` CLI (Homebrew), SvelteKit 5 frontend, Playwright e2e

## Global Constraints

- macOS only (Apple Silicon)
- `rapid-mlx` installed at `/opt/homebrew/bin/rapid-mlx`
- Rust 1.97.1 (rustc)
- `cargo clippy -- -D warnings` must pass after each task
- `npm run check` must pass after frontend changes
- Commit in logical segments with GitHub issue references

---

### Task 1: Fix backend ModelConfig + HardwareInfo (lib.rs + commands/mod.rs)

**Files:**

- Modify: `src-tauri/src/lib.rs:11` (comment), `lib.rs:182-190` (ModelConfig), `lib.rs:221-228` (HardwareInfo)
- Modify: `src-tauri/src/commands/mod.rs:45-52` (load_config defaults), `commands/mod.rs:1386-1393` (get_recommended_settings)

**Interfaces:**

- Consumes: existing ModelConfig with `quantization` field
- Produces: ModelConfig with `mlx_model_name`/`dtype`, HardwareInfo without `gpu_layers`

- [x] **Step 1: Write the failing assertion** — verify `cargo check` fails with current mismatch by confirming the deserialization error path

Run: `cd src-tauri && cargo check`
Expected: PASSES (no error yet — the issue is runtime, not compile-time)

- [x] **Step 2: Update `lib.rs` ModelConfig** — remove `quantization`, add `mlx_model_name` and `dtype`

```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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

- [x] **Step 3: Update `load_config` defaults** — replace `quantization` with MLX fields:

```rust
model: crate::ModelConfig {
    source: "local".to_string(),
    id: "default".to_string(),
    mlx_model_name: "qwen3.5-4b-4bit".to_string(),
    dtype: "float16".to_string(),
    context_length: 4096,
    downloaded: false,
    local_path: String::new(),
},
```

- [x] **Step 4: Update `HardwareInfo`** — remove `recommended_gpu_layers`:

```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HardwareInfo {
    pub recommended_context: usize,
    pub recommended_batch_size: usize,
    pub worker_count: usize,
    pub backend: String,
}
```

- [x] **Step 5: Update `get_recommended_settings`** — remove `recommended_gpu_layers: 32`

- [x] **Step 6: Update `lib.rs:11` comment** — change "llama.cpp pipeline" to "MLX pipeline"

- [x] **Step 7: Verify**

```bash
cd src-tauri && cargo check
```

Expected: PASS

---

### Task 2: Wire up init_reasoner + analyze_batch + Reasoner (lib.rs, commands/mod.rs, reasoner.rs)

**Files:**

- Modify: `src-tauri/src/lib.rs:607-650` (AppState + build_tauri_app)
- Modify: `src-tauri/src/commands/mod.rs:1486-1508` (init_reasoner + analyze_batch)
- Modify: `src-tauri/src/inference/reasoner.rs:24-37` (extract_facts)
- Modify: `src-tauri/src/inference/mlx_pipeline.rs:6` (remove `#![allow(dead_code)]`)

**Interfaces:**

- Consumes: `AppState`, `MmlxPipeline` (from Task 1), `Reasoner`
- Produces: Working reasoner stored in AppState, real LLM inference in analyze_batch

- [x] **Step 1: Add Mutex import to lib.rs** — add `use std::sync::Mutex;`

- [x] **Step 2: Import Reasoner in lib.rs** — add `use inference::reasoner::Reasoner;` (or use full path)

- [x] **Step 3: Add `reasoner` field to AppState**

```rust
pub struct AppState {
    pub db: Arc<core::database::Pool>,
    pub metadata: HashMap<String, Metadata>,
    pub facts: HashMap<String, Fact>,
    pub chains: HashMap<String, Chain>,
    pub file_results: HashMap<String, FileResult>,
    pub reasoner: Arc<Mutex<Option<Reasoner>>>,
}
```

- [x] **Step 4: Initialize `reasoner` in `build_tauri_app()`**

```rust
let app = AppState {
    db: Arc::new(db),
    metadata: HashMap::new(),
    facts: HashMap::new(),
    chains: HashMap::new(),
    file_results: HashMap::new(),
    reasoner: Arc::new(Mutex::new(None)),
};
```

- [x] **Step 5: Update `init_reasoner`** — take `tauri::State`, call `load()`, store `Reasoner`

```rust
#[tauri::command]
pub async fn init_reasoner(
    state: tauri::State<'_, crate::AppState>,
    model_name: String,
    context_size: usize,
) -> Result<()> {
    let mut pipeline = crate::inference::mlx_pipeline::MlxPipeline::new(model_name, context_size);
    pipeline.load()?;
    let reasoner = crate::inference::reasoner::Reasoner::new(pipeline);
    *state.reasoner.lock().unwrap() = Some(reasoner);
    info!("MLX reasoner initialized");
    Ok(())
}
```

- [x] **Step 6: Update `analyze_batch`** — take `tauri::State`, retrieve reasoner, use `extract_facts`

```rust
#[tauri::command]
pub async fn analyze_batch(
    state: tauri::State<'_, crate::AppState>,
    fingerprints: Vec<String>,
) -> Result<()> {
    let db = require_db()?;
    let reasoner = state.reasoner.lock().unwrap()
        .as_ref()
        .ok_or_else(|| AppError("Reasoner not initialized. Call init_reasoner first.".into()))?;

    for fp in &fingerprints {
        let row = db.query_row(
            "SELECT file_name, extracted_text FROM text_cache WHERE fingerprint = ?1",
            rusqlite::params![fp],
            |row| {
                Ok((
                    row.get::<_, String>(0),
                    row.get::<_, String>(1),
                ))
            },
        )
        .map_err(|e| AppError(format!("No extracted text for fingerprint {}: {}", fp, e)))?;

        let facts = reasoner.extract_facts(&row.1)?;
        for fact in facts {
            db.execute(
                "INSERT INTO intelligence (fingerprint, filename, fact_summary, category,
                 identified_crime, severity_score, confidence, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))",
                rusqlite::params![
                    fp,
                    row.0,
                    fact.fact_summary,
                    fact.category.unwrap_or_default(),
                    fact.identified_crime,
                    fact.severity_score,
                    fact.confidence,
                ],
            )?;
        }
    }
    info!("Analyzed {} fingerprints", fingerprints.len());
    Ok(())
}
```

- [x] **Step 7: Update `Reasoner::extract_facts`** — call `infer()` instead of returning placeholder

```rust
pub fn extract_facts(&self, text: &str) -> Result<Vec<crate::Fact>> {
    info!("Extracting facts from: {} chars", text.len());
    let content = self.pipeline.infer(
        &format!("Extract facts and entities from: {}", text),
        2048,
    )?;
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

- [x] **Step 8: Remove `#![allow(dead_code)]` from `mlx_pipeline.rs`** (line 6) — pipeline is now used

- [x] **Step 9: Verify**

```bash
cd src-tauri && cargo check
cd src-tauri && cargo clippy -- -D warnings
```

Expected: PASS (zero warnings)

---

### Task 3: Remove dead code (model_registry.rs + inference/mod.rs)

**Files:**

- Delete: `src-tauri/src/inference/model_registry.rs`
- Modify: `src-tauri/src/inference/mod.rs` — remove `pub mod model_registry;`

**Interfaces:**

- Produces: Clean inference module with no unused code

- [x] **Step 1: Remove `pub mod model_registry;` from `mod.rs`**

- [x] **Step 2: Delete `model_registry.rs`**

```bash
git rm src-tauri/src/inference/model_registry.rs
```

- [x] **Step 3: Verify**

```bash
cd src-tauri && cargo clippy -- -D warnings
```

Expected: PASS, no dead code warnings

---

### Task 4: Fix frontend types + settings page (app.ts, settings/+page.svelte)

**Files:**

- Modify: `src/routes/settings/+page.svelte:9-14` (ModelInfo interface)
- Modify: `src/routes/settings/+page.svelte:241` (downloadSelectedModel)
- Modify: `src/routes/settings/+page.svelte:431-432` (model selection)
- Modify: `src/routes/settings/+page.svelte:170-198` (saveConfig completeness)

**Interfaces:**

- Consumes: Backend `DownloadedModel` with `mlx_model_name` field
- Produces: Frontend correctly uses `mlx_model_name` everywhere

- [x] **Step 1: Update `ModelInfo` interface** — add `mlx_model_name`:

```typescript
interface ModelInfo {
	id: string;
	filename: string;
	size: number;
	path: string;
	mlx_model_name: string;
}
```

- [x] **Step 2: Fix `downloadSelectedModel`** — use `result.mlx_model_name` not `result.path`:

```typescript
// Before:
config.mlxModelName = result.path;
// After:
config.mlxModelName = result.mlx_model_name;
```

- [x] **Step 3: Fix downloaded models list** — use `model.mlx_model_name`:

```typescript
// Before:
onclick={() => (config.mlxModelName = model.path)}
class:selected={config.mlxModelName === model.path}
// After:
onclick={() => (config.mlxModelName = model.mlx_model_name)}
class:selected={config.mlxModelName === model.mlx_model_name}
```

- [x] **Step 4: Fix `saveConfig`** — include all HardwareConfig fields, fix version:

```typescript
const configData = {
    version: '0.3.0',
    ...
    hardware: {
        gpu_backend: 'cpu',
        gpu_memory_fraction: 0.8,
        cpu_workers: hwInfo.cpu_workers,
        auto_scale_workers: true,
        batch_size: hwInfo.recommended_batch_size,
        auto_scale_batch: true,
        ocr_provider: 'ocrs',
        whisper_size: 'base',
        whisper_model_path: config.whisperModelPath || null
    },
    ...
};
```

- [x] **Step 5: Verify**

```bash
npm run check
```

Expected: PASS

---

### Task 5: Fix frontend dashboard + analysis page (+page.svelte, analysis/+page.svelte)

**Files:**

- Modify: `src/routes/+page.svelte:7,89,92,95`
- Modify: `src/routes/analysis/+page.svelte:241,251`

**Interfaces:**

- Consumes: Config with `mlx_model_name` instead of `local_path` for model selection
- Produces: Dashboard and analysis use correct model field

- [x] **Step 1: Fix dashboard** — use `mlx_model_name` instead of `local_path`:

```typescript
// Before:
let modelPath = $derived($config?.model?.local_path || '');
// After:
let modelName = $derived($config?.model?.mlx_model_name || '');
```

Update the status display:

```typescript
// Before:
{
	$modelLoaded ? 'Loaded' : modelPath ? 'Not loaded' : 'No model configured';
}
// After:
{
	$modelLoaded ? 'Loaded' : modelName ? 'Not loaded' : 'No model configured';
}
```

- [x] **Step 2: Fix analysis page** — check `mlx_model_name` instead of `local_path`:

```typescript
// Before:
if (!$config?.model?.local_path) {
// After:
if (!$config?.model?.mlx_model_name) {
```

Fix model name fallback:

```typescript
// Before:
const modelName = $config?.model?.local_path || (models.length > 0 ? models[0].path : null);
// After:
const modelName =
	$config?.model?.mlx_model_name || (models.length > 0 ? models[0].mlx_model_name : null);
```

- [x] **Step 3: Verify**

```bash
npm run check
```

Expected: PASS

---

### Task 6: Fix e2e tests (settings.test.ts)

**Files:**

- Modify: `e2e/settings.test.ts:93-94`

**Interfaces:**

- Consumes: Settings page with `#mlxModelName` input
- Produces: Tests pass with correct selector

- [x] **Step 1: Fix selector** — change `#modelPath` to `#mlxModelName`

```typescript
// Before:
test('should have model path input', async ({ page }) => {
    await expect(page.locator('#modelPath')).toBeVisible();
// After:
test('should have model name input', async ({ page }) => {
    await expect(page.locator('#mlxModelName')).toBeVisible();
```

- [x] **Step 2: Verify** (if Playwright available)

```bash
npx playwright test e2e/settings.test.ts --grep "model name input"
```

Expected: Selector exists on page

---

### Task 7: Update AGENTS.md

**Files:**

- Modify: `AGENTS.md:79`

**Interfaces:**

- Produces: Documentation reflecting MLX pipeline

- [x] **Step 1: Update comment** — change `inference/            llama.cpp pipeline, reasoner, model registry` to `inference/            MLX pipeline, reasoner`

- [x] **Step 2: Verify** no llama/gguf references in AGENTS.md

```bash
grep -in "gguf\|llama" AGENTS.md
```

Expected: No matches

---

### Task 8: Update documentation (docs/\*, SPEC.md, README.md, CHANGELOG.md)

**Files:**

- Modify: `docs/backend/inference.md` — replace llama.cpp docs with rapid-mlx
- Modify: `docs/backend/models.md` — replace GGUF with MLX model management
- Modify: `docs/architecture/pipeline.md` — update LlamaModel → MlxPipeline
- Modify: `docs/architecture/system.md` — update system overview
- Modify: `docs/project-overview.md` — remove GGUF/llama references
- Modify: `docs/README.md` — update overview
- Modify: `SPEC.md` — update spec references
- Modify: `README.md` — update project README
- Modify: `CHANGELOG.md` — add entry for completion

**Interfaces:**

- Produces: Documentation consistent with rapid-mlx implementation

- [x] **Step 1: Update `docs/backend/inference.md`** — document MlxPipeline, rapid-mlx subprocess, OpenAI-compatible API

- [x] **Step 2: Update `docs/backend/models.md`** — document `rapid-mlx models`/`pull`/`serve` commands

- [x] **Step 3: Update `docs/architecture/pipeline.md`** — replace `LlamaModel` with `MlxPipeline`

- [x] **Step 4: Update remaining docs** — `system.md`, `project-overview.md`, `docs/README.md`, `SPEC.md`, `README.md`

- [x] **Step 5: Add CHANGELOG entry**

- [x] **Step 6: Verify** no stale references

```bash
grep -rin "gguf\|llama\|gpu_layers" docs/ AGENTS.md SPEC.md README.md CHANGELOG.md
```

Expected: No matches (except Whisper `ggml` placeholder which is unrelated)

---

### Task 9: Commit working tree to GitHub in segments

**Files:** All 117 dirty files

**Interfaces:**

- Produces: Clean git history with logical commits, each referencing a GitHub issue

- [x] **Step 1: Commit backend MLX fixes** (Task 1-3 changes)

```bash
git add src-tauri/src/lib.rs src-tauri/src/commands/mod.rs src-tauri/src/inference/
git commit -m "fix(backend): wire up rapid-mlx Reasoner pipeline, fix ModelConfig"
```

- [x] **Step 2: Commit reorganization deletions** (101 deleted files)

```bash
git add -A src-tauri/src/commands/ src-tauri/src/core/queries/ src-tauri/src/config/
git commit -m "refactor: consolidate command and query modules into mod.rs"
```

- [x] **Step 3: Commit other modifications** (build.rs, capabilities, icons, extractors, etc.)

```bash
git add -A
git commit -m "chore: update build config, capabilities, icons, extractors"
```

- [x] **Step 4: Commit frontend fixes** (Task 4-5 changes)

```bash
git add src/lib/stores/app.ts src/routes/settings/+page.svelte src/routes/analysis/+page.svelte src/routes/+page.svelte
git commit -m "fix(frontend): use mlx_model_name fields, fix config save/load"
```

- [x] **Step 5: Commit e2e test fix** (Task 6)

```bash
git add e2e/settings.test.ts
git commit -m "fix(tests): update settings.test.ts selectors for MLX model fields"
```

- [x] **Step 6: Commit docs + AGENTS.md** (Task 7-8)

```bash
git add AGENTS.md docs/ SPEC.md README.md CHANGELOG.md
git commit -m "docs: update for rapid-mlx swap completion"
```

- [x] **Step 7: Verify** `cargo check` + `npm run check` + `cargo clippy -- -D warnings`

- [x] **Step 8: Push to GitHub**

```bash
git push origin main
```

- [x] **Step 9: Close GitHub issues**

```bash
gh issue close <num> --comment "Completed"
```
