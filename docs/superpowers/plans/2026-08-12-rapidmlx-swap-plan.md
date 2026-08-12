# RapidMLX Swap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace llama.cpp stubs with rapid-mlx subprocess integration across Rust backend and SvelteKit frontend.

**Architecture:** MlxPipeline spawns `rapid-mlx serve <model>` as a subprocess and communicates via OpenAI-compatible HTTP API. Reasoner and ModelInfo are updated to use MlxPipeline. Frontend switches from GGUF file paths to MLX model names.

**Tech Stack:** Rust (Tauri 2), `reqwest` for HTTP, `rapid-mlx` CLI (Homebrew), SvelteKit 5 frontend

## Global Constraints

- macOS only (Apple Silicon)
- `rapid-mlx` installed at `/opt/homebrew/bin/rapid-mlx`
- Rust 1.97.1 (rustc)
- `cargo clippy -- -D warnings` must pass after each task
- `npm run check` must pass after frontend changes

---

### Task 1: Add reqwest dependency and create MlxPipeline file

**Files:**

- Modify: `src-tauri/Cargo.toml:8` (add reqwest dependency)
- Create: `src-tauri/src/inference/mlx_pipeline.rs`
- Delete: `src-tauri/src/inference/llama.rs`
- Modify: `src-tauri/src/inference/mod.rs:3` (replace `pub mod llama` with `pub mod mlx_pipeline`)

**Interfaces:**

- Produces: `MlxPipeline` struct with `new()`, `load()`, `infer()`, `Drop` impl

- [ ] Add reqwest to Cargo.toml:

```toml
reqwest = { version = "0.22", features = ["json", "rustls-tls"] }
```

- [ ] Create `mlx_pipeline.rs` with MlxPipeline struct and stub methods (no logic yet, just signatures):

```rust
use anyhow::Result;
use std::process::Child;
use tracing::info;

pub struct MlxPipeline {
    pub model_name: String,
    pub context_length: usize,
    pub server_url: String,
    pub child: Option<Child>,
}

impl MlxPipeline {
    pub fn new(model_name: String, context_length: usize) -> Self {
        Self {
            model_name,
            context_length,
            server_url: "http://127.0.0.1:8000".to_string(),
            child: None,
        }
    }

    pub fn load(&mut self) -> Result<()> {
        info!("Loading MLX model: {}", self.model_name);
        Ok(())
    }

    pub fn infer(&self, prompt: &str, max_tokens: usize) -> Result<String> {
        info!("Running MLX inference on prompt: {} ({} chars)", prompt.len(), max_tokens);
        Ok(String::new())
    }
}
```

- [ ] Delete `llama.rs` and update `mod.rs`:

```rust
// mod.rs — replace:
pub mod llama;
// with:
pub mod mlx_pipeline;
```

- [ ] Commit and verify: `cargo check` passes

```bash
git add src-tauri/Cargo.toml src-tauri/src/inference/mlx_pipeline.rs src-tauri/src/inference/mod.rs
git rm src-tauri/src/inference/llama.rs
git commit -m "refactor(inference): replace LlamaPipeline with MlxPipeline stub"
```

---

### Task 2: Implement MlxPipeline load() with rapid-mlx subprocess

**Files:**

- Modify: `src-tauri/src/inference/mlx_pipeline.rs`

**Interfaces:**

- Consumes: `rapid-mlx` binary at `/opt/homebrew/bin/rapid-mlx`
- Produces: `MmlxPipeline::load()` spawns subprocess, `infer()` makes HTTP calls

- [ ] Implement `load()` — spawns `rapid-mlx serve <model_name>` as subprocess:

```rust
use std::process::Command;
use std::time::Duration;

pub fn load(&mut self) -> Result<()> {
    info!("Starting rapid-mlx serve for model: {}", self.model_name);
    let child = Command::new("rapid-mlx")
        .args(["serve", &self.model_name])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    self.child = Some(child);

    // Wait for server to be ready (poll health endpoint)
    let client = reqwest::blocking::Client::new();
    for _ in 0..60 {
        if let Ok(resp) = client.get(&format!("{}/health", self.server_url)).send() {
            if resp.status().is_success() {
                info!("rapid-mlx serve is ready");
                return Ok(());
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    Err(anyhow::anyhow!("rapid-mlx serve did not become ready in 30s"))
}
```

- [ ] Implement `infer()` — POST to OpenAI-compatible API:

```rust
pub fn infer(&self, prompt: &str, max_tokens: usize) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let body = serde_json::json!({
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": max_tokens,
    });
    let resp = client
        .post(&format!("{}/v1/chat/completions", self.server_url))
        .json(&body)
        .send()?;
    let response: serde_json::Value = resp.json()?;
    let content = response["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    Ok(content)
}
```

- [ ] Update imports (remove `warn`, add needed imports)

- [ ] Commit and verify: `cargo check` + `cargo clippy -- -D warnings`

