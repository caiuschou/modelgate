//! Tracing subscriber setup: stderr (journal under systemd) and optional rolling log files.
use std::path::Path;

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::config::LoggingConfig;

/// Initialize global tracing. Safe to call from tests: second `try_init` is ignored.
pub fn init_tracing(cfg: &LoggingConfig) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let stderr = fmt::layer().with_writer(std::io::stderr).with_target(true);

    let dir = cfg.tracing_log_dir.trim();
    if dir.is_empty() {
        let _ = tracing_subscriber::registry()
            .with(filter)
            .with(stderr)
            .try_init();
        return;
    }

    let path = Path::new(dir);
    if let Err(e) = std::fs::create_dir_all(path) {
        eprintln!("modelgate: failed to create tracing_log_dir {path:?}: {e}");
        let _ = tracing_subscriber::registry()
            .with(filter)
            .with(stderr)
            .try_init();
        return;
    }

    let file_appender = tracing_appender::rolling::RollingFileAppender::new(
        tracing_appender::rolling::Rotation::DAILY,
        path,
        "modelgate.log",
    );
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    std::mem::forget(guard);

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true);

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(stderr)
        .with(file_layer)
        .try_init();
}
