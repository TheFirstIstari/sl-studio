#!/usr/bin/env bash
set -euo pipefail

# Unified CI runner: run frontend, rust, and tests in parallel when possible.

if command -v mise >/dev/null 2>&1; then
  echo "Running CI in parallel via mise..."
  (mise run ci_frontend) &
  (mise run ci_rust) &
  (mise run ci_test) &
  wait
  echo "Parallel mise CI finished."
else
  echo "mise not found. Running fallback checks serially."
  npm ci
  npm run check
  (cd src-tauri && cargo fmt --check)
  (cd src-tauri && cargo test --manifest-path Cargo.toml)
fi
