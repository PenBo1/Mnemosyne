// Embedding IPC 命令
//
// 配置读写复用 config.json(ai.embedding 段),向量操作走 DbState。
// 本地与云端统一 OpenAI 兼容协议,前端参数用 camelCase。

use tauri::State;

use crate::shared::error::{AppError, IpcResponse};
use crate::infrastructure::db::state::DbState;
use super::types::{EmbeddingConfig, IndexDocParams, SearchParams};
use super::{client, chunker, ingest};

/// 允许的文档类型白名单
const ALLOWED_DOC_TYPES: &[&str] = &["chapter", "wiki", "message", "material"];

fn validate_doc_type(doc_type: &str) -> Result<(), AppError> {
    if !ALLOWED_DOC_TYPES.contains(&doc_type) {
        return Err(AppError::invalid_input(format!(
            "Invalid doc_type: {} (allowed: {:?})", doc_type, ALLOWED_DOC_TYPES
        )));
    }
    Ok(())
}

/// 从 config.json 读取 embedding 配置(缺省时返回默认值)
fn read_embedding_config(config_path: &std::path::Path) -> EmbeddingConfig {
    if let Ok(data) = std::fs::read_to_string(config_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
            if let Some(ai) = json.get("ai") {
                if let Some(emb) = ai.get("embedding") {
                    if let Ok(cfg) = serde_json::from_value::<EmbeddingConfig>(emb.clone()) {
                        return cfg;
                    }
                }
            }
        }
    }
    EmbeddingConfig::default()
}

