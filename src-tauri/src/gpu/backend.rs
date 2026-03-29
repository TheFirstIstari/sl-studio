use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GpuBackend {
    Metal,
    Cuda,
    Vulkan,
    OpenCl,
    Cpu,
}

impl GpuBackend {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "metal" => GpuBackend::Metal,
            "cuda" => GpuBackend::Cuda,
            "vulkan" => GpuBackend::Vulkan,
            "opencl" => GpuBackend::OpenCl,
            _ => GpuBackend::Cpu,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            GpuBackend::Metal => "metal",
            GpuBackend::Cuda => "cuda",
            GpuBackend::Vulkan => "vulkan",
            GpuBackend::OpenCl => "opencl",
            GpuBackend::Cpu => "cpu",
        }
    }
}

impl Default for GpuBackend {
    fn default() -> Self {
        GpuBackend::Cpu
    }
}
