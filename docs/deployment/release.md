# Release Process

## Overview

Releases are triggered by pushing a version tag (e.g., `v0.2.0`). The CI pipeline builds all platform bundles and publishes a GitHub Release.

## Release Steps

### 1. Bump Version

Update version in two files:

- `src-tauri/Cargo.toml`: `version = "0.2.0"`
- `src-tauri/tauri.conf.json`: `"version": "0.2.0"`

### 2. Commit and Tag

```bash
git add -A
git commit -m "chore(release): bump sl-studio to v0.2.0"
git tag -a v0.2.0 -m "Release v0.2.0"
git push origin HEAD
git push origin v0.2.0
```

### 3. CI Pipeline Runs

The CI workflow triggers on the `v*` tag:

1. **Frontend** builds and uploads artifact
2. **Per-platform pipelines** run in parallel:
   - Rust checks + tests
   - Tauri app builds (DMG, DEB, AppImage, MSI, EXE)
3. **Release job** collects all artifacts and creates GitHub Release

### 4. Verify Release

After CI completes:

1. Go to GitHub > TheFirstIstari/sl-studio > Releases
2. Open the new release (e.g., v0.2.0)
3. Verify all platform artifacts are attached:
   - `tauri-aarch64-apple-darwin/*.dmg`, `*.app`
   - `tauri-x86_64-apple-darwin/*.dmg`, `*.app`
   - `tauri-x86_64-unknown-linux-gnu/*.deb`, `*.AppImage`
   - `tauri-x86_64-pc-windows-msvc/*.msi`, `*.exe`

## Release Artifacts

| Platform    | Target                     | Bundles       |
| ----------- | -------------------------- | ------------- |
| macOS ARM   | `aarch64-apple-darwin`     | DMG, APP      |
| macOS Intel | `x86_64-apple-darwin`      | DMG, APP      |
| Linux       | `x86_64-unknown-linux-gnu` | DEB, AppImage |
| Windows     | `x86_64-pc-windows-msvc`   | MSI, EXE      |

## Skipped-Safe Release

The release job uses `always() && !cancelled()` to ensure it runs even when some platform builds are skipped (e.g., self-hosted runners offline). This prevents releases from being blocked by unavailable runners.

## Secrets Required

| Secret               | Purpose                                |
| -------------------- | -------------------------------------- |
| `GITHUB_TOKEN`       | Automatic (provided by GitHub Actions) |
| `TAURI_PRIVATE_KEY`  | Tauri app signing                      |
| `TAURI_KEY_PASSWORD` | Private key password                   |
