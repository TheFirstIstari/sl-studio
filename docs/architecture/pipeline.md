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
│    Stage 2: Analysis       │ ← LLM inference via llama.cpp
│   (Reasoner + LLM Model)   │
└────────────┬───────────────┘
             │
             ▼
         Facts DB
```

## Stage 1: Text Extraction

### Architecture

The `Deconstructor` orchestrates text extraction by routing files to specialized extractors based on file extension:

```
File Input
    │
    ▼
┌─────────────┐
│ Deconstructor│ ← Extension-based routing
└──────┬──────┘
       │
   ┌───┴───┬───────────┬──────────┐
   ▼       ▼           ▼          ▼
 PDF    OCR/Image   Audio      Document
 Ext    Ext         Ext        Ext
```

### Supported File Types

| Type      | Extensions                                       | Extractor           | Notes                                  |
| --------- | ------------------------------------------------ | ------------------- | -------------------------------------- |
| PDF       | `.pdf`                                           | `PdfExtractor`      | Uses `pdf-extract` crate               |
| Images    | `.png`, `.jpg`, `.jpeg`, `.tiff`, `.bmp`, `.gif` | `OcrExtractor`      | Uses `ocrs` crate with preprocessing   |
| Audio     | `.mp3`, `.wav`, `.m4a`, `.mp4`, `.ogg`, `.flac`  | `AudioExtractor`    | whisper-rs integration (stub)          |
| Documents | `.docx`                                          | `DocumentExtractor` | ZIP/XML parsing                        |
| Text      | `.txt`, `.md`                                    | `DocumentExtractor` | Direct reading with encoding detection |

### Chunking

Large files are split into chunks with overlap to maintain context:

- **Default chunk size**: Auto-scaled based on available memory
- **Overlap**: Configurable to prevent context loss at chunk boundaries
- **Quality assessment**: Each chunk is scored for extraction quality

### Quality Assessment

Each extraction is assessed for:

- **Character count**: Minimum viable text length
- **Word density**: Ratio of words to characters
- **Scanned detection**: Identifies scanned PDFs that may need OCR instead

### Error Handling

| Error Type               | Handling                             |
| ------------------------ | ------------------------------------ |
| Password-protected files | Returns descriptive error            |
| Corrupted files          | Returns error with file path         |
| Large files              | Fallback extraction with size limits |
| Unsupported formats      | Skipped with warning                 |

## Stage 2: LLM Inference

### Architecture

The `Reasoner` combines the `Deconstructor` with an `LlamaModel` to perform AI-powered analysis:

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
│  │ LLM    │ │ ← Run inference via llama.cpp
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
