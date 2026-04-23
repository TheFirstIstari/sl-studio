# Model Management

## Overview

The models module (`models/mod.rs`, ~208 lines) manages GGUF model files for local LLM inference.

## ModelManager

```rust
struct ModelManager {
    models_dir: PathBuf,
}
```

### Methods

| Method                          | Description                       |
| ------------------------------- | --------------------------------- |
| `list_models()`                 | List all GGUF models in directory |
| `select_model(path)`            | Set active model                  |
| `delete_model(path)`            | Remove a model file               |
| `detect_quantization(filename)` | Detect quantization from filename |

## Quantization Types

```rust
enum Quantization {
    Q4_0,
    Q4_1,
    Q4_K_M,
    Q4_K_S,
    Q5_0,
    Q5_1,
    Q5_K_M,
    Q5_K_S,
    Q6_K,
    Q8_0,
    F16,
    F32,
}
```

### Quantization Comparison

| Type   | Size     | Quality   | Speed   |
| ------ | -------- | --------- | ------- |
| Q4_K_M | ~4-5GB   | Good      | Fast    |
| Q5_K_S | ~5-6GB   | Better    | Medium  |
| Q8_0   | ~8-10GB  | Best      | Slow    |
| F16    | ~14-16GB | Excellent | Slowest |

## Model Download

Models can be downloaded from HuggingFace via the Settings page:

1. Browse available models
2. Select quantization level
3. Download with progress bar
4. Model saved to `models_dir`

## Model Selection

Users can select from downloaded models in Settings:

- List of available GGUF files
- Model info (size, quantization)
- Active model indicator
- Load/unload controls

## Recommended Models

For forensic document analysis with structured JSON extraction, these models are recommended:

### Primary Recommendations

| Model                   | Size (Q4) | HuggingFace ID                       | Use Case                           |
| ----------------------- | --------- | ------------------------------------ | ---------------------------------- |
| **Qwen2.5-7B-Instruct** | ~4.5GB    | `Qwen/Qwen2.5-7B-Instruct-GGUF`      | Primary choice - clean JSON output |
| **Qwen2.5-3B-Instruct** | ~2GB      | `Qwen/Qwen2.5-3B-Instruct-GGUF`      | Lightweight option                 |
| **Phi-4 Mini**          | ~2.5GB    | `bartowski/Phi-4-mini-instruct-GGUF` | Good reasoning capabilities        |

### Alternative Models

| Model         | Size (Q4) | HuggingFace ID                         | Notes                                |
| ------------- | --------- | -------------------------------------- | ------------------------------------ |
| Phi-4 Mini    | ~2.5GB    | `bartowski/Phi-4-mini-instruct-GGUF`   | Good reasoning capabilities          |
| Llama 3.2 3B  | ~2GB      | `bartowski/Llama-3.2-3B-Instruct-GGUF` | General purpose, needs prompt tuning |
| Qwen 2.5 1.5B | ~1GB      | `bartowski/Qwen2.5-1.5B-Instruct-GGUF` | Lightweight option                   |

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

- **NuExtract 2.0**: Vision-only model - not supported by llama.cpp-rs backend
- **General instruction models**: May not follow JSON schema strictly without additional prompt engineering
