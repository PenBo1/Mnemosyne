
use serde::{Deserialize, Serialize};

use crate::shared::error::AppError;

/// 数据库错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbError {
    /// 错误信息
    pub message: String,
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Database error: {}", self.message)
    }
}

impl std::error::Error for DbError {}

/// JSON 解码辅助函数
///
/// 防御式编程：解码失败时提供详细的错误信息
pub(super) fn json_decode<T: serde::de::DeserializeOwned>(raw: &str, column: &str) -> Result<T, AppError> {
    serde_json::from_str(raw).map_err(|e| {
        AppError::internal(format!(
            "Failed to decode JSON column `{}`: {} (raw: {})",
            column,
            e,
            raw.chars().take(200).collect::<String>()
        ))
    })
}

/// JSON 编码辅助函数
pub(super) fn json_encode<T: serde::Serialize>(value: &T, column: &str) -> Result<String, AppError> {
    serde_json::to_string(value).map_err(|e| {
        AppError::internal(format!("Failed to encode JSON column `{}`: {}", column, e))
    })
}

/// 工作空间
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    /// 工作空间 ID
    pub id: String,
    /// 工作空间名称
    pub name: String,
    /// 工作空间路径
    pub path: String,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
    /// 最近打开时间（用于恢复上次活动工作区）
    pub last_opened_at: Option<String>,
}

/// 创建工作空间请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceRequest {
    /// 工作空间名称
    pub name: String,
    /// 工作空间路径（可选）
    pub path: Option<String>,
}

/// 更新工作空间请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorkspaceRequest {
    /// 工作空间 ID
    pub id: String,
    /// 工作空间名称
    pub name: Option<String>,
    /// 工作空间路径
    pub path: Option<String>,
}

/// Prompt 模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    /// Prompt ID
    pub id: String,
    /// Prompt 名称
    pub name: String,
    /// Prompt 内容
    pub content: String,
    /// 分类
    pub category: String,
    /// 标签列表
    pub tags: Vec<String>,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// 创建 Prompt 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePromptRequest {
    /// Prompt 名称
    pub name: String,
    /// Prompt 内容
    pub content: String,
    /// 分类
    pub category: String,
    /// 标签列表
    pub tags: Vec<String>,
}

/// 更新 Prompt 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePromptRequest {
    /// Prompt ID
    pub id: String,
    /// Prompt 名称
    pub name: Option<String>,
    /// Prompt 内容
    pub content: Option<String>,
    /// 分类
    pub category: Option<String>,
    /// 标签列表
    pub tags: Option<Vec<String>>,
}

/// 趋势数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trend {
    /// 趋势 ID
    pub id: String,
    /// 关键词
    pub keyword: String,
    /// 平台
    pub platform: String,
    /// 热度评分
    pub score: f64,
    /// 元数据
    pub metadata: serde_json::Value,
    /// 扫描时间
    pub scanned_at: String,
}

/// 小说
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Novel {
    /// 小说 ID
    pub id: String,
    /// 工作空间 ID
    pub workspace_id: String,
    /// 小说标题
    pub title: String,
    /// 类型/题材
    pub genre: String,
    /// 发布平台
    pub platform: String,
    /// 状态（draft/writing/published）
    pub status: String,
    /// 语言
    pub language: String,
    /// 总字数
    pub word_count: i64,
    /// 章节总数
    pub chapter_count: i64,
    /// 目标章节数
    pub target_chapters: i64,
    /// 每章目标字数
    pub chapter_words: i64,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// 章节
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    /// 章节 ID
    pub id: String,
    /// 小说 ID
    pub novel_id: String,
    /// 章节编号
    pub number: i64,
    /// 章节标题
    pub title: String,
    /// 状态（draft/reviewing/published）
    pub status: String,
    /// 字数
    pub word_count: i64,
    /// Audit 评分
    pub audit_score: Option<f64>,
    /// 修订次数
    pub revision_count: i64,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// Radar 扫描结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarScan {
    /// 扫描 ID
    pub id: String,
    /// 市场摘要
    pub market_summary: String,
    /// 推荐列表
    pub recommendations: Vec<RadarRecommendation>,
    /// 平台排行榜
    pub raw_rankings: Vec<PlatformRankings>,
    /// 创建时间
    pub created_at: String,
}

/// Radar 推荐
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RadarRecommendation {
    /// 推荐平台
    pub platform: String,
    /// 推荐题材
    pub genre: String,
    /// 推荐概念
    pub concept: String,
    /// 置信度评分
    pub confidence: f64,
    /// 推荐理由
    pub reasoning: String,
    /// 参考作品
    #[serde(default)]
    pub benchmark_titles: Vec<String>,
}

/// 平台排行榜
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformRankings {
    /// 平台名称
    pub platform: String,
    /// 排行榜条目
    pub entries: Vec<RankingEntry>,
}

/// 排行榜条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingEntry {
    /// 作品名称
    pub title: String,
    /// 作者
    pub author: String,
    /// 分类
    pub category: String,
    /// 额外信息
    pub extra: String,
}

/// Radar 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarResult {
    /// 推荐列表
    pub recommendations: Vec<RadarRecommendation>,
    /// 市场摘要
    pub market_summary: String,
}

/// 创建小说请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNovelRequest {
    /// 工作空间 ID
    pub workspace_id: String,
    /// 小说标题
    pub title: String,
    /// 类型/题材
    pub genre: String,
    /// 发布平台
    pub platform: String,
    /// 语言
    pub language: String,
    /// 目标章节数
    pub target_chapters: i64,
    /// 每章目标字数
    pub chapter_words: i64,
}

/// 更新小说请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNovelRequest {
    /// 小说标题
    pub title: Option<String>,
    /// 类型/题材
    pub genre: Option<String>,
    /// 发布平台
    pub platform: Option<String>,
    /// 语言
    pub language: Option<String>,
    /// 目标章节数
    pub target_chapters: Option<i64>,
    /// 每章目标字数
    pub chapter_words: Option<i64>,
}