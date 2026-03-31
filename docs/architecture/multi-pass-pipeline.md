# Multi-Pass LLM Pipeline

## Architecture

SL Studio supports configurable multi-pass analysis pipelines. Each pipeline consists of one or more passes, where each pass applies a different prompt template and output schema to extract specific types of information.

### Core Structures

```rust
struct Pipeline {
    id: String,
    name: String,
    description: String,
    is_file_type_specific: bool,
    file_type_filter: Option<String>,
    passes: Vec<PipelinePass>,
    sample_size: usize,
    is_builtin: bool,
}

struct PipelinePass {
    name: String,
    description: String,
    prompt_template: String,
    output_schema: String,
}
```

### Pipeline Execution Flow

```
Input Text
    │
    ▼
┌─────────────┐
│ Pass 1      │ ← Basic facts extraction
│ (prompt +   │
│  schema)    │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Pass 2      │ ← Entity extraction (optional)
│ (prompt +   │
│  schema)    │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Pass N      │ ← Pattern analysis (optional)
│ (prompt +   │
│  schema)    │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Dedup +     │ ← Merge results, remove duplicates
│ Quality     │ ← Score extraction quality
└─────────────┘
```

## Built-in Pipelines

### Basic Facts (1 pass) - All file types

- **Name**: Basic Facts
- **Description**: Extract fundamental facts from any document
- **Passes**: 1
  - Pass 1: Extract facts with category, severity, confidence, date, and quotes

### Financial Crimes (2 passes) - All file types

- **Name**: Financial Crimes
- **Description**: Two-pass analysis for financial investigations
- **Passes**: 2
  - Pass 1: Entity extraction (persons, organizations, accounts, amounts)
  - Pass 2: Pattern identification (money laundering, fraud indicators, suspicious transactions)

### Document Analysis (3 passes) - PDF/DOC only

- **Name**: Document Analysis
- **File Type Filter**: PDF, DOCX
- **Passes**: 3
  - Pass 1: Key facts and claims
  - Pass 2: Entity relationships
  - Pass 3: Timeline and sequence analysis

### Image OCR Analysis (2 passes) - Images only

- **Name**: Image OCR Analysis
- **File Type Filter**: Image files
- **Passes**: 2
  - Pass 1: OCR text extraction and fact analysis
  - Pass 2: Visual element description

### Audio Transcription (2 passes) - Audio only

- **Name**: Audio Transcription
- **File Type Filter**: Audio files
- **Passes**: 2
  - Pass 1: Transcription and speaker identification
  - Pass 2: Content analysis and fact extraction

## Custom Pipelines

Users can create custom pipelines through the Settings page:

1. Define pipeline name and description
2. Add passes with custom prompt templates
3. Specify output schemas (JSON Schema format)
4. Optionally filter by file type
5. Set sample size for testing

## Text Sampling

For large documents, the pipeline uses intelligent sampling:

- **Sample size**: Configurable per pipeline (default: 10,000 characters)
- **Strategy**: Distributed sampling across the document
- **Purpose**: Quick preview of extraction quality before full processing
