use steinline_lib::gpu::{detect, GpuBackend, GpuInfo, HardwareStatus};

#[test]
fn test_hardware_detection() {
    let status = detect();

    assert!(status.cpu_threads > 0);
    assert!(status.total_memory_gb > 0.0);
}

#[test]
fn test_hardware_status_fields() {
    let status = HardwareStatus::default();

    assert!(status.cpu_threads > 0);
    assert!(status.total_memory_gb > 0.0);
}

#[test]
fn test_gpu_backend_enum() {
    assert_eq!(GpuBackend::from_str("metal"), GpuBackend::Metal);
    assert_eq!(GpuBackend::from_str("cuda"), GpuBackend::Cuda);
    assert_eq!(GpuBackend::from_str("cpu"), GpuBackend::Cpu);
}

#[test]
fn test_gpu_backend_as_str() {
    assert_eq!(GpuBackend::Metal.as_str(), "metal");
    assert_eq!(GpuBackend::Cuda.as_str(), "cuda");
    assert_eq!(GpuBackend::Cpu.as_str(), "cpu");
}

#[test]
fn test_gpu_backend_default() {
    assert_eq!(GpuBackend::default(), GpuBackend::Cpu);
}
