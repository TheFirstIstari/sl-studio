# Hardware Detection

## Overview

Hardware detection commands are implemented in `commands/mod.rs` using the `sysinfo`
and `num_cpus` crates. There is no separate `gpu/` module — all detection and
auto-scaling logic lives directly in the command handlers.

## Commands

| Command                  | Returns            | Description                        |
| ------------------------ | ------------------ | ---------------------------------- |
| `detect_hardware`        | `HardwareStatus`   | Detect CPU, RAM, GPU info          |
| `get_hardware_info`      | `HardwareInfoExt`  | Detailed info for settings page    |
| `get_recommended_settings` | `HardwareInfo`   | Auto-scaled processing parameters  |
| `get_system_monitor`     | `SystemMonitor`    | Real-time CPU/memory snapshot      |

All types are defined in `lib.rs`.

### HardwareStatus

| Field               | Type   | Description                   |
| ------------------- | ------ | ----------------------------- |
| `cpu_cores`         | usize  | Total logical CPU cores       |
| `total_memory`      | usize  | Total system memory (bytes)   |
| `available_memory`  | usize  | Available memory (bytes)      |
| `gpu_backend`       | String | Detected GPU backend          |
| `gpu_name`          | String | GPU name (if available)       |
| `gpu_memory`        | usize  | GPU memory (bytes)            |

### HardwareInfoExt

| Field                  | Type  | Description                      |
| ---------------------- | ----- | -------------------------------- |
| `cpu_threads`          | usize | Logical CPU thread count         |
| `total_memory_gb`      | f64   | Total RAM in gigabytes           |
| `available_memory_gb`  | f64   | Available RAM in gigabytes       |
| `recommended_workers`  | usize | Auto-detected worker count       |
| `recommended_batch_size` | usize | Auto-detected batch size       |
| `cpu_workers`          | usize | Physical CPU core count          |

### HardwareInfo (Recommended Settings)

| Field                  | Type   | Description                    |
| ---------------------- | ------ | ------------------------------ |
| `recommended_context`  | usize  | Recommended LLM context length |
| `recommended_batch_size` | usize | Auto-scaled batch size         |
| `worker_count`         | usize  | Recommended CPU worker count   |
| `backend`              | String | Backend identifier             |

### SystemMonitor

| Field                | Type  | Description                    |
| -------------------- | ----- | ------------------------------ |
| `cpu_usage_percent`  | f64   | Current CPU usage percentage   |
| `memory_used_gb`     | f64   | Used memory in gigabytes       |
| `memory_available_gb` | f64   | Available memory in gigabytes  |
| `memory_percent`     | f64   | Memory usage percentage        |

## Implementation Notes

- CPU workers are calculated using `num_cpus::get_physical()` to leave headroom
  for the main thread and OS
- The default batch size is 6
- GPU detection currently falls back to "cpu" — Metal GPU auto-detection is
  handled by `rapid-mlx serve` at inference time
- Memory values are converted from bytes to gigabytes (f64) for the settings page
