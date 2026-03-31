# Configuration

## Overview

Configuration is managed through three layers: app config, project config, and settings helpers.

## AppConfig

`config/model.rs` (~154 lines)

Application-wide configuration stored at `~/.local/share/slstudio/config.json`:

```rust
struct AppConfig {
    project: ProjectConfig,
    model: ModelConfig,
    hardware: HardwareConfig,
    processing: ProcessingConfig,
}
```

### ProjectConfig

| Field                  | Type   | Description                          |
| ---------------------- | ------ | ------------------------------------ |
| `default_evidence_dir` | String | Default directory for evidence files |
| `default_export_dir`   | String | Default export directory             |

### ModelConfig

| Field            | Type   | Description                              |
| ---------------- | ------ | ---------------------------------------- |
| `source`         | String | `huggingface` or `local`                 |
| `model_id`       | String | HuggingFace model ID or local path       |
| `quantization`   | String | Quantization type (Q4_K_M, Q5_K_S, etc.) |
| `context_length` | usize  | LLM context window size                  |

### HardwareConfig

| Field                 | Type   | Description                   |
| --------------------- | ------ | ----------------------------- |
| `gpu_backend`         | String | Metal, CUDA, Vulkan, etc.     |
| `gpu_memory_fraction` | f32    | Fraction of GPU memory to use |
| `cpu_workers`         | usize  | Number of CPU workers         |

### ProcessingConfig

| Field            | Type  | Description             |
| ---------------- | ----- | ----------------------- |
| `batch_size`     | usize | Processing batch size   |
| `ocr_resolution` | u32   | OCR image resolution    |
| `max_chunk_size` | usize | Maximum text chunk size |

## ProjectFile

`config/project.rs` (~149 lines)

Project files use the `.sls` format:

```rust
struct ProjectFile {
    version: String,
    created_at: String,
    modified_at: String,
    investigator: InvestigatorInfo,
    paths: ProjectPaths,
    model: ProjectModel,
    hardware: ProjectHardware,
    processing: ProjectProcessing,
    metadata: InvestigationMetadata,
}
```

### InvestigatorInfo

| Field         | Description         |
| ------------- | ------------------- |
| `name`        | Investigator name   |
| `agency`      | Agency/organization |
| `case_number` | Case identifier     |
| `notes`       | Additional notes    |

### ProjectPaths

| Field             | Description                |
| ----------------- | -------------------------- |
| `evidence_root`   | Evidence files directory   |
| `registry_db`     | Registry database path     |
| `intelligence_db` | Intelligence database path |
| `text_cache_dir`  | Extracted text cache       |
| `export_dir`      | Export output directory    |
| `models_dir`      | Local models directory     |

## Settings Helpers

`config/settings.rs` (~13 lines)

Simple wrapper functions for loading and saving configuration.
