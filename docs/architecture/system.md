# System Architecture

## Overview

SL Studio is a desktop application built with **Tauri 2** (Rust backend) and **SvelteKit 5** (frontend). It processes evidence files through extraction and AI-powered reasoning pipelines to extract structured facts for forensic investigations. All processing runs locally with no cloud dependencies.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                        SL Studio Desktop App                       │
├──────────────────────────┬──────────────────────────────────────────┤
│      Frontend (Web)      │         Backend (Rust/Tauri)            │
│                          │                                          │
│  ┌────────────────────┐  │  ┌────────────────────────────────────┐ │
│  │   SvelteKit 5 SPA  │  │  │         Tauri Command Hub          │ │
│  │                    │  │  │  (60+ commands via IPC)            │ │
│  │  ┌──────────────┐  │  │  └────────────────┬───────────────────┘ │
│  │  │ 12 Pages     │  │  │                   │                     │
│  │  │ Dashboard    │  │  │  ┌────────────────┴──────────────────┐ │
│  │  │ Analysis     │  │  │  │           Core Modules            │ │
│  │  │ Results      │  │  │  │                                   │ │
│  │  │ Timeline     │  │  │  │  ┌─────────┐  ┌───────────────┐  │ │
│  │  │ Statistics   │  │  │  │  │  Core   │  │  Extractors   │  │ │
│  │  │ Network      │  │  │  │  │(DB+Reg) │  │(PDF/OCR/Audio)│  │ │
│  │  │ Maps         │  │  │  │  └─────────┘  └───────────────┘  │ │
│  │  │ Anomalies    │  │  │  │                                   │ │
│  │  │ Compare      │  │  │  │  ┌─────────┐  ┌───────────────┐  │ │
│  │  │ Export       │  │  │  │  │Inference│  │     GPU       │  │ │
│  │  │ Backup       │  │  │  │  │(LLM)    │  │  (Detection)  │  │ │
│  │  │ Settings     │  │  │  │  └─────────┘  └───────────────┘  │ │
│  │  └──────────────┘  │  │  │                                   │ │
│  │                    │  │  │  ┌─────────┐  ┌───────────────┐  │ │
│  │  ┌──────────────┐  │  │  │  │ Config  │  │    Models     │  │ │
│  │  │ Chart.js     │  │  │  │  └─────────┘  └───────────────┘  │ │
│  │  │ Cytoscape.js │  │  │  └───────────────────────────────────┘ │
│  │  │ Leaflet.js   │  │  │                                        │
│  │  └──────────────┘  │  │  ┌───────────────────────────────────┐ │
│  └────────────────────┘  │  │        SQLite Databases           │ │
│                          │  │  (Registry + Intelligence)        │ │
└──────────────────────────┴──┴───────────────────────────────────┘
```

## Component Overview

### Frontend (SvelteKit 5)

The frontend is a Single Page Application (SPA) served via Tauri's embedded webview. It consists of:

- **12 pages**: Dashboard, Analysis, Results, Timeline, Statistics, Network, Maps, Anomalies, Compare, Export, Backup, Settings
- **Visualization libraries**: Chart.js (charts), Cytoscape.js (network graphs), Leaflet.js (geographic maps)
- **State management**: Component-level state with Tauri invoke calls for backend communication
- **Dark theme**: Optimized for forensic analysis work environments

### Backend (Rust)

The backend is organized into 7 core modules:

| Module       | Purpose                                                                  |
| ------------ | ------------------------------------------------------------------------ |
| `core`       | Database operations (SQLite) and registry scanning (file fingerprinting) |
| `extractors` | Text extraction from PDFs, images (OCR), audio, and documents            |
| `inference`  | LLM-powered reasoning pipeline with multi-pass analysis                  |
| `gpu`        | Hardware detection (CPU/GPU) and auto-scaling parameters                 |
| `config`     | Application, project, and model configuration management                 |
| `models`     | GGUF model management (download, list, select, delete)                   |
| `utils`      | File utilities, structured logging, path helpers                         |

### Data Flow

```
Evidence Files
     │
     ▼
┌─────────────┐
│ Registry    │ ← SHA-256 fingerprinting (parallel via rayon)
│ Scanner     │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Extractors  │ ← PDF/OCR/Audio/Document text extraction
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Reasoner    │ ← LLM inference with multi-pass pipelines
│ (LLM)       │ ← Quality scoring + deduplication
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ SQLite DB   │ ← Facts, entities, annotations, chains
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Frontend    │ ← Visualization, search, export
└─────────────┘
```

## Processing Priority Order

1. **New files** - Never processed before
2. **Modified files** - Fingerprint changed since last scan
3. **Extracted files** - Text extracted but not yet analyzed by LLM
4. **Rerun files** - Explicitly requested reprocessing

## Key Design Decisions

- **Local-only processing**: No cloud dependencies for forensic integrity
- **SQLite databases**: Lightweight, portable, no external database server required
- **Parallel processing**: rayon for CPU-bound operations (fingerprinting, extraction)
- **Incremental processing**: Only reprocess changed files to save time
- **Multi-pass pipelines**: Different analysis passes for different file types and investigation needs
- **Quality scoring**: Automatic assessment of extraction quality with retry recommendations
