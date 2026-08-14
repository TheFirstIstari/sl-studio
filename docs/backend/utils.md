# Utilities

## Overview

There is no separate `utils/` module in the current codebase. Utility functionality
is handled by external crates (`tracing`, `std::fs`, `num_cpus`) and inline logic
in `commands/mod.rs` and `lib.rs`.

## Logging

Uses `tracing` + `tracing-subscriber` with:

- **stderr output**: For development debugging
- **Env filter**: Configurable log levels via `RUST_LOG`

```
RUST_LOG=info              # Info level
RUST_LOG=debug             # Debug level
RUST_LOG=sl_studio=trace   # Trace for sl_studio only
```

## File Operations

File I/O is performed directly via `std::fs`:

- `std::fs::read_to_string` — Read text files in extractors
- `std::fs::read` — Read binary files in extractors
- `std::fs::write` — Save config and export data
- `std::fs::read_dir` — Directory walking in `start_registry`

## Path Handling

Config files and databases use paths relative to the current working directory:

| Function            | Path                              |
| ------------------- | --------------------------------- |
| Config file         | `sl-studio-config.json` (cwd)     |
| Database            | `sl-studio.db` (cwd)              |

## Hardware Detection

Uses `sysinfo` crate for system information and `num_cpus` for CPU detection:

- `num_cpus::get()` — Logical CPU count
- `num_cpus::get_physical()` — Physical CPU core count
- `sysinfo::System::new_all()` — Memory and CPU usage
