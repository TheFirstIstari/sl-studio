use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareStatus {
    pub cpu_threads: u32,
    pub total_memory_gb: f64,
    pub available_memory_gb: f64,
    pub gpu_info: Vec<GpuInfo>,
    pub recommended_backend: String,
    pub scaling: ResourceScaling,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceScaling {
    pub batch_size: usize,
    pub cpu_workers: usize,
    pub ocr_batch_size: usize,
    pub max_chunk_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub vendor: String,
    pub vram_mb: u64,
    pub compute_units: u32,
    pub supported_backends: Vec<String>,
}

impl Default for HardwareStatus {
    fn default() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu_threads = sys.cpus().len() as u32;
        let total_memory_gb = sys.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
        let available_memory_gb = sys.available_memory() as f64 / (1024.0 * 1024.0 * 1024.0);

        let gpu_info = detect_gpu_internal();
        let recommended_backend = recommend_backend(&gpu_info);
        let scaling =
            calculate_scaling(cpu_threads, total_memory_gb, available_memory_gb, &gpu_info);

        HardwareStatus {
            cpu_threads,
            total_memory_gb,
            available_memory_gb,
            gpu_info,
            recommended_backend,
            scaling,
        }
    }
}

fn calculate_scaling(
    cpu_threads: u32,
    total_memory_gb: f64,
    available_memory_gb: f64,
    gpus: &[GpuInfo],
) -> ResourceScaling {
    let has_gpu = !gpus.is_empty();
    let has_apple_silicon = gpus.iter().any(|g| g.vendor == "Apple" && g.vram_mb > 0);

    // Calculate available memory ratio (use 60% of available memory for processing)
    let memory_ratio = (available_memory_gb / total_memory_gb).min(1.0);

    // Batch size scales with available memory and CPU
    let base_batch = if has_gpu || has_apple_silicon { 32 } else { 16 };
    let batch_size = (base_batch as f64 * (memory_ratio + 0.5) / 1.5).max(4.0) as usize;

    // CPU workers - use 75% of available threads
    let cpu_workers = ((cpu_threads as f64 * 0.75).max(1.0) as usize).min(cpu_threads as usize);

    // OCR batch - smaller due to memory intensity
    let ocr_batch_size = if has_gpu || has_apple_silicon { 16 } else { 8 };

    // Max chunk size based on available memory
    let max_chunk_size = if available_memory_gb > 16.0 {
        40000 // ~16K tokens
    } else if available_memory_gb > 8.0 {
        24000 // ~8K tokens
    } else {
        12000 // ~4K tokens
    };

    ResourceScaling {
        batch_size,
        cpu_workers,
        ocr_batch_size,
        max_chunk_size,
    }
}

fn detect_gpu_internal() -> Vec<GpuInfo> {
    let mut gpus = Vec::new();

    #[cfg(target_os = "macos")]
    {
        if let Some(name) = get_metal_name() {
            gpus.push(GpuInfo {
                name: name.clone(),
                vendor: "Apple".to_string(),
                vram_mb: get_apple_silicon_memory(),
                compute_units: 0,
                supported_backends: vec!["metal".to_string(), "cpu".to_string()],
            });
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(name) = get_cuda_name() {
            gpus.push(GpuInfo {
                name: name.clone(),
                vendor: "NVIDIA".to_string(),
                vram_mb: get_nvidia_vram(),
                compute_units: 0,
                supported_backends: vec![
                    "cuda".to_string(),
                    "vulkan".to_string(),
                    "opencl".to_string(),
                    "cpu".to_string(),
                ],
            });
        }
    }

    gpus
}

#[cfg(target_os = "macos")]
fn get_metal_name() -> Option<String> {
    let output = std::process::Command::new("sysctl")
        .args(["-n", "machdep.cpu.brand_string"])
        .output()
        .ok()?;

    if output.status.success() {
        let brand = String::from_utf8_lossy(&output.stdout);
        if brand.contains("Apple") {
            return Some(brand.trim().to_string());
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn get_apple_silicon_memory() -> u64 {
    let output = std::process::Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok();

    output
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok())
        .map(|v: u64| v / (1024 * 1024))
        .unwrap_or(0)
}

#[cfg(target_os = "windows")]
fn get_cuda_name() -> Option<String> {
    None
}

#[cfg(target_os = "windows")]
fn get_nvidia_vram() -> u64 {
    0
}

fn recommend_backend(gpus: &[GpuInfo]) -> String {
    for gpu in gpus {
        for backend in &gpu.supported_backends {
            match backend.as_str() {
                "metal" => return "metal".to_string(),
                "cuda" => return "cuda".to_string(),
                "vulkan" => return "vulkan".to_string(),
                "opencl" => return "opencl".to_string(),
                _ => continue,
            }
        }
    }
    "cpu".to_string()
}

pub fn detect() -> HardwareStatus {
    HardwareStatus::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_returns_valid() {
        let status = detect();
        assert!(status.cpu_threads > 0);
        assert!(status.total_memory_gb > 0.0);
    }
}
