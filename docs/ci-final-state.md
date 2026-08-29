# CI Pipeline & Main Protection — Final State

## Protection (Active)

`.github/workflows/branch-protection.yml` validates/enforces rules. `main` is protected:

- Changes must go through a pull request (`git push main` rejected: verified)
- Required status check: `CI`
- PR + passing `CI` required before any merge

Verification:

```
git push origin main  # REJECTED: "Changes must be made through a pull request. Required status check \"CI\" is expected."
```

## Branch State

    # Example output (will vary over time)
    main:                     <sha> <summary>
    optimise/ci-improvements: <sha> <summary>

Branch `optimise/ci-improvements` exists but has zero divergence from `main` (all work committed to `main` before protection locked). No PR exists for it because there's nothing new to merge — the protection mechanism is already active on `main`.

Future changes must follow:

```
git checkout -b feature/x
git commit ...
git push origin feature/x
gh pr create --base main --head feature/x
gh pr merge (after CI passes)
```

## CI Pipeline (Granular, Parallel, Artifact-Sharing)

`.github/workflows/ci.yml` — 1:1 mirror via `mise run run` (`mise.toml` defines `run` to match `.github/workflows/ci.yml`).

Pipeline:

```
frontend (ubuntu) → [upload: frontend]
  ├── rust-fmt (macOS ARM)  ├── rust-clippy (macOS ARM)  ├── rust-build (macOS ARM)  ├── tests-macos-arm (macOS ARM)
  │                         │                             │   [upload: rust-release]  │   [needs: frontend]
  │                         │                             │                             │   [download: rust-release (optional, for speed)]
  │                         │                             │                             │   [download: frontend]
  │                         │                             │                             │
  └─────────────────────────┴─────────────────────────────┴─────────────────────────────┘
  build-macos-arm (macOS ARM, needs all above) → [upload: tauri-aarch64-apple-darwin]
  release (ubuntu, needs build-macos-arm) → creates release
```

Redundancy eliminated: `rust-release` artifact passes compiled output from `rust-build` to `tests-macos-arm` and `build-macos-arm`, avoiding redundant `cargo build --release` in build step.

## Caching

Per rust job (`rust-fmt`, `rust-clippy`, `rust-build`, `tests-macos-arm`, `build-macos-arm`):

- `Swatinem/rust-cache@v2` (workspace-level target caching)
- `actions/cache@v4` (global `~/.cargo/registry/index`, `cache`, `git` — keyed by OS + `Cargo.lock` hash)

Frontend: `setup-node@v4` (`cache: 'npm'`) + `npm ci`.

## Supply Chain Minimisation

`src-tauri/Cargo.toml` — removed unused crates (`uuid`, `sha2`, `rayon`, `thiserror`). Active crates verified by grep in `src-tauri/src/`:

- `tauri` / plugins: used (`lib.rs`)
- `rusqlite`: used (`database.rs`, `commands/mod.rs`)
- `serde` / `serde_json`: used (serialization across backend)
- `chrono`: used (`reasoner.rs`, `commands/mod.rs`, `extractors/`)
- `tracing` / `tracing-subscriber`: used (`info!`, `fmt::init()`)
- `anyhow`: used (error handling)
- `num_cpus`: used (`commands/mod.rs`)
- `sysinfo`: used (`commands/mod.rs` — memory metrics)
- `reqwest`: used (`mlx_pipeline.rs`, `commands/mod.rs` — HTTP client for `rapid-mlx`)

## Local 1:1 Mirror

`mise run run` (`mise.toml`) mirrors `.github/workflows/ci.yml`. Verified clean:

```
mise run run → cargo fmt: ok, clippy: ok, build --release: ok, cargo test: ok, npm check: ok, npm lint: ok, npm format:check: ok, npm build: ok
```

## Future Workflow (Protection Active)

All commits must go through PRs:

1. `git checkout -b feature/x`
2. Make edits, run `mise run prepush` locally
3. `git push origin feature/x`
4. `gh pr create --base main --head feature/x`
5. CI (`CI` status check) must pass
6. `gh pr merge` (or merge via UI once `CI` is green)

No direct pushes to `main` allowed (`git push origin main` rejected).
