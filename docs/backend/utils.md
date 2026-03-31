# Utilities

## Overview

The utils module provides shared functionality across the backend.

## File Utilities

`utils/files.rs` (~204 lines)

### FileType

```rust
enum FileType {
    Pdf,
    Image,
    Audio,
    Video,
    Docx,
    Text,
}
```

### FileMetadata

```rust
struct FileMetadata {
    file_type: FileType,
    size: u64,
    extension: String,
    // Additional metadata per type
}
```

### SHA-256 Fingerprinting

- **Small files**: Hash entire file
- **Large files**: Hash first + last 64KB for efficiency
- **Purpose**: Fast duplicate detection without full content comparison

### Directory Walking

Uses `walkdir` crate for recursive directory traversal with:

- Extension filtering
- Symlink handling
- Error recovery

## Path Helpers

`utils/paths.rs` (~30 lines)

Provides standard application directories:

| Function            | Path                              |
| ------------------- | --------------------------------- |
| `app_data_dir()`    | `~/.local/share/slstudio/`        |
| `models_dir()`      | `~/.local/share/slstudio/models/` |
| `dev_models_dir()`  | Development models directory      |
| `logs_dir()`        | `~/.local/share/slstudio/logs/`   |
| `ensure_app_dirs()` | Create all directories if missing |

## Structured Logging

`utils/logging.rs` (~53 lines)

Uses `tracing` + `tracing-subscriber` with:

- **Daily rotating file appender**: One log file per day
- **Stderr output**: For development debugging
- **Env filter**: Configurable log levels via `RUST_LOG`

### Log Configuration

```
RUST_LOG=info              # Info level
RUST_LOG=debug             # Debug level
RUST_LOG=slstudio=trace    # Trace for slstudio only
```
