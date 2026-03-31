# Development Workflow

## Task Runner

SL Studio uses `mise` as its task runner. All common commands are defined in `mise.toml`.

## Common Tasks

### Development

```bash
mise run dev              # Start dev server + Tauri app
mise run dev_frontend     # Start frontend only (Vite)
mise run dev_tauri        # Start Tauri app only
```

### Building

```bash
mise run build            # Build frontend + Tauri app
mise run build_frontend   # Build frontend only
mise run build_tauri      # Build Tauri app only
```

### Testing

```bash
mise run test             # Run all Rust tests
mise run test_quick       # Quick test run
mise run e2e              # Run Playwright E2E tests
mise run e2e_ui           # E2E tests with UI
```

### Linting & Formatting

```bash
mise run lint             # Run ESLint + clippy
mise run format           # Format code (Prettier + rustfmt)
mise run format:check     # Check formatting
mise run check            # TypeScript type check
```

### CI Pipeline

```bash
mise run ci               # Run full CI pipeline
mise run ci_frontend      # Frontend CI checks
mise run ci_rust          # Rust CI checks
mise run ci_test          # Rust tests
```

### Benchmarks

```bash
mise run benchmark          # Run all benchmarks
mise run benchmark_quick    # Quick benchmark
mise run benchmark_search   # Search parsing only
mise run benchmark_entities # Entity overlap only
mise run benchmark_strings  # String operations only
mise run benchmark_collections # Collection operations only
```

### Release Bundles

```bash
mise run release_dmg      # Build macOS DMG
mise run release_deb      # Build Linux DEB
mise run release_appimage # Build Linux AppImage
mise run release_msi      # Build Windows MSI
```

### Database

```bash
mise run db_inspect       # Inspect database schema
```

### Watch Mode

```bash
mise run watch            # Watch mode with bacon
```

## Debugging

### Backend Debugging

```bash
# Enable debug logging
RUST_LOG=debug mise run dev

# Trace level for slstudio only
RUST_LOG=slstudio=trace mise run dev
```

### Frontend Debugging

- Open DevTools in the Tauri webview (Cmd+Option+I on macOS)
- Use `console.log()` for quick debugging
- Use the PerformanceMonitor component for performance metrics

### Database Inspection

```bash
# Open registry database
sqlite3 ~/.local/share/slstudio/registry.db

# Open intelligence database
sqlite3 ~/.local/share/slstudio/intelligence.db
```

## Code Style

### Rust

- Follow `rustfmt` defaults
- No `unsafe_code` (forbidden in Cargo.toml)
- All clippy warnings must be resolved
- MSRV: 1.75

### TypeScript/Svelte

- Tabs for indentation
- Single quotes
- No trailing commas
- 100 character print width
- Strict TypeScript mode

## Commit Messages

Follow conventional commits:

```
feat: add entity resolution
fix: correct z-score calculation
docs: update API reference
chore(release): bump version to v0.2.0
```
