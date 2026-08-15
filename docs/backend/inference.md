# Inference Engine

## Overview

The inference module handles LLM-powered reasoning. An `MloxPipeline` wraps the
`rapid-mlx serve` subprocess and communicates via an OpenAI-compatible HTTP API.
The `Reasoner` wraps `MloxPipeline` and provides `extract_facts()` which builds a
prompt, calls `pipeline.infer()`, and wraps the LLM response into `Fact` structs.

## Components

### MlxPipeline

`inference/mlx_pipeline.rs` (~91 lines)

Wrapper around the `rapid-mlx serve <model_name>` subprocess. Communicates via
an OpenAI-compatible HTTP API on the local server URL.

```rust
pub struct MlxPipeline {
    pub model_name: String,
    pub context_length: usize,
    pub server_url: String,
    pub child: Option<Child>,
}
```

#### Key Methods

- `new(model_name, context_length)` - Create pipeline config (does not spawn subprocess)
- `load()` - Spawn `rapid-mlx serve <model_name>`, poll `/health` endpoint until ready (up to 30s)
- `infer(prompt, max_tokens)` - POST to `/v1/chat/completions` and extract response content
- `Drop` impl - Kills the subprocess on cleanup

#### OpenAI-Compatible API

```
POST /v1/chat/completions
{
  "messages": [{"role": "user", "content": "..."}],
  "max_tokens": 2048
}

Response:
{
  "choices": [{
    "message": {"role": "assistant", "content": "..."}
  }]
}
```

### Reasoner

`inference/reasoner.rs` (~38 lines)

Combines `MlxPipeline` with fact extraction logic:

```rust
pub struct Reasoner {
    pipeline: crate::inference::mlx_pipeline::MlxPipeline,
}
```

#### Key Methods

- `new(pipeline)` - Create reasoner wrapping an `MlxPipeline`
- `reason(text)` - Run inference on input text
- `extract_facts(text)` - Run LLM inference, parse response into `Vec<Fact>`

The reasoner is stored in `AppState` as `Arc<Mutex<Option<Reasoner>>>` and
initialized on demand via the `init_reasoner` command.

## Processing Flow

```
Extracted Text
     │
     ▼
┌─────────────┐
│  Reasoner   │
│             │
│  ┌────────┐ │
│  │ Prompt │ │ ← Build prompt with instruction
│  └───┬────┘ │
│      ▼      │
│  ┌────────┐ │
│  │ LLM    │ │ ← Run inference via rapid-mlx (MlxPipeline)
│  └───┬────┘ │
│      ▼      │
│  ┌────────┐ │
│  │ Parse  │ │ ← Extract content from response
│  └───┬────┘ │
└──────┼──────┘
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

## Built-in Pipelines

`inference/mod.rs:6` — `get_builtin_pipelines()` returns two configurable
analysis pipelines:

| Pipeline         | Passes | Description                          |
| ---------------- | ------ | ------------------------------------ |
| Default Analysis | 3      | Text extraction → fact extraction → entity recognition |
| Deep Forensic    | 3      | OCR extraction → fact validation → timeline construction |

Each pipeline pass is defined by a `PipelinePass` struct with a prompt template,
output schema, max tokens, temperature, and sample size. Pipeline passes are
stored as JSON in the `pipelines` table.
