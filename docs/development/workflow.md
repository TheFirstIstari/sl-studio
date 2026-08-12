# Development Workflow

## Task Runner

SL Studio uses [mise](https://mise.run) as its task runner. All common commands
are defined in `mise.toml`. Run `mise tasks` from the repo root to see the full
list, or use `mise run <task-name>`.

## Common Tasks

### Development

```bash
mise run dev              # Start Tauri dev server + Vite hot-reload
```

### Building

```bash
mise run build            # Production Tauri build (npm run tauri build)
```

### Testing

```bash
mise run test             # TypeScript checks + E2E tests
mise run test_types       # TypeScript type check only
mise run test_rust        # Rust unit tests only
mise run test_e2e         # Playwright E2E tests (chromium, firefox, webkit)
```

### Linting & Formatting

```bash
mise run lint             # ESLint + clippy
mise run lint_types       # npm run check + npm run lint
mise run lint_clippy      # cargo clippy -- -D warnings
mise run format           # Prettier + rustfmt check (format:check + fmt --check)
mise run format_fix       # Auto-fix: npm run format + cargo fmt
```

### CI Pipeline (mirrors `.github/workflows/ci.yml`)

```bash
mise run run              # Full CI pass: format + frontend + rust CI + tests (alias: ci, check)
mise run ci_rust          # Rust CI: fmt --check + clippy + build --release
mise run ci_test          # Rust tests: cargo test
mise run precommit        # Pre-commit: format check + type/lint (alias: pc)
mise run prepush          # Pre-push: full CI (alias: push)
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

### Utilities

```bash
mise run setup            # Install deps: cargo fetch + npm install
mise run clean            # Clean build artifacts
mise run open             # Open built application
```

## Code Style

### Rust

- Follow `rustfmt` defaults
- No `unsafe_code` (forbidden in Cargo.toml)
- All clippy warnings must be resolved (`-D warnings`)
- Use stable toolchain (managed by mise)

### TypeScript/Svelte

- Tabs for indentation
- Single quotes
- No trailing commas
- 100 character print width
- Strict TypeScript mode
- Svelte 5 runes syntax

## Commit Messages

Follow conventional commits:

```
feat: add entity resolution
fix: correct z-score calculation
docs: update API reference
chore(release): bump version to v0.3.0
perf: optimize search query parsing
test: add backup/restore E2E tests
```
