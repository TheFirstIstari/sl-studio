use crate::config;
use crate::gpu::{self, HardwareStatus};
use crate::AppState;
use serde::Serialize;
use tauri::State;

#[tauri::command]
pub fn detect_hardware() -> HardwareStatus {
    gpu::detect()
}

#[tauri::command]
pub fn get_hardware_info(state: State<AppState>) -> config::HardwareInfo {
    state
        .config
        .read()
        .map(|g| g.get_hardware_info())
        .unwrap_or_default()
}

#[tauri::command]
pub fn get_recommended_settings() -> config::HardwareInfo {
    config::HardwareInfo::default()
}

#[derive(Serialize, Clone)]
pub struct SystemMonitor {
    pub cpu_usage_percent: f32,
    pub memory_used_gb: f64,
    pub memory_available_gb: f64,
    pub memory_percent: f32,
    pub gpu_usage_percent: Option<f32>,
    pub gpu_memory_used_mb: Option<u64>,
}

#[tauri::command]
pub fn get_system_monitor() -> SystemMonitor {
    use sysinfo::System;

    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_usage = sys.global_cpu_usage();
    let memory_used = sys.used_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
    let memory_total = sys.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
    let memory_available = sys.available_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
    let memory_percent = if memory_total > 0.0 {
        (memory_used / memory_total * 100.0) as f32
    } else {
        0.0
    };

    let gpu_status = gpu::detect();
    let gpu_usage = None;
    let gpu_memory = gpu_status.gpu_info.first().map(|g| g.vram_mb);

    SystemMonitor {
        cpu_usage_percent: cpu_usage,
        memory_used_gb: memory_used,
        memory_available_gb: memory_available,
        memory_percent,
        gpu_usage_percent: gpu_usage,
        gpu_memory_used_mb: gpu_memory,
    }
}

#[derive(Serialize, Clone)]
pub struct ProcessingStats {
    pub files_processed: i64,
    pub files_pending: i64,
    pub total_files: i64,
    pub processing_rate: f64,
}

#[tauri::command]
pub fn get_processing_stats(state: State<AppState>) -> Result<ProcessingStats, String> {
    let db_opt = {
        let guard = state
            .db
            .read()
            .map_err(|e| format!("Failed to read database: {}", e))?;
        guard.as_ref().cloned()
    };
    if let Some(db) = db_opt {
        let stats = db.get_overall_statistics().map_err(|e| e.to_string())?;

        Ok(ProcessingStats {
            files_processed: stats.total_facts,
            files_pending: 0,
            total_files: stats.total_facts,
            processing_rate: 0.0,
        })
    } else {
        Err("Database not initialized".to_string())
    }
}
