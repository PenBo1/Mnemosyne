//! ═══════════════════════════════════════════════════════════════════════════
//! 风格命令 - IPC 命令处理
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! - style_analyze: 提取参考文本的风格指纹画像

use crate::shared::error::{AppError, IpcResponse};
use std::time::Instant;

use super::analyzer::{analyze_style, Language};
use super::types::{StyleAnalyzeInput, StyleProfile};

#[tauri::command]
pub async fn style_analyze(input: StyleAnalyzeInput) -> Result<IpcResponse<StyleProfile>, AppError> {
    let start = Instant::now();
    tracing::info!(
        text_len = input.text.len(),
        language = ?input.language,
        "style_analyze: enter"
    );
    
    if input.text.trim().is_empty() {
        tracing::error!("style_analyze: text cannot be empty");
        return Err(AppError::invalid_input("text cannot be empty"));
    }
    if input.text.len() > 100_000 {
        tracing::error!(len = input.text.len(), "style_analyze: text too large");
        return Err(AppError::invalid_input("text too large (max 100000 chars)"));
    }
    
    let language = Language::parse(input.language.as_deref().unwrap_or("zh"));
    let profile = analyze_style(&input.text, input.source_name.as_deref(), language);
    
    tracing::info!(
        duration_ms = start.elapsed().as_millis(),
        "style_analyze: exit"
    );
    Ok(IpcResponse::ok(profile))
}