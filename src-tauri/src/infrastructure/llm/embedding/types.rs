//! ═══════════════════════════════════════════════════════════════════════════
//! Embedding 类型 - 配置与返回类型
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

// ── 配置类型 ────────────────────────────────────────────────────────────────

/// Embedding 配置(独立于对话模型配置,存于 config.json 的 ai.embedding)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddingConfig {
    /// 是否启用向量检索
    pub enabled: bool,
    /// OpenAI 兼容端点(含 /v1),本地默认 Ollama
    pub base_url: String,
    /// API Key(本地服务可空)
    pub api_key: String,
    /// 模型名称
    pub model: String,
    /// 向量维度(须与模型实际维度一致)
    pub dim: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: "http://localhost:11434/v1".into(),
            api_key: String::new(),
            model: "nomic-embed-text".into(),
            dim: 768,
        }
    }
}

// ── 文档类型常量 ────────────────────────────────────────────────────────────

/// 允许的文档类型白名单
pub const ALLOWED_DOC_TYPES: &[&str] = &["chapter", "wiki", "message", "material"];

/// 文档类型常量
pub const DOC_TYPE_CHAPTER: &str = "chapter";
pub const DOC_TYPE_WIKI: &str = "wiki";
pub const DOC_TYPE_MESSAGE: &str = "message";
pub const DOC_TYPE_MATERIAL: &str = "material";

// ── 索引请求类型 ────────────────────────────────────────────────────────────

/// 索引请求参数
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexDocParams {
    pub doc_type: String,
    pub doc_id: String,
    pub content: String,
    pub workspace_id: Option<String>,
}

/// 搜索请求参数
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchParams {
    pub query: String,
    pub workspace_id: Option<String>,
    pub doc_type: Option<String>,
    pub limit: Option<i64>,
}

/// 文件摄入参数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestFileParams {
    /// 本地文件绝对路径
    pub file_path: String,
    /// 文档 id(可选,不传则自动生成)
    pub doc_id: Option<String>,
    /// 工作空间 id(可选)
    pub workspace_id: Option<String>,
}

// ── 响应类型 ────────────────────────────────────────────────────────────────

/// 语义搜索结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub doc_type: String,
    pub doc_id: String,
    pub chunk_idx: i64,
    pub content: String,
    pub score: f32,
}

/// 索引统计
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorStats {
    pub total: i64,
    pub by_doc_type: Vec<DocTypeCount>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocTypeCount {
    pub doc_type: String,
    pub count: i64,
}

/// 摄入结果
#[derive(Debug, Serialize)]
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
