//! ═══════════════════════════════════════════════════════════════════════════
//! 设置管理 - 应用配置与日志
//! ═══════════════════════════════════════════════════════════════════════════

use tauri::Manager;
use tauri::State;
use crate::shared::error::{IpcResponse, AppError};
use crate::infrastructure::db::state::DbState;
use serde::Serialize;
use std::path::PathBuf;

const MAX_LOG_READ_SIZE: u64 = 2 * 1024 * 1024; // 单次读取上限 2MB

fn parse_theme(theme: &str) -> Result<Option<tauri::Theme>, AppError> {
    match theme.to_lowercase().as_str() {
        "dark" => Ok(Some(tauri::Theme::Dark)),
        "light" => Ok(Some(tauri::Theme::Light)),
        "system" | "auto" => Ok(None),
        _ => Err(AppError::invalid_input(format!("Unknown theme: {}", theme))),
    }
}

#[tauri::command]
pub fn set_window_theme(app: tauri::AppHandle, theme: String) -> Result<IpcResponse<()>, AppError> {
    let tauri_theme = parse_theme(&theme)?;

    // 更新主窗口主题
    if let Some(window) = app.get_webview_window("main") {
        // 忽略窗口已关闭或句柄无效的错误
        if let Err(e) = window.set_theme(tauri_theme) {
            tracing::debug!("Failed to set theme for main window: {}", e);
        }
    }

    // 更新日志查看窗口主题（忽略错误，窗口可能未打开或已关闭）
    if let Some(window) = app.get_webview_window("log-viewer") {
        let _ = window.set_theme(tauri_theme);
    }

    // 更新进程监控窗口主题（忽略错误，窗口可能未打开或已关闭）
    if let Some(window) = app.get_webview_window("process-monitor") {
        let _ = window.set_theme(tauri_theme);
    }

    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub fn get_data_dir_path(state: State<'_, DbState>) -> Result<IpcResponse<String>, AppError> {
    // 返回应用数据目录根路径，供前端展示与打开
    let path = state.data_dir.root().display().to_string();
    Ok(IpcResponse::ok(path))
}

#[tauri::command]
pub fn get_log_level(state: State<'_, DbState>) -> Result<IpcResponse<String>, AppError> {
    let config_path = state.data_dir.config_path();

    if let Ok(data) = std::fs::read_to_string(&config_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
            if let Some(level) = json.get("system").and_then(|s| s.get("log_level")).and_then(|l| l.as_str()) {
                return Ok(IpcResponse::ok(level.to_string()));
            }
        }
    }
    Ok(IpcResponse::ok("info".to_string()))
}

#[tauri::command]
pub fn set_log_level(state: State<'_, DbState>, level: String) -> Result<IpcResponse<()>, AppError> {
    let valid_levels = ["trace", "debug", "info", "warn", "error"];
    if !valid_levels.contains(&level.as_str()) {
        return Err(AppError::invalid_input(format!("Invalid log level: {}", level)));
    }

    let config_path = state.data_dir.config_path();

    // 区分"文件不存在（首次运行，用 {} 兜底）"与"文件存在但解析失败（损坏，报错）"。
    // 原实现用 unwrap_or(json!({})) 静默吞掉解析错误，导致配置损坏后用户无感知。
    let mut json: serde_json::Value = match std::fs::read_to_string(&config_path) {
        Ok(data) => serde_json::from_str(&data).map_err(|e| {
            AppError::invalid_format(format!(
                "Config file is corrupt at {}: {} \
                 (fix or delete the file manually)",
                config_path.display(),
                e
            ))
        })?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => serde_json::json!({}),
        Err(e) => return Err(AppError::internal(format!("Failed to read config: {}", e))),
    };

    if json.get("system").is_none() {
        json["system"] = serde_json::json!({});
    }
    json["system"]["log_level"] = serde_json::json!(level);

    let pretty = serde_json::to_string_pretty(&json)
        .map_err(|e| AppError::internal(format!("Failed to serialize config: {}", e)))?;
    std::fs::write(&config_path, pretty)
        .map_err(|e| AppError::internal(format!("Failed to write config: {}", e)))?;

    tracing::info!(level = %level, "Log level updated (restart required to take effect)");
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub fn get_git_enabled(state: State<'_, DbState>) -> Result<IpcResponse<bool>, AppError> {
    let config_path = state.data_dir.config_path();
    if let Ok(data) = std::fs::read_to_string(&config_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
            if let Some(enabled) = json
                .get("system")
                .and_then(|s| s.get("git_enabled"))
                .and_then(|v| v.as_bool())
            {
                return Ok(IpcResponse::ok(enabled));
            }
        }
    }
    // 默认启用 Git 功能
    Ok(IpcResponse::ok(true))
}