```bash
git add src-tauri/src/inference/mlx_pipeline.rs
git commit -m "feat(inference): implement MxlPipeline load() and infer() with rapid-mlx"
```

---

### Task 3: Update Reasoner to use MmlxPipeline

**Files:**

- Modify: `src-tauri/src/inference/reasoner.rs`

**Interfaces:**

- Consumes: `MlxPipeline` from Task 1/2
- Produces: `Reasoner` with `MlxPipeline` backend

- [ ] Update struct and constructor:

```rust
// Before:
pub struct Reasoner {
    pipeline: crate::inference::llama::LlamaPipeline,
}
// After:
pub struct Reasoner {
    pipeline: crate::inference::mlx_pipeline::MlxPipeline,
}

// impl:
impl Reasoner {
    pub fn new(pipeline: crate::inference::mlx_pipeline::MlxPipeline) -> Self {
        Self { pipeline }
    }
    // reason() and extract_facts() unchanged — they call pipeline.infer()
}
```

- [ ] Commit and verify: `cargo check`

```bash
git add src-tauri/src/inference/reasoner.rs
git commit -m "refactor(inference): update Reasoner to use MlxPipeline"
```

---

### Task 4: Update ModelInfo for MLX

**Files:**

- Modify: `src-tauri/src/inference/model_registry.rs`

**Interfaces:**

- Consumes: MLX model naming conventions (e.g., "qwen3.5-4b-4bit")
- Produces: `ModelInfo` with MLX fields

- [ ] Update `ModelInfo` struct:

```rust
// Before:
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub source: String,
    pub quantization: String,
    pub context_length: usize,
    pub downloaded: bool,
    pub local_path: String,
}

// After:
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub source: String,
    pub mlx_model_name: String,  // e.g., "qwen3.5-4b-4bit"
    pub dtype: String,           // e.g., "float16"
    pub context_length: usize,
    pub downloaded: bool,
    pub local_path: String,      // local directory or empty for remote
}
```

- [ ] Update `ModelInfo::new()` default values:

```rust
pub fn new(id: String, name: String) -> Self {
    Self {
        id,
        name,
        source: "huggingface".to_string(),
        mlx_model_name: String::new(),
        dtype: "float16".to_string(),
        context_length: 4096,
        downloaded: false,
        local_path: String::new(),
    }
}
```

- [ ] Commit and verify: `cargo check`

```bash
git add src-tauri/src/inference/model_registry.rs
git commit -m "refactor(inference): update ModelInfo for MLX model fields"
```

---

### Task 5: Update DownloadedModel struct

**Files:**

- Modify: `src-tauri/src/lib.rs:252`

**Interfaces:**

- Consumes: MLX model naming
- Produces: `DownloadedModel` with `mlx_model_name` field

- [ ] Add `mlx_model_name` field to `DownloadedModel`:

```rust
pub struct DownloadedModel {
    pub id: String,
    pub filename: String,
    pub size: u64,
    pub path: String,
    pub mlx_model_name: String,  // e.g., "qwen3.5-4b-4bit"
}
```

