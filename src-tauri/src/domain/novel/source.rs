//! ═══════════════════════════════════════════════════════════════════════════
//! 书源管理 - 书源加载与查找
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 从 `<app_data>/novel_sources.json` 加载全部书源,提供按 name 查找的能力。
//! 文件字段兼容 camelCase。

use std::path::Path;

use crate::domain::novel::types::BookSource;
use crate::shared::error::AppError;

/// 从指定路径加载全部书源。文件不存在时返回空列表。
pub fn load_sources(path: &Path) -> Result<Vec<BookSource>, AppError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(path)
        .map_err(|_| AppError::file_read_error(path.display().to_string()))?;
    let sources: Vec<BookSource> = serde_json::from_str(&content)
        .map_err(|e| AppError::internal(format!("Failed to parse novel sources: {}", e)))?;
    Ok(sources)
}

/// 按 name 查找书源。
pub fn find_source<'a>(sources: &'a [BookSource], name: &str) -> Option<&'a BookSource> {
    sources.iter().find(|s| s.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_camel_case_fields() {
        // 用 r##"..."## 避免 "#content" 中的 "# 提前终止原始字符串
        let json = r##"[
            {
                "url": "http://example.com/",
                "name": "示例",
                "language": "zh",
                "search": {
                    "url": "http://example.com/search",
                    "method": "post",
                    "data": "{searchkey: %s}",
                    "result": ".result > li",
                    "bookName": "a.book",
                    "latestChapter": "span.latest"
                },
                "toc": { "item": "dd > a" },
                "chapter": {
                    "title": "h1",
                    "content": "#content",
                    "paragraphTagClosed": false
                }
            }
        ]"##;
        let tmp = std::env::temp_dir().join("mnemosyne_test_sources.json");
        std::fs::write(&tmp, json).unwrap();
        let sources = load_sources(&tmp).unwrap();
        assert_eq!(sources.len(), 1);
        let s = &sources[0];
        assert_eq!(s.name, "示例");
        assert_eq!(s.language, "zh");
        let search = s.search.as_ref().unwrap();
        assert_eq!(search.book_name, "a.book");
        assert_eq!(search.latest_chapter, "span.latest");
        let toc = s.toc.as_ref().unwrap();
        assert_eq!(toc.item, "dd > a");
        let chapter = s.chapter.as_ref().unwrap();
        assert_eq!(chapter.title, "h1");
        assert!(!chapter.paragraph_tag_closed);
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn returns_empty_when_file_missing() {
        let path = std::path::PathBuf::from("/tmp/__definitely_not_exists__.json");
        assert!(load_sources(&path).unwrap().is_empty());
    }

    #[test]
    fn find_source_by_name() {
        let sources = vec![
            BookSource {
                url: "http://a".into(),
                name: "A".into(),
                comment: String::new(),
                language: "zh".into(),
                enabled: true,
                search: None,
                book: None,
                toc: None,
                chapter: None,
            },
            BookSource {
                url: "http://b".into(),
                name: "B".into(),
                comment: String::new(),
                language: "zh".into(),
                enabled: true,
                search: None,
                book: None,
                toc: None,
                chapter: None,
            },
        ];
        assert!(find_source(&sources, "B").is_some());
        assert!(find_source(&sources, "C").is_none());
    }
}
