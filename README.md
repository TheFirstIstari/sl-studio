# SL Studio - Forensic Document Analysis Platform

**v0.3.2** — A high-performance desktop application for forensic document analysis, built with Tauri 2 + Rust + SvelteKit 5.

All processing runs locally — no cloud dependencies.

## Features

- Ingest PDFs, images, audio, DOCX, and plain-text evidence files
- Local LLM inference (rapid-mlx, MLX models, Metal GPU on Apple Silicon)
- Multi-pass analysis pipelines (Basic Facts, Financial Crimes, Document Analysis, OCR, Audio)
- Document metadata extraction: EXIF, PDF properties, language detection, structured fields
- Fact deduplication, quality scoring, annotation, and tagging
- Timeline, network graph, geographic map, and statistics visualizations
- Export to JSON, CSV, PDF, and Excel
- Backup/restore with optional evidence file bundling

## Documentation

See [docs/README.md](docs/README.md) for comprehensive documentation.

### Quick Links

- [Getting Started](docs/development/getting-started.md)
- [System Architecture](docs/architecture/system.md)
- [Tauri Commands API](docs/api/tauri-commands.md)
- [Database Schema](docs/database/schema.md)
- [Release Process](docs/deployment/release.md)
- [Contributing](docs/development/contributing.md)
- [AGENTS.md](AGENTS.md) — build/test commands and architecture notes for contributors

## Quick Start

```bash
# Install JS dependencies
npm install

# Start full Tauri dev build (Rust + frontend hot-reload)
npm run tauri dev

# Type-check frontend
npm run check

# Type-check backend
cd src-tauri && cargo check
```

> **mise users:** `mise run setup && mise run dev` also works if you have mise installed.

## License

AGPL-3.0-only (see LICENSE file)
