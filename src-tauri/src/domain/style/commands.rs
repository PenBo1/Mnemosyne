// 风格分析器 IPC 命令(前端 camelCase 调用):
// - style_analyze: 提取参考文本的风格指纹画像

use crate::shared::error::{AppError, IpcResponse};

use super::analyzer::{analyze_style, Language};
use super::types::{StyleAnalyzeInput, StyleProfile};

#[tauri::command]
pub async fn style_analyze(input: StyleAnalyzeInput) -> Result<IpcResponse<StyleProfile>, AppError> {
    if input.text.trim().is_empty() {
        return Err(AppError::invalid_input("text cannot be empty"));
    }
    if input.text.len() > 100_000 {
        return Err(AppError::invalid_input("text too large (max 100000 chars)"));
    }
    let language = Language::parse(input.language.as_deref().unwrap_or("zh"));
    let profile = analyze_style(&input.text, input.source_name.as_deref(), language);
    Ok(IpcResponse::ok(profile))
}
