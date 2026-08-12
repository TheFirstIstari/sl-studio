# Inference Engine

## Overview

The inference module handles LLM-powered reasoning through a multi-pass pipeline architecture.

## Components

### MlxPipeline

`inference/mlx_pipeline.rs` (~87 lines)

Wrapper for rapid-mlx integration:

```rust
pub struct MlxPipeline {
    pub model_name: String,
    pub context_length: usize,
    pub server_url: String,
    pub child: Option<Child>,
}
```

The `MlxPipeline` spawns `rapid-mlx serve <model_name>` as a subprocess and
communicates via an OpenAI-compatible HTTP API. It handles startup polling,
health checks, and cleanup (kills the subprocess on `Drop`).

### PipelineRunner

`PipelineRunner` (`inference/pipeline.rs`, ~355 lines)

Executes multi-pass analysis pipelines:

```rust
struct PipelineRunner {
    model: MlxPipeline,
    pipeline: Pipeline,
}
```

#### Key Methods

- `run(text: &str)` - Execute all passes on input text
- `build_prompt(pass: &PipelinePass, text: &str)` - Build prompt with template
- `parse_json(response: &str)` - Extract structured facts from LLM response

### Reasoner

`inference/reasoner.rs` (~380 lines)

Combines extraction with LLM inference:

```rust
struct Reasoner {
    deconstructor: Deconstructor,
    pipeline: MlxPipeline,
    system_prompt: String,
}
```

#### Key Methods

- `analyze_file(file_path: &Path)` - Full file analysis pipeline
- `analyze_text(text: &str)` - Analyze pre-extracted text
- `chunk_text(text: &str)` - Split text into manageable chunks

## Processing Flow

```
File Path
    │
    ▼
┌─────────────┐
│ Deconstructor│ ← Extract text
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Chunk Text  │ ← Split into manageable pieces
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Build Prompt│ ← Template + schema + system prompt
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ LLM Inference│ ← Run via rapid-mlx
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Parse JSON  │ ← Extract facts from response
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Deduplicate │ ← Remove duplicate facts
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Score Quality│ ← Assess extraction quality
└──────┬──────┘
       │
       ▼
   Facts DB
```

## System Prompt

The default system prompt configures the LLM for forensic analysis:

- Extract structured facts from evidence documents
- Categorize by crime type, severity, and confidence
- Include direct quotes with page references
- Identify entities (persons, organizations, locations, dates, amounts)

## Prompt Templates

Located in `inference/prompts/`:

| File                     | Purpose                     |
| ------------------------ | --------------------------- |
| `basic_facts.txt`        | Basic fact extraction       |
| `financial_entities.txt` | Financial entity extraction |
| `financial_patterns.txt` | Financial pattern detection |

## Output Schemas

Located in `inference/schemas/`:

| File            | Purpose                  |
| --------------- | ------------------------ |
| `facts.json`    | Fact extraction schema   |
| `entities.json` | Entity extraction schema |
| `patterns.json` | Pattern detection schema |
