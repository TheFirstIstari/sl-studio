# AGENTS.md — SL Studio Developer Reference

## Project Overview

SL Studio is a Tauri 2 (Rust) + SvelteKit 5 desktop application for forensic document analysis.
The Rust backend lives in `src-tauri/`, the SvelteKit frontend in `src/`.

---

## Prerequisites

- Rust stable toolchain (`rustup toolchain install stable`)
- Node.js ≥ 18 + npm
- Tauri CLI v2 (installed as an npm devDependency — use `npm run tauri`)
- On macOS Apple Silicon: Metal SDK (ships with Xcode Command Line Tools)

---

## Common Commands

### Frontend

```bash
npm install               # install JS deps
npm run dev               # Vite dev server only (no Tauri shell)
npm run build             # production Vite build
npm run check             # svelte-check TypeScript/Svelte type checking
npm run lint              # ESLint
npm run lint:fix          # ESLint with auto-fix
npm run format            # Prettier write
npm run format:check      # Prettier check
```

### Backend (Rust)

```bash
# Run from repo root — the npm script sets required env vars for Apple Silicon
npm run tauri dev         # full Tauri dev build + hot-reload frontend
npm run tauri build       # production release build

# Or run cargo directly from src-tauri/
cd src-tauri
cargo check               # fast type+borrow check (no codegen)
cargo clippy -- -D warnings   # lints
cargo test                # unit tests
cargo bench               # criterion benchmarks (database_bench)
```

### Type-check everything

```bash
npm run check && cd src-tauri && cargo check
```

---

## Architecture

```
src/                      SvelteKit 5 frontend (Runes API / $state)
  lib/
    components/           Shared UI components (PageHeader, StatCard, FilterBar)
    stores/               Svelte stores (workflow, loading, error)
    utils.ts              Shared helpers: getSeverityColor, getCategoryIcon,
                          getQualityBadgeColor, formatFileSize
    styles/
      theme.css           CSS custom properties (--color-severity-*, etc.)
  routes/                 One folder per page

src-tauri/
  src/
    commands/             Tauri command handlers (one file per domain)
      mod.rs              require_db() helper — always use this instead of
                          manually locking AppState.db
    core/
      database.rs         SQLite pool + migrations
      queries/            Domain query modules (analytics, timeline, …)
    extractors/           PDF / image / audio / DOCX extraction
    inference/            llama.cpp pipeline, reasoner, model registry
    lib.rs                AppState definition + Tauri builder
```

### Key patterns

- **`require_db(state)`** (`commands/mod.rs`) — returns `Arc<Database>` or a
  string error; use in every command that touches the DB instead of manual
  lock/check boilerplate.
- **Svelte 5 `$state` Sets/Maps** — mutate by reassigning
  (`x = new Set([...x, item])`) not in-place (`.add()` won't trigger reactivity).
- **CSS vars** — use `var(--color-severity-high)` etc. from `theme.css`; never
  hardcode hex colors in component files.

---

## Testing

- Rust unit tests: `cargo test` inside `src-tauri/`
  - If `database_test` tests fail intermittently when run in parallel, use
    `cargo test -- --test-threads=1` to serialise them (each test uses its own
    `TempDir` but concurrent I/O can cause flakiness on some machines).
- Frontend E2E (Playwright): `npm test` — requires a built app
- No integration test suite yet; manual smoke-test via `npm run tauri dev`

---

## Verification Before Committing

1. `cd src-tauri && cargo check` — must be clean
2. `npm run check` — must be clean
3. `npm run lint` — no errors
4. Optional: `npm run format:check`
