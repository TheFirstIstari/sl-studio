use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum GpuBackend {
    #[default]
    Metal,
    Cuda,
    Vulkan,
    OpenCl,
    Cpu,
}

impl GpuBackend {
    pub fn parse(s: &str) -> Self {
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
