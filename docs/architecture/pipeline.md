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

The `extract_metadata_from_path` function in `extractors/mod.rs` routes files to
specialized extractors based on file extension:

```
File Input
    │
    ▼
┌─────────────────────┐
│ extract_metadata_   │ ← Extension-based routing (extractors/mod.rs)
│ from_path()         │
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

The `Reasoner` wraps an `MloxPipeline` with an `MlxPipeline` to perform AI-powered analysis:

```
Extracted Text
     │
     ▼
┌─────────────┐
│  Reasoner   │
│             │
│  ┌────────┐ │
│  │ Chunk  │ │ ← Split text into manageable chunks
│  └───┬────┘ │
│      ▼      │
│  ┌────────┐ │
│  │ Prompt │ │ ← Build prompt with template + schema
│  └───┬────┘ │
│      ▼      │
│  ┌────────┐ │
│  │ LLM    │ │ ← Run inference via rapid-mlx
│  └───┬────┘ │
│      ▼      │
│  ┌────────┐ │
│  │ Parse  │ │ ← Extract JSON facts from response
│  └───┬────┘ │
│      ▼      │
│  ┌────────┐ │
│  │ Dedup  │ │ ← Remove duplicate facts
│  └───┬────┘ │
│      ▼      │
│  ┌────────┐ │
│  │ Score  │ │ ← Quality assessment
│  └───┬────┘ │
└──────┼──────┘
       │
       ▼
   Facts DB
```

### System Prompt

The default system prompt configures the LLM for forensic analysis:

- Extract structured facts from evidence documents
- Categorize by crime type, severity, and confidence
- Include direct quotes with page references
- Identify entities (persons, organizations, locations, dates, amounts)

### Fact Structure

Each extracted fact contains:

| Field         | Type           | Description                |
| ------------- | -------------- | -------------------------- |
| `id`          | UUID           | Unique identifier          |
| `fingerprint` | String         | Hash for deduplication     |
| `source_file` | String         | Original file path         |
| `page`        | Option<i32>    | Page number if applicable  |
| `quote`       | String         | Direct quote from source   |
| `summary`     | String         | Concise fact statement     |
| `category`    | String         | Crime/fact category        |
| `date`        | Option<String> | Associated date            |
| `severity`    | String         | Critical/High/Medium/Low   |
| `confidence`  | f64            | Confidence score (0.0-1.0) |
| `quality`     | f64            | Extraction quality score   |
