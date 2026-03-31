# SL Studio - Documentation

**SL Studio** is a forensic document analysis desktop application that processes evidence files (PDFs, images, audio/video, documents) through extraction and AI-powered reasoning pipelines to extract structured facts for forensic investigations. All processing runs locally -- no cloud dependencies.

## Documentation Index

### Architecture

- [System Architecture](architecture/system.md) - Overall system design, component diagram, data flow
- [Processing Pipeline](architecture/pipeline.md) - Text extraction and LLM inference stages
- [Multi-Pass LLM Pipeline](architecture/multi-pass-pipeline.md) - Pipeline architecture and built-in pipelines

### Backend (Rust)

- [Backend Overview](backend/overview.md) - Module structure and entry points
- [Database Layer](backend/database.md) - SQLite operations, schema, caching
- [Registry Scanner](backend/registry.md) - File fingerprinting and batch processing
- [Extractors](backend/extractors.md) - PDF, OCR, Audio, Document extraction
- [Inference Engine](backend/inference.md) - LLM integration and reasoner
- [Quality & Deduplication](backend/quality.md) - Scoring metrics and dedup strategies
- [Hardware Detection](backend/hardware.md) - GPU/CPU detection and auto-tuning
- [Configuration](backend/config.md) - App, project, and model configuration
- [Model Management](backend/models.md) - GGUF model loading and management
- [Utilities](backend/utils.md) - File ops, logging, paths

### Frontend (SvelteKit)

- [Frontend Overview](frontend/overview.md) - SvelteKit structure and routing
- [Pages](frontend/pages.md) - All 12 application pages
- [Components](frontend/components.md) - Shared components and utilities

### API

- [Tauri Commands](api/tauri-commands.md) - All 60+ commands exposed to frontend

### Database

- [Schema Reference](database/schema.md) - Complete table definitions and relationships
- [Search](database/search.md) - FTS5 full-text search with Boolean operators
- [Analysis Queries](database/analysis.md) - Network, anomaly, temporal analysis

### Deployment

- [CI/CD](deployment/ci-cd.md) - GitHub Actions pipeline configuration
- [Release Process](deployment/release.md) - Building and publishing releases
- [Self-Hosted Runners](deployment/runners.md) - Fedora/NixOS runner setup

### Testing

- [Unit Tests](testing/unit.md) - Rust test structure
- [E2E Tests](testing/e2e.md) - Playwright test coverage
- [Benchmarks](testing/benchmarks.md) - Criterion benchmark results and how to run them

### Performance

- [Performance Report](performance/report.md) - Benchmark results and optimization recommendations

### Development

- [Getting Started](development/getting-started.md) - Setup and first run
- [Development Workflow](development/workflow.md) - mise tasks, watch mode, debugging
- [Contributing](development/contributing.md) - Code style, PR process

### Project

- [Project Overview](project-overview.md) - Features, tech stack, getting started, database schema, migration notes
