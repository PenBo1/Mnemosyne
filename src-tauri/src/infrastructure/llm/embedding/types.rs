// Embedding 配置与返回类型

use serde::{Deserialize, Serialize};

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

/// 文档类型:决定索引来源与检索过滤
pub const DOC_TYPE_CHAPTER: &str = "chapter";
pub const DOC_TYPE_WIKI: &str = "wiki";
pub const DOC_TYPE_MESSAGE: &str = "message";

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
