# Processing Pipeline

## Overview

SL Studio processes evidence files through a two-stage pipeline with parallel extraction:

1. **Stage 1: Text Extraction** - Extract raw text from various file formats (parallel with rayon)
2. **Stage 2: LLM Inference** - Analyze extracted text through AI-powered reasoning pipelines

Both stages run independently and can be resumed from checkpoints.

## Two-Stage Architecture

```
File Input
    │
    ▼
┌─────────────────────────────┐
│    Stage 1: Extraction     │ ← Parallel processing via rayon
│  (PDF, OCR, Audio, Doc)    │
└────────────┬───────────────┘
             │ Extracted Text
             ▼
┌─────────────────────────────┐
│    Stage 2: Analysis       │ ← LLM inference via rapid-mlx (MLX pipeline)
│   (Reasoner + LLM Model)   │
└────────────┬───────────────┘
             │
             ▼
         Facts DB
```

## Stage 1: Text Extraction

### Architecture

The `extract_batch` command in `commands/mod.rs` routes files to specialized
extractors based on file extension via a private `extract_file` helper:

```
File Input
    │
    ▼
┌─────────────────────┐
│ extract_file()      │ ← Extension-based routing (commands/mod.rs:1800)
│ (private helper)    │
└────────┬────────────┘
         │
   ┌────┴───┬───────────┬─────────┐
   ▼        ▼           ▼         ▼
 PDF      Image       Audio     DOCX
 extract_ extract_   extract_  extract_
 pdf()   image()      audio()    docx()
```

### Supported File Types

| Type      | Extensions                                       | Function                  | Notes                    |
| --------- | ------------------------------------------------ | ------------------------- | ------------------------ |
| PDF       | `.pdf`                                           | `extract_pdf`             | Stub (planned: `pdf-extract`) |
| Images    | `.png`, `.jpg`, `.jpeg`, `.tiff`, `.bmp`         | `extract_image`           | Stub (planned: `ocrs`)        |
| Audio     | `.mp3`, `.wav`, `.m4a`, `.flac`, `.aac`          | `extract_audio`           | Metadata extraction      |
| Documents | `.docx`                                          | `extract_docx`            | ZIP/XML parsing          |
| Text      | `.txt`, `.md`, `.json`, `.xml`, `.csv`           | `extract_metadata_from_path` | Direct reading        |

> **Note**: Extractors are currently stub implementations that read file content
> and return a `Metadata` struct with placeholder category/fact data. Full text
> extraction (OCR, PDF layout analysis, DOCX body parsing) is under development.

## Stage 2: LLM Inference

### Architecture

The `Reasoner` wraps an `MloxPipeline` to perform AI-powered fact extraction:

```
Extracted Text
     │
     ▼
┌─────────────┐
│  Reasoner   │
│             │
│  ┌────────┐ │
│  │ Prompt │ │ ← Build extraction prompt
│  └───┬────┘ │
│      ▼      │
│  ┌────────┐ │
│  │ LLM    │ │ ← Run inference via rapid-mlx
│  └───┬────┘ │
│      ▼      │
│  ┌────────┐ │
│  │ Parse  │ │ ← Wrap response into Fact struct
│  └───┬────┘ │
│      ▼      │
│  ┌────────┐ │
│  │ Score  │ │ ← Assign confidence & severity
│  └───┬────┘ │
└──────┼──────┘
       │
       ▼
   Facts DB
```

### System Prompt

The inference prompt is a simple instruction passed to the LLM via `Reasoner::extract_facts()`:

```
Extract facts and entities from: {text}
```

The LLM response is wrapped directly into a `Fact` struct. No system prompt, prompt
templates, or output schemas are currently used — these are planned for future pipeline
pass configuration. See the `Pipelines` section for configurable pipeline passes.

### Fact Structure

Each extracted fact contains:

| Field                | Type           | Description                |
| -------------------- | -------------- | -------------------------- |
| `id`                 | u64            | Unique identifier          |
| `fingerprint`        | String         | Hash for deduplication     |
| `filename`           | String         | Source file name           |
| `fact_summary`       | String         | LLM-generated fact statement |
| `category`           | Option<String> | Fact category (or null)    |
| `identified_crime`   | Option<String> | Crime type if detected     |
| `severity_score`     | u8             | Severity (0-10 scale)      |
| `confidence`         | Option<f64>    | Confidence (0.0-1.0)       |
| `created_at`         | String         | RFC3339 timestamp          |
