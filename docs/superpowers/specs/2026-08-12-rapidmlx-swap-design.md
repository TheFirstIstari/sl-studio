# Design Spec: Replace llama.cpp with MLX

## Overview

SL Studio is a macOS-only desktop app for forensic document analysis. The current
inference module (`src-tauri/src/inference/`) contains stub implementations
referencing llama.cpp concepts (GGUF files, `gpu_layers`, `LlamaPipeline`). This
spec describes replacing those stubs with Apple MLX (`rapidmlx` / `mlx` crate),
which is the native ML framework for Apple Silicon.

No GGUF/llama.cpp code is currently wired up — all backends are stubs — so the
swap is low-risk.

## Scope

### In scope
- Replace `src-tauri/src/inference/llama.rs` (`LlamaPipeline`) → `mlx_pipeline.rs` (`MlxPipeline`)
- Update `reasoner.rs` to depend on `MlxPipeline`
- Update `model_registry.rs` `ModelInfo` for MLX model fields (`.safetensors` format)
- Update backend Tauri commands: `init_reasoner`, `validate_model`, `download_model`,
  `is_model_loaded`, `list_downloaded_models`
- Update frontend: `settings/+page.svelte`, `analysis/+page.svelte`, `stores/app.ts`
- Add `rapidmlx` (or `mlx`) dependency to `Cargo.toml`
- Remove all `gguf`/`llama` references from `src/`

### Out of scope
- Training MLX models — SL Studio uses pre-trained models only
- Support for both GGUF and MLX backends — this is a full replacement
- Changes to database schema (the `models` table may need column updates but
  these are handled in the commands layer, not schema migrations)

## Architecture

### Rust backend (`src-tauri/src/inference/`)

```
inference/
  mod.rs            — get_builtin_pipelines() (unchanged)
  mlx_pipeline.rs   — NEW: MlxPipeline wrapping mlx::model::Model
  reasoner.rs       — Updated to use MlxPipeline
  model_registry.rs — Updated ModelInfo for MLX
```

#### MlxPipeline (replaces LlamaPipeline)

```rust
pub struct MlxPipeline {
    model: mlx::model::Model,   // MLX model instance
    context_length: usize,
}

impl MlxPipeline {
    pub fn new(model_path: String, context_length: usize) -> Self
    pub fn load(&mut self) -> Result<()>     // load weights from .safetensors
    pub fn infer(&self, prompt: &str, max_tokens: usize) -> Result<String>
}
```

Key differences from LlamaPipeline:
- No `gpu_layers` parameter (MLX auto-detects Metal GPU)
- Model loaded from `.safetensors` instead of `.gguf`
- Uses `mlx` crate API instead of `llama-cpp-gguf` C bindings

#### Reasoner

```rust
pub struct Reasoner {
    pipeline: crate::inference::mlx_pipeline::MlxPipeline,
}
```

All methods stay the same; only the `pipeline` field type changes.

#### ModelInfo

```rust
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub mlx_repo_id: String,       // e.g., "mistralai/Mistral-7B-v0.1"
    pub dtype: String,             // e.g., "float16", "bfloat16"
    pub context_length: usize,
    pub downloaded: bool,
    pub local_path: String,        // path to .safetensors file
}
```

### Tauri commands (`src-tauri/src/commands/mod.rs`)

| Command | Change |
|---------|--------|
| `init_reasoner` | Remove `gpu_layers` param; accept MLX model path |
| `validate_model` | Check `.safetensors` extension instead of `.gguf` |
| `download_model` | Update for MLX model format |
| `is_model_loaded` | No structural change |
| `list_downloaded_models` | No structural change |

### Frontend

| File | Change |
|------|--------|
| `routes/settings/+page.svelte` | File dialog filter: `.gguf` → `.safetensors`; placeholder text |
| `routes/analysis/+page.svelte` | `init_reasoner` invoke: remove `gpuLayers` arg |
| `stores/app.ts` | Update `ModelConfig` type: remove `gpuLayers`, `modelPath` points to `.safetensors` |

## Data flow

1. User selects an MLX model (`.safetensors`) in Settings
2. `download_model` fetches from Hugging Face Hub
3. User starts analysis → `init_reasoner` loads the model via `MlxPipeline::load()`
4. `analyze_batch` calls `Reasoner::extract_facts()` for each document
5. Facts are stored in SQLite via existing `intelligence` table

## Error handling

- `AppError` type remains the single error type (`Result<T, AppError>`)
- MLX errors from `rapidmlx`/`mlx` crate convert via `From<anyhow::Error> for AppError`
  (already implemented)
- All commands return `Result<T>` using the project's `Result` type alias

## Testing

- `cargo clippy -- -D warnings` must pass (zero warnings)
- `cargo test` must pass (unit tests in database module)
- `npm run check` must pass (SvelteKit type checking)
- Manual smoke test: `npm run tauri dev` loads and Settings page opens

## Dependencies

Add to `Cargo.toml`:
```toml
rapidmlx = "0.6"  # or mlx = "0.6" — TBD based on latest available version
```

No removal of existing dependencies needed (no GGUF crate was ever added).

## Implementation order

1. Create GitHub issues (done — 7 issues created)
2. Add `rapidmlx` dependency to `Cargo.toml`, verify compilation
3. Replace `LlamaPipeline` → `MlxPipeline` in `inference/llama.rs` → `inference/mlx_pipeline.rs`
4. Update `reasoner.rs` to use `MlxPipeline`
5. Update `model_registry.rs` for MLX model fields
6. Update backend command handlers
7. Update frontend
8. Full audit — no GGUF/llama references remain
