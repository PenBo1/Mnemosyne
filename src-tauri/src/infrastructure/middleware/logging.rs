use std::path::Path;
use tracing_appender::rolling;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use crate::infrastructure::file_storage::data_dir::DataDir;

fn read_log_level(settings_path: &Path) -> String {
    if let Ok(data) = std::fs::read_to_string(settings_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
            if let Some(level) = json.get("system").and_then(|s| s.get("log_level")).and_then(|l| l.as_str()) {
                return level.to_string();
            }
        }
    }
    "info".to_string()
}

pub fn init(logs_dir: &Path, data_dir: &DataDir) {
    let file_appender = rolling::daily(logs_dir, "mnemosyne.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    std::mem::forget(_guard);

    let log_level = read_log_level(&data_dir.config_path());
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&log_level));

    let use_json = std::env::var("LOG_FORMAT").unwrap_or_default() == "json";

    if use_json {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .with_writer(non_blocking)
                    .with_ansi(false)
                    .json()
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_file(true)
                    .with_line_number(true),
            )
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .with_writer(std::io::stdout)
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_file(true)
                    .with_line_number(true)
                    .pretty(),
            )
            .with(
                fmt::layer()
                    .with_writer(non_blocking)
                    .with_ansi(false)
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_file(true)
                    .with_line_number(true),
            )
            .init();
    }

    tracing::info!(
        log_dir = %logs_dir.display(),
        log_level = %log_level,
        use_json,
        "Logging initialized"
    );
}
