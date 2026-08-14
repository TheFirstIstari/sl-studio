# Configuration

## Overview

Configuration is managed through the `AppConfig` struct defined in `lib.rs`.
The `load_config` and `save_config` Tauri commands persist configuration to
`sl-studio-config.json` in the current working directory.

## Types

All config types are defined in `src-tauri/src/lib.rs`:

### AppConfig

```rust
pub struct AppConfig {
    pub version: String,
    pub project: ProjectConfig,
    pub model: ModelConfig,
    pub hardware: HardwareConfig,
    pub processing: ProcessingConfig,
}
```

### ProjectConfig

| Field           | Type   | Description                     |
| --------------- | ------ | -------------------------------- |
| `name`          | String | Project / app name               |
| `evidence_root` | String | Evidence files directory         |
| `registry_db`   | String | Registry database filename       |
| `intelligence_db` | String | Intelligence database filename |

### ModelConfig

| Field              | Type   | Description                                  |
| ------------------ | ------ | -------------------------------------------- |
| `source`           | String | `"local"` or `"rapid-mlx"`                    |
| `id`               | String | Model identifier                             |
| `mlx_model_name`   | String | MLX model name (e.g. `qwen3.5-4b-4bit`)      |
| `dtype`            | String | Data type (`float16`, `bfloat16`)            |
| `context_length`   | usize  | LLM context window size                      |
| `downloaded`       | bool   | Whether the model has been pulled            |

### HardwareConfig

| Field                 | Type   | Description                     |
| --------------------- | ------ | ------------------------------- |
| `gpu_backend`         | String | Detected GPU backend            |
| `gpu_memory_fraction` | f64    | Fraction of GPU memory to use   |
| `cpu_workers`         | usize  | Number of CPU workers           |
| `auto_scale_workers`  | bool   | Auto-scale worker count          |
| `batch_size`          | usize  | Processing batch size            |
| `auto_scale_batch`    | bool   | Auto-scale batch size            |
| `ocr_provider`        | String | OCR engine (`ocrs`)              |
| `whisper_size`        | String | Whisper model size (`base`)      |
| `whisper_model_path`  | Option | Optional whisper model path      |

### ProcessingConfig

| Field                | Type   | Description                   |
| -------------------- | ------ | ----------------------------- |
| `batch_size`         | usize  | Processing batch size         |
| `max_image_resolution` | usize | Maximum image resolution for OCR |

## Persistence

Configuration is stored as JSON in `sl-studio-config.json`:

- **Default location**: Current working directory
- **Format**: Pretty-printed JSON via `serde_json::to_string_pretty`
- **Behavior**: `load_config` returns defaults if no file exists; `save_config` writes the full config
