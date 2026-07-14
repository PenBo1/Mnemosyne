// 检测 IPC 命令(前端 camelCase 调用):
// - detection_scan:    检测章节内容,记录历史
// - detection_stats:   获取某 book 的检测聚合统计
// - detection_history: 获取某 book 的检测历史
//
// 安全: API key 不经 IPC 传入,由命令层从 secrets 存储服务端解析
// (service=mnemosyne-detection, account 默认为 provider 名,可用 apiKeyAccount 覆盖)。

use tauri::{AppHandle, State};

use crate::infrastructure::fs::data_dir::DataDir;
use crate::infrastructure::secrets::{get_secret, SecretsState};
use crate::infrastructure::validation::validate_id;
use crate::shared::error::{AppError, IpcResponse};

use super::detector::{default_api_url, detect_ai_content};
use super::insights::analyze_detection_insights;
use super::store::{load_history, record_entry};
use super::types::{DetectionHistoryEntry, DetectionResult, DetectionStats};
use super::DETECTION_SECRET_SERVICE;

fn validate_book_id(book_id: &str) -> Result<(), AppError> {
    validate_id(book_id, "book_id").map_err(AppError::invalid_input)
}

/// 检测请求(前端 camelCase)。apiUrl/apiKeyAccount 可选(有默认)。
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionScanInput {
    pub book_id: String,
    pub chapter_number: u32,
    pub content: String,
    /// gptzero / originality / custom
    pub provider: String,
    #[serde(default)]
    pub api_url: Option<String>,
    /// secrets 存储中的 account 名(默认等于 provider)
    #[serde(default)]
    pub api_key_account: Option<String>,
}

#[tauri::command]
pub async fn detection_scan(
    app: AppHandle,
    secrets_state: State<'_, SecretsState>,
    data_dir: State<'_, DataDir>,
    input: DetectionScanInput,
) -> Result<IpcResponse<DetectionResult>, AppError> {
    validate_book_id(&input.book_id)?;
    let provider = input.provider.trim().to_lowercase();
    if provider.is_empty() {
        return Err(AppError::missing_field("provider"));
    }
    if input.content.trim().is_empty() {
        return Err(AppError::invalid_input("content cannot be empty"));
    }

    // 1. 服务端解析 API key(不经 IPC 传入)
    let account = input
        .api_key_account
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&provider);
    let api_key = get_secret(&app, &secrets_state, DETECTION_SECRET_SERVICE, account)?
        .ok_or_else(|| {
            AppError::invalid_input(format!(
                "Detection API key not configured. Set it via secrets_set (service='{}', account='{}').",
                DETECTION_SECRET_SERVICE, account
            ))
        })?;

    // 2. 解析 endpoint(custom 必须显式传,其余有默认)
    let api_url = input
        .api_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .or_else(|| default_api_url(&provider).map(String::from))
        .ok_or_else(|| AppError::missing_field("apiUrl (required for custom provider)"))?;

    // 3. 调用检测 API
    let result = detect_ai_content(&provider, &api_url, &api_key, &input.content).await?;

    // 4. 记录历史
    record_entry(
        &data_dir,
        &input.book_id,
        input.chapter_number,
        "detect",
        result.score,
        &result.provider,
        &result.detected_at,
    )?;

    tracing::info!(
        book_id = %input.book_id,
        chapter = input.chapter_number,
        provider = %result.provider,
        score = result.score,
        "Detection scan recorded"
    );
    Ok(IpcResponse::ok(result))
}

#[tauri::command]
pub async fn detection_stats(
    data_dir: State<'_, DataDir>,
    book_id: String,
) -> Result<IpcResponse<DetectionStats>, AppError> {
    validate_book_id(&book_id)?;
    let history = load_history(&data_dir, &book_id)?;
    let stats = analyze_detection_insights(&history);
    Ok(IpcResponse::ok(stats))
}

#[tauri::command]
pub async fn detection_history(
    data_dir: State<'_, DataDir>,
    book_id: String,
) -> Result<IpcResponse<Vec<DetectionHistoryEntry>>, AppError> {
    validate_book_id(&book_id)?;
    let history = load_history(&data_dir, &book_id)?;
    Ok(IpcResponse::ok(history))
}
