//! ═══════════════════════════════════════════════════════════════════════════
//! Embedding 服务 - 业务逻辑层
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 提供文档索引、语义搜索、文件摄入等核心业务逻辑。
//! 从 commands.rs 中提取，遵循分层架构原则。

use std::path::Path;
use crate::shared::error::AppError;
use crate::infrastructure::db::connection::Database;
use super::types::{IndexDocParams, SearchParams, SearchResult};
use super::types::{IngestFileParams, IngestResult};
use super::{client, chunker, ingest, config};

// ── 文档索引服务 ────────────────────────────────────────────────────────────

/// 索引文档：切分 -> embed -> 存入向量表
///
/// # 参数
/// - `db`: 数据库实例
/// - `config_path`: config.json 路径
/// - `params`: 索引参数
///
/// # 返回值
/// 返回索引的 chunk 数量
pub async fn index_document(
    db: &Database,
    config_path: &Path,
    params: IndexDocParams,
) -> Result<usize, AppError> {
    let cfg = config::read_embedding_config(config_path);
    if !cfg.enabled {
        return Err(AppError::invalid_input("Embedding is disabled"));
    }

    // 切分文本
    let chunks = chunker::split_text(&params.content, None);
    if chunks.is_empty() {
        return Err(AppError::invalid_input("content is empty after chunking"));
    }

    tracing::info!(
        doc_type = %params.doc_type,
        doc_id = %params.doc_id,
        chunks = chunks.len(),
        "Indexing document"
    );

    // 批量嵌入
    let embeddings = client::embed_batch(&chunks, &cfg).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to embed chunks");
        e
    })?;

    // 组装 chunk-embedding 对
    let pairs: Vec<(String, Vec<f32>)> = chunks.into_iter()
        .zip(embeddings.into_iter())
        .collect();

    // 存入向量表
    let count = db.upsert_vectors(
        params.workspace_id.as_deref(),
        &params.doc_type,
        &params.doc_id,
        &pairs,
        &cfg.model,
        cfg.dim,
    ).map_err(|e| {
        tracing::error!(error = %e, "Failed to store vectors");
        e
    })?;

    Ok(count)
}

// ── 语义搜索服务 ────────────────────────────────────────────────────────────

/// 语义搜索：查询文本 -> embed -> 向量检索
///
/// # 参数
/// - `db`: 数据库实例
/// - `config_path`: config.json 路径
/// - `params`: 搜索参数
///
/// # 返回值
/// 返回相似度最高的搜索结果列表
pub async fn search(
    db: &Database,
    config_path: &Path,
    params: SearchParams,
) -> Result<Vec<SearchResult>, AppError> {
    let cfg = config::read_embedding_config(config_path);
    if !cfg.enabled {
        return Err(AppError::invalid_input("Embedding is disabled"));
    }

    // 嵌入查询文本
    let query_vec = client::embed(&params.query, &cfg).await?;

    // 向量检索
    let limit = params.limit.unwrap_or(10);
    let results = db.search_similar(
        params.workspace_id.as_deref(),
        &query_vec,
        &cfg.model,
        params.doc_type.as_deref(),
        limit,
    )?;

    Ok(results)
}

// ── 文件摄入服务 ────────────────────────────────────────────────────────────

/// 摄入文件：提取文本 -> 切分 -> embed -> 存入向量表
///
/// # 参数
/// - `db`: 数据库实例
/// - `config_path`: config.json 路径
/// - `params`: 摄入参数
///
/// # 返回值
/// 返回摄入结果（文档类型、字符数、chunk 数等）
pub async fn ingest_file(
    db: &Database,
    config_path: &Path,
    params: IngestFileParams,
) -> Result<IngestResult, AppError> {
    let path = Path::new(&params.file_path);

    // 路径验证
    if !path.is_absolute() {
        return Err(AppError::invalid_input("file_path must be absolute"));
    }
    if params.file_path.contains("..") {
        return Err(AppError::invalid_input("file_path contains '..'"));
    }
    if !path.exists() {
        return Err(AppError::not_found(format!("File not found: {}", params.file_path)));
    }

    let cfg = config::read_embedding_config(config_path);
    if !cfg.enabled {
        return Err(AppError::invalid_input("Embedding is disabled"));
    }

    // 提取文本
    let material = ingest::extract_text_from_file(path).map_err(|e| {
        tracing::error!(error = %e, "Failed to extract text");
        e
    })?;

    let char_count = material.text.chars().count();
    let excerpt = material.text.chars().take(200).collect();

    // 生成文档 ID
    let doc_id = params.doc_id.unwrap_or_else(|| generate_doc_id(path));

    // 切分文本
    let chunks = chunker::split_text(&material.text, None);
    if chunks.is_empty() {
        return Err(AppError::invalid_input("Material text is empty after chunking"));
    }

    let chunk_count = chunks.len();
    tracing::info!(
        kind = ?material.kind,
        doc_id = %doc_id,
        char_count,
        chunks = chunk_count,
        "Ingesting material"
    );

    // 批量嵌入
    let embeddings = client::embed_batch(&chunks, &cfg).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to embed chunks");
        e
    })?;

    // 组装 chunk-embedding 对
    let pairs: Vec<(String, Vec<f32>)> = chunks.into_iter()
        .zip(embeddings.into_iter())
        .collect();

    // 存入向量表
    db.upsert_vectors(
        params.workspace_id.as_deref(),
        "material",
        &doc_id,
        &pairs,
        &cfg.model,
        cfg.dim,
    ).map_err(|e| {
        tracing::error!(error = %e, "Failed to store vectors");
        e
    })?;

    Ok(IngestResult {
        kind: format!("{:?}", material.kind).to_lowercase(),
        title: material.title,
        char_count,
        chunk_count,
        doc_id,
        excerpt,
    })
}

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/// 生成文档 ID
///
/// 格式：{filename}-{timestamp}
fn generate_doc_id(path: &Path) -> String {
    let filename = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("material");
    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
    format!("{}-{}", filename, timestamp)
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_doc_id() {
        let path = Path::new("/some/path/test.pdf");
        let doc_id = generate_doc_id(path);
        assert!(doc_id.starts_with("test-"));
        assert!(doc_id.len() > 10); // timestamp adds more chars
    }
}