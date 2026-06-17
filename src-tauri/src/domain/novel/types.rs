use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookSource {
    pub url: String,
    pub name: String,
    #[serde(default)]
    pub comment: String,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub search: Option<SearchRule>,
    #[serde(default)]
    pub book: Option<BookRule>,
    #[serde(default)]
    pub toc: Option<TocRule>,
    #[serde(default)]
    pub chapter: Option<ChapterRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRule {
    #[serde(default)]
    pub disabled: bool,
    pub url: String,
    pub method: String,
    #[serde(default)]
    pub data: String,
    #[serde(default)]
    pub cookies: String,
    pub result: String,
    #[serde(default, alias = "bookName")]
    pub book_name: String,
    #[serde(default = "default_empty", alias = "author")]
    pub author: String,
    #[serde(default, alias = "category")]
    pub category: String,
    #[serde(default, alias = "wordCount")]
    pub word_count: String,
    #[serde(default, alias = "status")]
    pub status: String,
    #[serde(default, alias = "latestChapter")]
    pub latest_chapter: String,
    #[serde(default, alias = "lastUpdateTime")]
    pub last_update_time: String,
    #[serde(default)]
    pub pagination: bool,
    #[serde(default, alias = "nextPage")]
    pub next_page: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookRule {
    #[serde(default)]
    pub url: String,
    #[serde(default, alias = "bookName")]
    pub book_name: String,
    #[serde(default, alias = "author")]
    pub author: String,
    #[serde(default)]
    pub intro: String,
    #[serde(default)]
    pub category: String,
    #[serde(default, alias = "coverUrl")]
    pub cover_url: String,
    #[serde(default, alias = "latestChapter")]
    pub latest_chapter: String,
    #[serde(default, alias = "lastUpdateTime")]
    pub last_update_time: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocRule {
    #[serde(default, alias = "baseUri")]
    pub base_uri: String,
    #[serde(default)]
    pub url: String,
    pub item: String,
    #[serde(default, alias = "isDesc")]
    pub is_desc: bool,
    #[serde(default)]
    pub pagination: bool,
    #[serde(default, alias = "nextPage")]
    pub next_page: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterRule {
    pub title: String,
    pub content: String,
    #[serde(default, alias = "paragraphTagClosed")]
    pub paragraph_tag_closed: bool,
    #[serde(default, alias = "paragraphTag")]
    pub paragraph_tag: String,
    #[serde(default, alias = "filterTxt")]
    pub filter_txt: String,
    #[serde(default, alias = "filterTag")]
    pub filter_tag: String,
    #[serde(default)]
    pub pagination: bool,
    #[serde(default, alias = "nextPage")]
    pub next_page: String,
}

fn default_empty() -> String {
    String::new()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchBookResult {
    pub book_name: String,
    pub author: String,
    pub url: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub word_count: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub latest_chapter: String,
    #[serde(default)]
    pub last_update_time: String,
    pub source_name: String,
    pub source_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookDetail {
    pub book_name: String,
    pub author: String,
    pub url: String,
    #[serde(default)]
    pub intro: String,
    #[serde(default)]
    pub cover_url: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub latest_chapter: String,
    #[serde(default)]
    pub status: String,
    pub source_name: String,
    pub source_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterInfo {
    pub title: String,
    pub url: String,
    pub index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterContent {
    pub title: String,
    pub content: String,
    pub index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub book_name: String,
    pub total_chapters: usize,
    pub downloaded: usize,
    pub current_chapter: String,
    pub status: String,
}