/// 写入 embedding 配置到 config.json(保留其他字段)
fn write_embedding_config(config_path: &std::path::Path, cfg: &EmbeddingConfig) -> Result<(), AppError> {
    let mut json: serde_json::Value = if let Ok(data) = std::fs::read_to_string(config_path) {
        serde_json::from_str(&data).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    if json.get("ai").is_none() {
        json["ai"] = serde_json::json!({});
    }
    json["ai"]["embedding"] = serde_json::to_value(cfg)
        .map_err(|e| AppError::internal(format!("Failed to serialize embedding config: {}", e)))?;
    let pretty = serde_json::to_string_pretty(&json)
        .map_err(|e| AppError::internal(format!("Failed to serialize config: {}", e)))?;
    std::fs::write(config_path, pretty)
        .map_err(|e| AppError::internal(format!("Failed to write config: {}", e)))?;
    Ok(())
}

/// 读取 embedding 配置
#[tauri::command]
pub fn embedding_get_config(state: State<'_, DbState>) -> Result<IpcResponse<EmbeddingConfig>, AppError> {
    let cfg = read_embedding_config(&state.data_dir.config_path());
    Ok(IpcResponse::ok(cfg))
}

/// 保存 embedding 配置
#[tauri::command]
pub fn embedding_set_config(
    state: State<'_, DbState>,
    config: EmbeddingConfig,
) -> Result<IpcResponse<()>, AppError> {
    write_embedding_config(&state.data_dir.config_path(), &config)?;
    tracing::info!(enabled = config.enabled, model = %config.model, "Embedding config saved");
    Ok(IpcResponse::ok(()))
}

/// 测试 embedding 连接:返回实际维度
#[tauri::command]
pub async fn embedding_test(
    state: State<'_, DbState>,
) -> Result<IpcResponse<usize>, AppError> {
    let cfg = read_embedding_config(&state.data_dir.config_path());
    if !cfg.enabled {
        return Err(AppError::invalid_input("Embedding is disabled, enable it first"));
    }
    let dim = client::test_connection(&cfg).await?;
    tracing::info!(dim, "Embedding test passed");
    Ok(IpcResponse::ok(dim))
}

/// 索引文档:切分 -> embed -> 存入向量表
#[tauri::command]
pub async fn embedding_index_doc(
    state: State<'_, DbState>,
    params: IndexDocParams,
) -> Result<IpcResponse<usize>, AppError> {
    validate_doc_type(&params.doc_type)?;
    if params.doc_id.trim().is_empty() {
        return Err(AppError::invalid_input("doc_id is empty"));
    }
    if params.content.trim().is_empty() {
        return Err(AppError::invalid_input("content is empty"));
    }

    let cfg = read_embedding_config(&state.data_dir.config_path());
    if !cfg.enabled {
        return Err(AppError::invalid_input("Embedding is disabled"));
    }

    // 切分
    let chunks = chunker::split_text(&params.content, None);
    if chunks.is_empty() {
        return Err(AppError::invalid_input("content is empty after chunking"));
    }
    tracing::info!(doc_type = %params.doc_type, doc_id = %params.doc_id, chunks = chunks.len(), "Indexing document");

    // 批量 embed
    let embeddings = client::embed_batch(&chunks, &cfg).await?;

    // 组装 (content, embedding) 对
    let pairs: Vec<(String, Vec<f32>)> = chunks.into_iter()
        .zip(embeddings.into_iter())
        .collect();

    // 存入 DB
    let count = state.db.upsert_vectors(
        params.workspace_id.as_deref(),
        &params.doc_type,
        &params.doc_id,
        &pairs,
        &cfg.model,
        cfg.dim,
    )?;

    Ok(IpcResponse::ok(count))
}

/// 语义搜索
#[tauri::command]
pub async fn embedding_search(
    state: State<'_, DbState>,
    params: SearchParams,
) -> Result<IpcResponse<Vec<super::types::SearchResult>>, AppError> {
    if let Some(ref dt) = params.doc_type {
        validate_doc_type(dt)?;
    }
    if params.query.trim().is_empty() {
        return Err(AppError::invalid_input("query is empty"));
    }

    let cfg = read_embedding_config(&state.data_dir.config_path());
    if !cfg.enabled {
        return Err(AppError::invalid_input("Embedding is disabled"));
    }

    let query_vec = client::embed(&params.query, &cfg).await?;
    let limit = params.limit.unwrap_or(10);
    let results = state.db.search_similar(
        params.workspace_id.as_deref(),
        &query_vec,
        &cfg.model,
        params.doc_type.as_deref(),
        limit,
    )?;

    Ok(IpcResponse::ok(results))
}

/// 删除指定文档的向量索引
#[tauri::command]
#[allow(non_snake_case)]
pub fn embedding_delete_doc(
    state: State<'_, DbState>,
    docType: String,
    docId: String,
) -> Result<IpcResponse<usize>, AppError> {
    validate_doc_type(&docType)?;
    if docId.trim().is_empty() {
        return Err(AppError::invalid_input("docId is empty"));
    }
    let n = state.db.delete_vectors_by_doc(&docType, &docId)?;
    Ok(IpcResponse::ok(n))
}

/// 向量索引统计
#[tauri::command]
pub fn embedding_stats(
    state: State<'_, DbState>,
) -> Result<IpcResponse<super::types::VectorStats>, AppError> {
    let stats = state.db.vector_stats()?;
    Ok(IpcResponse::ok(stats))
}

/// 摄入文件(PDF/HTML/EPUB/TXT/MD)→ 提取文本 → 切分 → embed → 存入向量表
///
/// `file_path` 必须是已通过前端沙箱校验的本地文件路径。
/// `doc_id` 不传时,基于文件名+时间戳生成。
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestFileParams {
    /// 本地文件绝对路径
    pub file_path: String,
    /// 文档 id(可选,不传则自动生成)
    pub doc_id: Option<String>,
    /// 工作空间 id(可选)
    pub workspace_id: Option<String>,
}

/// 摄入结果
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestResult {
    /// 文档类型(pdf/html/epub/text)
    pub kind: String,
    /// 提取的标题(可能为空)
    pub title: Option<String>,
    /// 提取的纯文本字符数
    pub char_count: usize,
    /// 切分后的 chunk 数(=索引条数)
    pub chunk_count: usize,
    /// 实际存入 DB 的 doc_id
    pub doc_id: String,
    /// 文本前 200 字预览
    pub excerpt: String,
}

#[tauri::command]
pub async fn embedding_ingest_file(
    state: State<'_, DbState>,
    params: IngestFileParams,
) -> Result<IpcResponse<IngestResult>, AppError> {
    // 基本路径校验(防止空路径/相对路径)
    let path = std::path::Path::new(&params.file_path);
    if !path.is_absolute() {
        return Err(AppError::invalid_input("file_path must be absolute"));
    }
    // 防穿越:不允许包含 ..
    if params.file_path.contains("..") {
        return Err(AppError::invalid_input("file_path contains '..'"));
    }
    if !path.exists() {
        return Err(AppError::not_found(format!("File not found: {}", params.file_path)));
    }

    let cfg = read_embedding_config(&state.data_dir.config_path());
    if !cfg.enabled {
        return Err(AppError::invalid_input("Embedding is disabled"));
    }

    // 1. 提取文本
    let material = ingest::extract_text_from_file(path)?;
    let char_count = material.text.chars().count();
    let excerpt = material.text.chars().take(200).collect();

    // 2. 生成 doc_id(如未提供)
    let doc_id = params.doc_id.unwrap_or_else(|| {
        let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("material");
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
        format!("{}-{}", filename, timestamp)
    });

    // 3. 切分
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

    // 4. 批量 embed
    let embeddings = client::embed_batch(&chunks, &cfg).await?;

    // 5. 组装 (content, embedding) 对
    let pairs: Vec<(String, Vec<f32>)> = chunks.into_iter()
        .zip(embeddings.into_iter())
        .collect();

    // 6. 存入 DB(doc_type = "material")
    state.db.upsert_vectors(
        params.workspace_id.as_deref(),
        "material",
        &doc_id,
        &pairs,
        &cfg.model,
        cfg.dim,
    )?;

    Ok(IpcResponse::ok(IngestResult {
        kind: format!("{:?}", material.kind).to_lowercase(),
        title: material.title,
        char_count,
        chunk_count,
        doc_id,
        excerpt,
    }))
}