#[tauri::command]
pub fn set_git_enabled(state: State<'_, DbState>, enabled: bool) -> Result<IpcResponse<()>, AppError> {
    let config_path = state.data_dir.config_path();
    // 同 set_log_level：解析失败时显式报错而非静默回退
    let mut json: serde_json::Value = match std::fs::read_to_string(&config_path) {
        Ok(data) => serde_json::from_str(&data).map_err(|e| {
            AppError::invalid_format(format!(
                "Config file is corrupt at {}: {}",
                config_path.display(),
                e
            ))
        })?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => serde_json::json!({}),
        Err(e) => return Err(AppError::internal(format!("Failed to read config: {}", e))),
    };
    if json.get("system").is_none() {
        json["system"] = serde_json::json!({});
    }
    json["system"]["git_enabled"] = serde_json::json!(enabled);
    let pretty = serde_json::to_string_pretty(&json)
        .map_err(|e| AppError::internal(format!("Failed to serialize config: {}", e)))?;
    std::fs::write(&config_path, pretty)
        .map_err(|e| AppError::internal(format!("Failed to write config: {}", e)))?;
    tracing::info!(enabled, "Git feature toggle updated");
    Ok(IpcResponse::ok(()))
}

// ── 日志查看 ─────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct LogFileInfo {
    pub name: String,
    pub size: u64,
    pub modified: Option<String>,
}

/// 校验日志文件名：仅允许 `mnemosyne*.log` 形式，禁止路径分隔与目录穿越
fn validate_log_file_name(name: &str) -> Result<String, AppError> {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
        || name.contains('\0')
    {
        return Err(AppError::path_traversal());
    }
    // 匹配 mnemosyne.YYYY-MM-DD.log 或 mnemosyne.log 格式
    if !name.starts_with("mnemosyne") || !name.ends_with(".log") {
        return Err(AppError::invalid_input("Invalid log file name"));
    }
    Ok(name.to_string())
}

#[tauri::command]
pub fn list_log_files(state: State<'_, DbState>) -> Result<IpcResponse<Vec<LogFileInfo>>, AppError> {
    let logs_dir = state.data_dir.logs_dir();
    if !logs_dir.exists() {
        return Ok(IpcResponse::ok(Vec::new()));
    }

    let mut files: Vec<LogFileInfo> = Vec::new();
    for entry in std::fs::read_dir(&logs_dir)
        .map_err(|e| AppError::internal(format!("Failed to read logs dir: {}", e)))?
    {
        let entry = entry.map_err(|e| AppError::internal(format!("Failed to read entry: {}", e)))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        // 匹配 mnemosyne.YYYY-MM-DD.log 格式
        if !name.starts_with("mnemosyne") || !name.ends_with(".log") {
            continue;
        }
        let metadata = entry.metadata()
            .map_err(|e| AppError::internal(format!("Failed to read metadata: {}", e)))?;
        let modified = metadata.modified().ok().map(|t| {
            let dt: chrono::DateTime<chrono::Local> = t.into();
            dt.format("%Y-%m-%d %H:%M:%S").to_string()
        });
        files.push(LogFileInfo {
            name,
            size: metadata.len(),
            modified,
        });
    }

    // 按文件名降序（日期越新越靠前）
    files.sort_by(|a, b| b.name.cmp(&a.name));

    Ok(IpcResponse::ok(files))
}

#[tauri::command]
pub fn read_log_file(state: State<'_, DbState>, name: String) -> Result<IpcResponse<String>, AppError> {
    let validated = validate_log_file_name(&name)?;
    let logs_dir = state.data_dir.logs_dir();
    let file_path: PathBuf = logs_dir.join(&validated);

    if !file_path.exists() || !file_path.is_file() {
        return Err(AppError::not_found(format!("Log file not found: {}", validated)));
    }

    let metadata = std::fs::metadata(&file_path)
        .map_err(|e| AppError::internal(format!("Failed to read metadata: {}", e)))?;
    let size = metadata.len();

    let content = if size > MAX_LOG_READ_SIZE {
        // 文件过大时只读取尾部 2MB，并丢弃首行残缺内容
        use std::io::{Read, Seek, SeekFrom};
        let mut file = std::fs::File::open(&file_path)
            .map_err(|e| AppError::internal(format!("Failed to open log file: {}", e)))?;
        let start = size - MAX_LOG_READ_SIZE;
        file.seek(SeekFrom::Start(start))
            .map_err(|e| AppError::internal(format!("Failed to seek log file: {}", e)))?;
        let mut buffer = Vec::with_capacity(MAX_LOG_READ_SIZE as usize);
        file.read_to_end(&mut buffer)
            .map_err(|e| AppError::internal(format!("Failed to read log file: {}", e)))?;
        let text = String::from_utf8_lossy(&buffer).into_owned();
        // 跳过首行（可能被截断）
        if let Some(idx) = text.find('\n') {
            text[idx + 1..].to_string()
        } else {
            text
        }
    } else {
        std::fs::read_to_string(&file_path)
            .map_err(|e| AppError::internal(format!("Failed to read log file: {}", e)))?
    };

    Ok(IpcResponse::ok(content))
}

#[tauri::command]
pub fn clear_log_file(state: State<'_, DbState>, name: String) -> Result<IpcResponse<()>, AppError> {
    let validated = validate_log_file_name(&name)?;
    let logs_dir = state.data_dir.logs_dir();
    let file_path: PathBuf = logs_dir.join(&validated);

    if !file_path.exists() || !file_path.is_file() {
        return Err(AppError::not_found(format!("Log file not found: {}", validated)));
    }

    std::fs::write(&file_path, "")
        .map_err(|e| AppError::internal(format!("Failed to clear log file: {}", e)))?;

    tracing::info!(file = %validated, "Log file cleared by user");
    Ok(IpcResponse::ok(()))
}