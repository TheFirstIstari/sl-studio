# Design Spec: Replace llama.cpp with rapid-mlx

## Overview

SL Studio is a macOS-only desktop app for forensic document analysis. The current
inference module (`src-tauri/src/inference/`) contains stub implementations
referencing llama.cpp concepts (GGUF files, `gpu_layers`, `LlamaPipeline`). This
spec describes replacing those stubs with **rapid-mlx** — a production-ready
Python package that provides Apple MLX inference with an OpenAI-compatible HTTP API.

`rapid-mlx` is installed via Homebrew (`/opt/homebrew/bin/rapid-mlx`) and provides:

- `rapid-mlx serve <model>` — starts an OpenAI-compatible HTTP server on `localhost:8000`
- `rapid-mlx models` — lists available model aliases (e.g., `qwen3.5-4b-4bit`)

No GGUF/llama.cpp code is currently wired up — all backends are stubs — so the
swap is low-risk.

## Scope

### In scope

- Replace `src-tauri/src/inference/llama.rs` (`LlamaPipeline`) → `mlx_pipeline.rs` (`MlxPipeline`)
- Update `reasoner.rs` to depend on `MlxPipeline`
- Update `model_registry.rs` `ModelInfo` for MLX model fields (model name instead of file path)
- Update backend Tauri commands: `init_reasoner`, `validate_model`, `download_model`,
  `is_model_loaded`, `list_downloaded_models`
- Update frontend: `settings/+page.svelte`, `analysis/+page.svelte`, `stores/app.ts`
- Add `reqwest` dependency to `Cargo.toml` (for HTTP calls to rapid-mlx serve)
- Remove all `gguf`/`llama` references from `src/`

### Out of scope

- Training MLX models — SL Studio uses pre-trained models only
- Support for both GGUF and MLX backends — this is a full replacement
- Changes to database schema

## Architecture

### Rust backend (`src-tauri/src/inference/`)

```
inference/
  mod.rs            — get_builtin_pipelines() + ModuleRegistry (unchanged)
  mlx_pipeline.rs   — NEW: MlxPipeline wrapping rapid-mlx subprocess + HTTP client
  reasoner.rs       — Updated to use MlxPipeline
  model_registry.rs — Updated ModelInfo for MLX model names
```

#### MlxPipeline (replaces LlamaPipeline)

The pipeline manages a `rapid-mlx serve` subprocess and communicates via
OpenAI-compatible HTTP API.

```rust
pub struct MlxPipeline {
    model_name: String,       // e.g., "qwen3.5-4b-4bit"
    context_length: usize,
    server_url: String,       // e.g., "http://127.0.0.1:8000"
    child: Option<Child>,     // rapid-mlx serve subprocess handle
}

impl MlxPipeline {
    pub fn new(model_name: String, context_length: usize) -> Self {
        Self { model_name, context_length, server_url: "http://127.0.0.1:8000".into(), child: None }
    }

    pub fn load(&mut self) -> Result<()> {
        // Start "rapid-mlx serve <model_name>" as background subprocess
        // Wait for HTTP server to be ready (poll localhost:8000/health)
    }

    pub fn infer(&self, prompt: &str, max_tokens: usize) -> Result<String> {
        // POST to /v1/chat/completions
        // Parse response, return assistant message content
    }
}

impl Drop for MlxPipeline {
    // Terminate the rapid-mlx serve subprocess
}
```

Key differences from LlamaPipeline:

- Uses `rapid-mlx serve` subprocess (no native Rust MLX crate needed)
- Model identified by name (e.g., `qwen3.5-4b-4bit`), not file path
- No `gpu_layers` parameter (rapid-mlx handles Metal GPU automatically)
- Uses `reqwest` for HTTP calls to the OpenAI-compatible API

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
    pub source: String,          // e.g., "huggingface" or "local"
    pub mlx_model_name: String,  // e.g., "qwen3.5-4b-4bit"
    pub context_length: usize,
    pub downloaded: bool,
    pub local_path: String,      // path to model (may be empty for remote)
}
```

### Tauri commands (`src-tauri/src/commands/mod.rs`)

| Command                  | Change                                                       |
| ------------------------ | ------------------------------------------------------------ |
| `init_reasoner`          | Remove `gpu_layers` param; `model_path` becomes `model_name` |
| `validate_model`         | Call `rapid-mlx models` to check if model exists             |
| `download_model`         | Use `rapid-mlx pull <model_name>` command                    |
| `is_model_loaded`        | Check if rapid-mlx serve is running / model is cached        |
| `list_downloaded_models` | Call `rapid-mlx models` to list available models             |

### Frontend

| File                           | Change                                                                             |
| ------------------------------ | ---------------------------------------------------------------------------------- |
| `routes/settings/+page.svelte` | Remove `.gguf` file dialog; use model name dropdown or rapid-mlx model selector    |
| `routes/analysis/+page.svelte` | `init_reasoner` invoke: remove `gpuLayers`; use `modelName` instead of `modelPath` |
| `stores/app.ts`                | Update `ModelConfig` type: remove `gpuLayers`, `local_path` → `mlx_model_name`     |

## Data flow

1. User selects an MLX model (e.g., `qwen3.5-4b-4bit`) in Settings
2. `download_model` calls `rapid-mlx pull <model_name>` to download/cache
3. User starts analysis → `init_reasoner` starts `rapid-mlx serve <model_name>`
4. `analyze_batch` calls `Reasoner::extract_facts()` → `MlxPipeline::infer()` → HTTP POST to localhost:8000
5. Facts are stored in SQLite via existing `intelligence` table

## Error handling

- `AppError` type remains the single error type (`Result<T, AppError>`)
- MLX/rapid-mlx errors convert via `From<anyhow::Error> for AppError` (already implemented)
- All commands return `Result<T>` using the project's `Result` type alias
- If `rapid-mlx` is not installed, return a clear error message

## Dependencies

Add to `Cargo.toml`:

```toml
reqwest = { version = "0.28", features = ["json", "rustls-tls"] }
```

`rapid-mlx` is a system-level dependency installed via Homebrew (not a Rust crate).
The app should check for its presence at startup and guide the user to install it if missing.

## Testing

- `cargo clippy -- -D warnings` must pass (zero warnings)
- `cargo test` must pass (unit tests in database module)
- `npm run check` must pass (SvelteKit type checking)
- Manual smoke test: `npm run tauri dev` loads and Settings page opens
- `rapid-mlx` binary is present at `/opt/homebrew/bin/rapid-mlx`
