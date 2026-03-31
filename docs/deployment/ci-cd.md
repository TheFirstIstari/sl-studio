# CI/CD Pipeline

## Overview

SL Studio uses GitHub Actions for continuous integration and deployment across multiple platforms.

## Workflow Structure

```
frontend (Ubuntu)
    │
    ├── rust-linux ── tests-linux ── build-linux ─┐
    ├── rust-macos-arm ── tests-macos-arm ── build-macos-arm ─┤
    ├── rust-macos-intel ── tests-macos-intel ── build-macos-intel ─┤
    ├── rust-windows ── tests-windows ── build-windows ─┤
    ├── rust-fedora ── tests-fedora ────────────────────────┤ (no build)
    ├── rust-nixos ── tests-nixos ──────────────────────────┘ (no build)
    │
    └── release (on version tags only)
```

## Triggers

| Event        | Condition           |
| ------------ | ------------------- |
| Push         | `main` branch       |
| Push         | Tags matching `v*`  |
| Pull Request | `main` branch       |
| Manual       | `workflow_dispatch` |

## Jobs

### Frontend

- **Runner**: `ubuntu-latest`
- **Steps**: Checkout, Node setup, install deps, type check, lint, format check, build
- **Artifact**: `frontend` (build directory)

### Per-Platform Pipelines

Each platform (Linux, macOS ARM, macOS Intel, Windows, Fedora, NixOS) has:

1. **Rust check**: Format check, clippy, build
2. **Tests**: `cargo test`
3. **Build** (GitHub-hosted only): Tauri app bundle

#### Platform Details

| Platform    | Runner                  | Bundles           |
| ----------- | ----------------------- | ----------------- |
| Linux       | `ubuntu-22.04`          | DEB, AppImage     |
| macOS ARM   | `macos-latest`          | DMG, APP          |
| macOS Intel | `macos-latest`          | DMG, APP          |
| Windows     | `windows-latest`        | MSI, EXE          |
| Fedora      | `[self-hosted, fedora]` | Rust + tests only |
| NixOS       | `[self-hosted, nixos]`  | Rust + tests only |

### Release

- **Runner**: `ubuntu-latest`
- **Trigger**: Tags starting with `v`
- **Condition**: `always() && !cancelled()` (runs even if some builds are skipped)
- **Steps**: Checkout, cleanup, download all artifacts, create GitHub Release

## Concurrency

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
```

Prevents duplicate runs on the same branch.

## Key Design Decisions

- **Per-platform isolation**: Each platform's build only depends on its own checks
- **Skipped-safe release**: Release runs even when some platforms are skipped (e.g., self-hosted runners offline)
- **Parallel execution**: All platform pipelines run simultaneously after frontend completes
- **Artifact sharing**: Frontend build is uploaded once and downloaded by all platform jobs
