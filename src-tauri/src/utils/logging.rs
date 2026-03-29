use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_logging() -> Result<WorkerGuard, Box<dyn std::error::Error>> {
    let log_dir = get_log_dir();
    std::fs::create_dir_all(&log_dir)?;

    let file_appender = RollingFileAppender::new(Rotation::DAILY, &log_dir, "slstudio.log");

    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,slstudio=debug"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(true),
        )
        .with(
            fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(true)
                .with_target(true),
        )
        .init();

    tracing::info!("Logging initialized. Log directory: {:?}", log_dir);
    Ok(guard)
}

pub fn get_log_dir() -> std::path::PathBuf {
    dirs::data_dir()
        .unwrap_or_default()
        .join("slstudio")
        .join("logs")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_dir_created() {
        let dir = get_log_dir();
        assert!(dir.to_string_lossy().contains("slstudio"));
    }
}
