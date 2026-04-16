# Hardware Detection

## Overview

The GPU module handles hardware detection and auto-scaling parameters for optimal performance.

## Hardware Detection

`gpu/detect.rs` (~200 lines)

Detects system capabilities and calculates optimal processing parameters using the `sysinfo` crate:

```rust
fn detect_hardware() -> HardwareInfo {
    // Detect CPU, RAM, GPU
    // Calculate auto-scaling parameters
}
```

### Auto-Scaling Features

#### Hardware Auto-Detection (sysinfo)

Uses the `sysinfo` crate for cross-platform hardware detection:

- **CPU**: Core count, architecture, frequency
- **Memory**: Total RAM, available memory
- **System**: OS type, host name

#### Smart Worker Count

CPU workers are calculated as `num_cores - 2` to leave headroom for the main thread and OS:

| CPU Cores | Worker Threads |
| --------- | -------------- |
| 4         | 2              |
| 8         | 6              |
| 16        | 14             |
| 32        | 30             |

#### Memory-Aware Batching

Batch sizes scale dynamically based on available memory:

```rust
let available_memory = system.available_memory();
let batch_size = match available_memory {
    0..=8_GB => 4,
    8_GB..=16_GB => 8,
    16_GB..=32_GB => 16,
    _ => 24,
};
```

#### Thread Pool Reuse

Rayon thread pools are initialized once and reused across processing operations:

- Eliminates pool creation overhead
- Maintains warm thread affinity
- Configured per-worker memory allocation

### Detected Information

| Metric           | Source                        |
| ---------------- | ----------------------------- |
| CPU threads      | `num_cpus` crate              |
| Total memory     | System info                   |
| Available memory | System info                   |
| GPU info         | Metal (macOS), CUDA (Windows) |
| GPU memory       | GPU backend specific          |

### Auto-Scaling Parameters

Based on detected hardware:

| Parameter      | Calculation                |
| -------------- | -------------------------- |
| Batch size     | Scaled by available RAM    |
| CPU workers    | Based on CPU thread count  |
| OCR batch size | Scaled by available memory |
| Max chunk size | Limited by context window  |

## GPU Backend

`gpu/backend.rs` (~33 lines)

```rust
enum GpuBackend {
    Metal,
    Cuda,
    Vulkan,
    OpenCl,
    Cpu,
}
```

### Platform Support

| Platform         | Backend       |
| ---------------- | ------------- |
| macOS            | Metal         |
| Windows (NVIDIA) | CUDA          |
| Linux (AMD)      | Vulkan/OpenCL |
| Fallback         | CPU           |

## System Metrics

Real-time monitoring via `get_system_monitor`:

```rust
struct SystemMetrics {
    gpu_available: bool,
    gpu_utilization: f32,
    gpu_memory_used_mb: u64,
    gpu_memory_total_mb: u64,
    cpu_count: usize,
    cpu_usage: f32,
    ram_used_mb: u64,
    ram_total_mb: u64,
    disk_space_available_mb: u64,
}
```