- [ ] Commit and verify: `cargo check`

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add mlx_model_name to DownloadedModel"
```

---

### Task 6: Update backend model commands

**Files:**

- Modify: `src-tauri/src/commands/mod.rs` — functions `init_reasoner` (line ~1475),
  `validate_model` (line ~1462), `download_model` (line ~1442),
  `list_downloaded_models` (line ~1415), `is_model_loaded` (line ~1456)

**Interfaces:**

- Consumes: `MmlxPipeline` from Task 2
- Consumes: `DownloadedModel` updated in Task 5
- Produces: MLX-aware command handlers

- [ ] Update `init_reasoner` — remove `gpu_layers`, use `model_name`:

```rust
pub async fn init_reasoner(model_name: String, context_size: usize) -> Result<()> {
    crate::commands::init_pipeline(model_name, context_size).await
}
```

- [ ] Update `validate_model` — check `.safetensors` instead of `.gguf`:

```rust
pub async fn validate_model(model_path: String) -> Result<bool> {
    let path = std::path::Path::new(&model_path);
    if !path.exists() {
        return Ok(false);
    }
    // Also accept model names (no extension) since rapid-mlx uses aliases
    if path.extension().and_then(|e| e.to_str()) != Some("safetensors")
        && path.extension().is_some() {
        return Ok(false);
    }
    Ok(true)
}
```

- [ ] Update `download_model` — use `rapid-mlx pull`:

```rust
pub async fn download_model(repo_id: String, filename: String) -> Result<crate::DownloadedModel> {
    let model_name = if filename.is_empty() { repo_id } else { filename };
    let status = std::process::Command::new("rapid-mlx")
        .args(["pull", &model_name])
        .status()?;
    if !status.success() {
        return Err(AppError(format!("Failed to pull model: {}", model_name)));
    }
    Ok(crate::DownloadedModel {
        id: model_name.clone(),
        filename: model_name.clone(),
        size: 0,
        path: String::new(),
        mlx_model_name: model_name,
    })
}
```

- [ ] Update `list_downloaded_models` — call `rapid-mlx models`:

```rust
pub async fn list_downloaded_models() -> Result<Vec<crate::DownloadedModel>> {
    let output = std::process::Command::new("rapid-mlx")
        .arg("models")
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse model aliases from output (one per line)
    let models: Vec<_> = stdout
        .lines()
        .filter(|line| !line.starts_with("Available") && !line.starts_with("---") && !line.contains("Alias"))
        .map(|line| {
            let name = line.split_whitespace().next().unwrap_or("").to_string();
            crate::DownloadedModel {
                id: name.clone(),
                filename: name.clone(),
                size: 0,
                path: String::new(),
                mlx_model_name: name,
            }
        })
        .collect();
    Ok(models)
}
```

- [ ] Update `is_model_loaded` — check if rapid-mlx serve is running:

```rust
pub async fn is_model_loaded() -> Result<bool> {
    let client = reqwest::blocking::Client::new();
    match client.get("http://127.0.0.1:8000/health").send() {
        Ok(resp) => Ok(resp.status().is_success()),
        Err(_) => Ok(false),
    }
}
```

- [ ] Commit and verify: `cargo check` + `cargo clippy -- -D warnings`

```bash
git add src-tauri/src/commands/mod.rs
git commit -m "feat(commands): update model commands for rapid-mlx"
```

---

### Task 7: Update frontend types

**Files:**

- Modify: `src/lib/stores/app.ts` — `ModelConfig` interface (line ~17),
  `HardwareConfig` interface (line ~26, remove `recommended_gpu_layers`)

**Interfaces:**

- Consumes: Backend commands using `model_name` instead of `model_path`

- [ ] Update `ModelConfig`:

```typescript
// Before:
export interface ModelConfig {
	source: 'huggingface' | 'local';
	id: string;
	quantization: string;
	context_length: number;
	downloaded: boolean;
	local_path: string;
}

// After:
export interface ModelConfig {
	source: 'huggingface' | 'local';
	id: string;
	mlx_model_name: string; // e.g., "qwen3.5-4b-4bit"
	dtype: string; // e.g., "float16"
	context_length: number;
	downloaded: boolean;
	local_path: string; // may be empty for remote models
}
```

- [ ] Update `HardwareConfig` — remove `recommended_gpu_layers`:

```typescript
// Remove:
recommended_gpu_layers: number;
```

- [ ] Commit and verify: `npm run check`

```bash
git add src/lib/stores/app.ts
git commit -m "refactor(frontend): update types for MLX model config"
```

---

### Task 8: Update frontend components

**Files:**

- Modify: `src/routes/settings/+page.svelte` — file dialog filter, placeholder
- Modify: `src/routes/analysis/+page.svelte` — `init_reasoner` invoke call

**Interfaces:**

- Consumes: Backend `init_reasoner` now takes `modelName` instead of `modelPath` + `gpuLayers`

- [ ] Update `analysis/+page.svelte` — remove `gpuLayers`, rename `modelPath` to `modelName`:

```typescript
// Before:
await invoke('init_reasoner', {
	modelPath,
	contextSize: $config?.model?.context_length || 8192,
	gpuLayers: 32
});

// After:
await invoke('init_reasoner', {
	modelName: modelPath,
	contextSize: $config?.model?.context_length || 8192
});
```

- [ ] Update `settings/+page.svelte` — remove `.gguf` file filter:

```typescript
// Before:
filters: [{ name: 'GGUF Models', extensions: ['gguf'] }];

// After: (remove the file dialog entirely or change to model selector dropdown)
// Since rapid-mlx uses model names, replace with dropdown of available models
```

- [ ] Commit and verify: `npm run check`

```bash
git add src/routes/analysis/+page.svelte src/routes/settings/+page.svelte
git commit -m "refactor(frontend): update invokes for MLX model handling"
```

---

### Task 9: Final audit — remove all GGUF/llama references

**Files:**

- All files in `src-tauri/src/` and `src/`

**Interfaces:**

- Consumes: All previous tasks completed
- Produces: Clean codebase with no GGUF/llama references

- [ ] Run audit:

```bash
grep -ri "gguf\|llama" src-tauri/src/ src/
grep -ri "gpu_layers\|gpuLayers" src-tauri/src/ src/
```

- [ ] Verify all checks pass:

```bash
cd src-tauri && cargo clippy -- -D warnings
cd ../.. && npm run check
```

- [ ] Update GitHub issues (close all 7 issues)

```bash
gh issue close 1 --comment "Completed in commit <hash>"
gh issue close 2 --comment "Completed in commit <hash>"
# ... repeat for all
```

- [ ] Commit final state

```bash
git add -A
git commit -m "chore: rapid-mlx swap complete — all GGUF/llama references removed"
git push origin main
```
