
use serde::{Deserialize, Serialize};

/// 书源配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookSource {
    /// 书源 URL
    pub url: String,
    /// 书源名称
    pub name: String,
    /// 备注说明
    #[serde(default)]
    pub comment: String,
    /// 是否禁用
    #[serde(default)]
    pub disabled: bool,
    /// 搜索规则
    #[serde(default)]
    pub search: Option<SearchRule>,
    /// 书籍详情规则
    #[serde(default)]
    pub book: Option<BookRule>,
    /// 目录规则
    #[serde(default)]
    pub toc: Option<TocRule>,
    /// 章节内容规则
    #[serde(default)]
    pub chapter: Option<ChapterRule>,
}

/// 搜索规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRule {
    /// 是否禁用搜索
    #[serde(default)]
    pub disabled: bool,
    /// 搜索 URL
    pub url: String,
    /// HTTP 方法
    pub method: String,
    /// POST 数据
    #[serde(default)]
    pub data: String,
    /// Cookies
    #[serde(default)]
    pub cookies: String,
    /// 结果列表 XPath/JSONPath
    pub result: String,
    /// 书名规则
    #[serde(default, alias = "bookName")]
    pub book_name: String,
    /// 作者规则
    #[serde(default = "default_empty", alias = "author")]
    pub author: String,
    /// 分类规则
    #[serde(default, alias = "category")]
    pub category: String,
    /// 字数规则
    #[serde(default, alias = "wordCount")]
    pub word_count: String,
    /// 状态规则
    #[serde(default, alias = "status")]
    pub status: String,
    /// 最新章节规则
    #[serde(default, alias = "latestChapter")]
    pub latest_chapter: String,
    /// 最后更新时间规则
    #[serde(default, alias = "lastUpdateTime")]
    pub last_update_time: String,
    /// 是否支持分页
    #[serde(default)]
    pub pagination: bool,
    /// 下一页规则
    #[serde(default, alias = "nextPage")]
    pub next_page: String,
}

/// 书籍详情规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookRule {
    /// 详情页 URL 规则
    #[serde(default)]
    pub url: String,
    /// 书名规则
    #[serde(default, alias = "bookName")]
    pub book_name: String,
    /// 作者规则
    #[serde(default, alias = "author")]
    pub author: String,
    /// 简介规则
    #[serde(default)]
    pub intro: String,
    /// 分类规则
    #[serde(default)]
    pub category: String,
    /// 封面 URL 规则
    #[serde(default, alias = "coverUrl")]
    pub cover_url: String,
    /// 最新章节规则
    #[serde(default, alias = "latestChapter")]
    pub latest_chapter: String,
    /// 最后更新时间规则
    #[serde(default, alias = "lastUpdateTime")]
    pub last_update_time: String,
    /// 状态规则
    #[serde(default)]
    pub status: String,
}

/// 目录规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocRule {
    /// 基础 URI（用于拼接相对路径）
    #[serde(default, alias = "baseUri")]
    pub base_uri: String,
    /// 目录页 URL 规则
    #[serde(default)]
    pub url: String,
    /// 章节列表规则
    pub item: String,
    /// 是否倒序排列
    #[serde(default, alias = "isDesc")]
    pub is_desc: bool,
    /// 是否支持分页
    #[serde(default)]
    pub pagination: bool,
    /// 下一页规则
    #[serde(default, alias = "nextPage")]
    pub next_page: String,
}

/// 章节内容规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterRule {
    /// 标题规则
    pub title: String,
    /// 内容规则
    pub content: String,
    /// 段落标签是否闭合
    #[serde(default, alias = "paragraphTagClosed")]
    pub paragraph_tag_closed: bool,
    /// 段落标签名称
    #[serde(default, alias = "paragraphTag")]
    pub paragraph_tag: String,
    /// 文本过滤规则
    #[serde(default, alias = "filterTxt")]
    pub filter_txt: String,
    /// 标签过滤规则
    #[serde(default, alias = "filterTag")]
    pub filter_tag: String,
    /// 是否支持分页
    #[serde(default)]
    pub pagination: bool,
    /// 下一页规则
    #[serde(default, alias = "nextPage")]
    pub next_page: String,
}

fn default_empty() -> String {
    String::new()
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchBookResult {
    /// 书名
    pub book_name: String,
    /// 作者
    pub author: String,
    /// 书籍 URL
    pub url: String,
    /// 分类
    #[serde(default)]
    pub category: String,
    /// 字数
    #[serde(default)]
    pub word_count: String,
    /// 状态
    #[serde(default)]
    pub status: String,
    /// 最新章节
    #[serde(default)]
    pub latest_chapter: String,
    /// 最后更新时间
    #[serde(default)]
    pub last_update_time: String,
    /// 书源名称
    pub source_name: String,
    /// 书源 URL
    pub source_url: String,
}

/// 书籍详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookDetail {
    /// 书名
    pub book_name: String,
    /// 作者
    pub author: String,
    /// 书籍 URL
    pub url: String,
    /// 简介
    #[serde(default)]
    pub intro: String,
    /// 封面 URL
    #[serde(default)]
    pub cover_url: String,
    /// 分类
    #[serde(default)]
    pub category: String,
    /// 最新章节
    #[serde(default)]
    pub latest_chapter: String,
    /// 状态
    #[serde(default)]
    pub status: String,
    /// 书源名称
    pub source_name: String,
    /// 书源 URL
    pub source_url: String,
}

/// 章节信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterInfo {
    /// 章节标题
    pub title: String,
    /// 章节 URL
    pub url: String,
    /// 章节序号
    pub index: usize,
}

/// 章节内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterContent {
    /// 章节标题
    pub title: String,
    /// 章节内容
    pub content: String,
    /// 章节序号
    pub index: usize,
}

/// 下载进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    /// 书名
    pub book_name: String,
    /// 总章数
    pub total_chapters: usize,
    /// 已下载章数
    pub downloaded: usize,
    /// 当前章节名称
    pub current_chapter: String,
    /// 下载状态
    pub status: String,
}