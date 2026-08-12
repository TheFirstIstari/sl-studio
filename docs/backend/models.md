# Model Management

## Overview

The models module (`commands/mod.rs`, model management section) manages MLX models for local LLM inference via the `rapid-mlx` CLI.

## ModelManager

Model management is handled through Tauri commands that invoke `rapid-mlx`:

```rust
struct ModelManager {
    models_dir: PathBuf,
}
```

### Methods

| Method                          | Description                          |
| ------------------------------- | ------------------------------------ |
| `list_models()`                 | List all available MLX models        |
| `select_model(model_name)`      | Set active MLX model                 |
| `download_model(repo_id, filename)` | Pull model via `rapid-mlx pull`   |
| `validate_model(model_name)`    | Validate model name or `.safetensors` file |

## Data Types

```rust
enum DType {
    Float16,
    BFloat16,
}
```

### Data Type Comparison

| Type       | Size     | Quality   | Speed   |
| ---------- | -------- | --------- | ------- |
| Float16    | ~4-5GB   | Good      | Fast    |
| BFloat16   | ~5-6GB   | Better    | Medium  |

## Model Download

Models can be downloaded via `rapid-mlx pull` from HuggingFace:

1. Browse available MLX models
2. Select data type (float16 or bfloat16)
3. Download with progress bar
4. Model cached by `rapid-mlx`

## Model Selection

Users can select from available MLX models in Settings:

- List of available MLX models
- Model info (size, dtype)
- Active model indicator
- Load/unload controls

## Recommended Models

For forensic document analysis with structured JSON extraction, these models are recommended:

### Primary Recommendations

| Model                   | Size    | MLX Model Name          | Use Case                           |
| ----------------------- | ------- | ----------------------- | ---------------------------------- |
| **Qwen 3.5 4B (4-bit)** | ~2.5GB  | `qwen3.5-4b-4bit`       | Recommended for 16GB Macs          |
| **Qwen 3.5 9B (4-bit)** | ~5.5GB  | `qwen3.5-9b-4bit`       | Better quality for 32GB+ Macs      |
| **Bonsai 27B (2-bit)**  | ~8GB    | `bonsai-27b-2bit`       | High quality, requires 64GB+ RAM   |

### Model Configuration

For best JSON extraction results:

```json
{
	"temperature": 0.0,
	"max_tokens": 2048,
	"top_p": 0.95,
	"repeat_penalty": 1.1
}
```

### Known Limitations

- **NuExtract 2.0**: Vision-only model - not supported by MLX inference
- **General instruction models**: May not follow JSON schema strictly without additional prompt engineering
