//! ═══════════════════════════════════════════════════════════════════════════
//! 检测命令 - 检测系统 IPC 命令
//! ═══════════════════════════════════════════════════════════════════════════

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

// ── 辅助函数 ────────────────────────────────────────────────────────────────

fn validate_book_id(book_id: &str) -> Result<(), AppError> {
    validate_id(book_id, "book_id").map_err(AppError::invalid_input)
}

// ── 类型定义 ────────────────────────────────────────────────────────────────

/// 检测请求输入
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
    /// secrets 存储中的 account 名
    #[serde(default)]
    pub api_key_account: Option<String>,
}

// ── IPC 命令 ────────────────────────────────────────────────────────────────

/// 检测章节内容并记录历史
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

    // 服务端解析 API key
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

    // 解析 endpoint
    let api_url = input
        .api_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .or_else(|| default_api_url(&provider).map(String::from))
        .ok_or_else(|| AppError::missing_field("apiUrl (required for custom provider)"))?;

    // 调用检测 API
    let result = detect_ai_content(&provider, &api_url, &api_key, &input.content).await?;

    // 记录历史
    let data_dir_for_record = data_dir.inner().clone();
    let book_id_for_record = input.book_id.clone();
    let chapter_number = input.chapter_number;
    let score = result.score;
    let provider_str = result.provider.clone();
    let detected_at = result.detected_at.clone();
    tokio::task::spawn_blocking(move || {
        record_entry(
            &data_dir_for_record,
            &book_id_for_record,
            chapter_number,
            "detect",
            score,
            &provider_str,
            &detected_at,
        )
    })
    .await
    .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;

    tracing::info!(
        book_id = %input.book_id,
        chapter = input.chapter_number,
        provider = %result.provider,
        score = result.score,
        "Detection scan recorded"
    );
    Ok(IpcResponse::ok(result))
}

/// 获取检测聚合统计
#[tauri::command]
pub async fn detection_stats(
    data_dir: State<'_, DataDir>,
    book_id: String,
) -> Result<IpcResponse<DetectionStats>, AppError> {
    validate_book_id(&book_id)?;
    let data_dir = data_dir.inner().clone();
    let history = tokio::task::spawn_blocking(move || load_history(&data_dir, &book_id))
        .await
        .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;
    let stats = analyze_detection_insights(&history);
    Ok(IpcResponse::ok(stats))
}

/// 获取检测历史
#[tauri::command]
pub async fn detection_history(
    data_dir: State<'_, DataDir>,
    book_id: String,
) -> Result<IpcResponse<Vec<DetectionHistoryEntry>>, AppError> {
    validate_book_id(&book_id)?;
    let data_dir = data_dir.inner().clone();
    let history = tokio::task::spawn_blocking(move || load_history(&data_dir, &book_id))
        .await
        .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;
    Ok(IpcResponse::ok(history))
}