#!/usr/bin/env bash
set -euo pipefail

# Cleanup script before making a production release.
# Safe: removes transient build outputs and caches; does not touch sources.

ROOT_DIR="${1:-.}"
if [ ! -d "$ROOT_DIR" ]; then
  ROOT_DIR="."
fi

echo "Cleaning release artifacts from ${ROOT_DIR}..."

# Remove common build outputs
rm -rf "$ROOT_DIR/.svelte-kit" "$ROOT_DIR/build" "$ROOT_DIR/dist" "$ROOT_DIR/target" "$ROOT_DIR/src-tauri/target" \
  "$ROOT_DIR/e2e/.playwright" "$ROOT_DIR/coverage" "$ROOT_DIR/.vitedit" 2>/dev/null || true

# Clear Node/NPM caches for a clean release
rm -rf "$ROOT_DIR/node_modules/.cache" 2>/dev/null || true
rm -rf "$ROOT_DIR/playwright-report" "$ROOT_DIR/e2e/.playwright" 2>/dev/null || true
rm -rf "$ROOT_DIR/.cache" 2>/dev/null || true

echo "Release cleanup complete."
