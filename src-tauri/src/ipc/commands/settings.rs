use tauri::Manager;
use tauri::State;
use crate::shared::errors::{IpcResponse, AppError};
use crate::core::state::AppState;

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
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::not_found("Main window not found"))?;

    let tauri_theme = parse_theme(&theme)?;
    window
        .set_theme(tauri_theme)
        .map_err(|e| AppError::internal(e.to_string()))?;

    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub fn get_log_level(state: State<'_, AppState>) -> Result<IpcResponse<String>, AppError> {
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
pub fn set_log_level(state: State<'_, AppState>, level: String) -> Result<IpcResponse<()>, AppError> {
    let valid_levels = ["trace", "debug", "info", "warn", "error"];
    if !valid_levels.contains(&level.as_str()) {
        return Err(AppError::invalid_input(format!("Invalid log level: {}", level)));
    }

    let config_path = state.data_dir.config_path();

    let mut json: serde_json::Value = if let Ok(data) = std::fs::read_to_string(&config_path) {
        serde_json::from_str(&data).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
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
