//! ═══════════════════════════════════════════════════════════════════════════
//! Embedding 命令 - Tauri IPC 命令
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 配置读写复用 config.json（ai.embedding 段），向量操作走 DbState。
//! 本地与云端统一 OpenAI 兼容协议，前端参数用 camelCase。

use tauri::State;
use crate::shared::error::{AppError, IpcResponse};
use crate::infrastructure::db::state::DbState;
use super::types::{EmbeddingConfig, IndexDocParams, SearchParams, IngestFileParams, ALLOWED_DOC_TYPES};
use super::{config, service};

// ── 参数验证 ────────────────────────────────────────────────────────────────

/// 验证文档类型
fn validate_doc_type(doc_type: &str) -> Result<(), AppError> {
    if !ALLOWED_DOC_TYPES.contains(&doc_type) {
        return Err(AppError::invalid_input(format!(
            "Invalid doc_type: {} (allowed: {:?})", doc_type, ALLOWED_DOC_TYPES
        )));
    }
    Ok(())
}

// ── 配置命令 ────────────────────────────────────────────────────────────────

/// 读取 embedding 配置
#[tauri::command]
pub fn embedding_get_config(state: State<'_, DbState>) -> Result<IpcResponse<EmbeddingConfig>, AppError> {
    let cfg = config::read_embedding_config(&state.data_dir.config_path());
    tracing::info!(
        enabled = cfg.enabled,
        model = %cfg.model,
        "embedding_get_config: exit"
    );
    Ok(IpcResponse::ok(cfg))
}

/// 保存 embedding 配置
#[tauri::command]
pub fn embedding_set_config(
    state: State<'_, DbState>,
    config: EmbeddingConfig,
) -> Result<IpcResponse<()>, AppError> {
    config::write_embedding_config(&state.data_dir.config_path(), &config)?;
    tracing::info!(enabled = config.enabled, model = %config.model, "Embedding config saved");
    Ok(IpcResponse::ok(()))
}

// ── 测试命令 ────────────────────────────────────────────────────────────────

/// 测试 embedding 连接：返回实际维度
#[tauri::command]
pub async fn embedding_test(state: State<'_, DbState>) -> Result<IpcResponse<usize>, AppError> {
    let cfg = config::read_embedding_config(&state.data_dir.config_path());
    if !cfg.enabled {
        return Err(AppError::invalid_input("Embedding is disabled, enable it first"));
    }
    let dim = super::client::test_connection(&cfg).await?;
    tracing::info!(dim, "Embedding test passed");
    Ok(IpcResponse::ok(dim))
}

// ── 索引命令 ────────────────────────────────────────────────────────────────

/// 索引文档：切分 -> embed -> 存入向量表
#[tauri::command]
pub async fn embedding_index_doc(
    state: State<'_, DbState>,
    params: IndexDocParams,
) -> Result<IpcResponse<usize>, AppError> {
    let start = std::time::Instant::now();
    tracing::info!(
        doc_type = %params.doc_type,
        doc_id = %params.doc_id,
        workspace_id = ?params.workspace_id,
        "embedding_index_doc: enter"
    );

    // 参数验证
    validate_doc_type(&params.doc_type)?;
    if params.doc_id.trim().is_empty() {
        return Err(AppError::invalid_input("doc_id is empty"));
    }
    if params.content.trim().is_empty() {
        return Err(AppError::invalid_input("content is empty"));
    }

    // 委托给服务层
    let count = service::index_document(&state.db, &state.data_dir.config_path(), params).await?;

    tracing::info!(
        count,
        duration_ms = start.elapsed().as_millis(),
        "embedding_index_doc: exit"
    );
    Ok(IpcResponse::ok(count))
}

// ── 搜索命令 ────────────────────────────────────────────────────────────────

/// 语义搜索
#[tauri::command]
pub async fn embedding_search(
    state: State<'_, DbState>,
    params: SearchParams,
) -> Result<IpcResponse<Vec<super::types::SearchResult>>, AppError> {
    // 参数验证
    if let Some(ref dt) = params.doc_type {
        validate_doc_type(dt)?;
    }
    if params.query.trim().is_empty() {
        return Err(AppError::invalid_input("query is empty"));
    }

    // 委托给服务层
    let results = service::search(&state.db, &state.data_dir.config_path(), params).await?;

    Ok(IpcResponse::ok(results))
}

// ── 删除命令 ────────────────────────────────────────────────────────────────

/// 删除指定文档的向量索引
#[tauri::command]
pub fn embedding_delete_doc(
    state: State<'_, DbState>,
    doc_type: String,
    doc_id: String,
) -> Result<IpcResponse<usize>, AppError> {
    validate_doc_type(&doc_type)?;
    if doc_id.trim().is_empty() {
        return Err(AppError::invalid_input("doc_id is empty"));
    }
    let n = state.db.delete_vectors_by_doc(&doc_type, &doc_id)?;
    Ok(IpcResponse::ok(n))
}

// ── 统计命令 ────────────────────────────────────────────────────────────────

/// 向量索引统计
#[tauri::command]
pub fn embedding_stats(
    state: State<'_, DbState>,
) -> Result<IpcResponse<super::types::VectorStats>, AppError> {
    let stats = state.db.vector_stats()?;
    Ok(IpcResponse::ok(stats))
}

// ── 文件摄入命令 ────────────────────────────────────────────────────────────

/// 摄入文件(PDF/HTML/EPUB/TXT/MD)→ 提取文本 → 切分 → embed → 存入向量表
#[tauri::command]
pub async fn embedding_ingest_file(
    state: State<'_, DbState>,
    params: IngestFileParams,
) -> Result<IpcResponse<super::types::IngestResult>, AppError> {
    let start = std::time::Instant::now();
    tracing::info!(
        file_path = %params.file_path,
        doc_id = ?params.doc_id,
        workspace_id = ?params.workspace_id,
        "embedding_ingest_file: enter"
    );

    // 委托给服务层（服务层包含路径验证）
    let result = service::ingest_file(&state.db, &state.data_dir.config_path(), params).await?;

    tracing::info!(
        kind = %result.kind,
        doc_id = %result.doc_id,
        char_count = result.char_count,
        chunk_count = result.chunk_count,
        duration_ms = start.elapsed().as_millis(),
        "embedding_ingest_file: exit"
    );

    Ok(IpcResponse::ok(result))
}