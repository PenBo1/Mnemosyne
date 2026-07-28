//! ═══════════════════════════════════════════════════════════════════════════
//! Novel Parser Book - 书籍详情解析
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 输入详情页 HTML,按书源 `book.*` 选择器逐字段提取。`cover_url` 若以
//! `meta[...]` 形式书写则取 `content` 属性,否则取元素文本。

use scraper::Html;

use crate::domain::novel::parser::{make_selector, select_first_text};
use crate::domain::novel::types::{BookDetail, BookSource};
use crate::shared::error::AppError;

/// 解析书籍详情页。
pub fn parse(source: &BookSource, url: &str, html: &str) -> Result<BookDetail, AppError> {
    let rule = source
        .book
        .as_ref()
        .ok_or_else(|| AppError::invalid_input(format!("source `{}` has no book rule", source.name)))?;

    let doc = Html::parse_document(html);

    let book_name = select_first_text(&doc, &rule.book_name)?;
    let author = select_first_text(&doc, &rule.author)?;

    // 校验：书名和作者必须非空
    if book_name.is_empty() && author.is_empty() {
        return Err(AppError::internal(format!(
            "book name and author are both empty on detail page {}",
            url
        )));
    }

    let intro = if rule.intro.is_empty() {
        String::new()
    } else {
        select_first_text(&doc, &rule.intro)?
    };
    let category = if rule.category.is_empty() {
        String::new()
    } else {
        select_first_text(&doc, &rule.category)?
    };
    let latest_chapter = if rule.latest_chapter.is_empty() {
        String::new()
    } else {
        select_first_text(&doc, &rule.latest_chapter)?
    };
    let status = if rule.status.is_empty() {
        String::new()
    } else {
        select_first_text(&doc, &rule.status)?
    };
    let cover_url = if rule.cover_url.is_empty() {
        String::new()
    } else {
        extract_meta_or_text(&doc, &rule.cover_url)?
    };

    Ok(BookDetail {
        book_name,
        author,
        url: url.to_string(),
        intro,
        cover_url,
        category,
        latest_chapter,
        status,
        source_name: source.name.clone(),
        source_url: source.url.clone(),
    })
}

/// 支持 `meta[property="og:image"]` 这种取属性的特殊选择器(取 `content` 属性);
/// 其余当作普通文本选择器。
fn extract_meta_or_text(doc: &Html, rule: &str) -> Result<String, AppError> {
    if rule.trim_start().starts_with("meta[") {
        let sel = make_selector(rule)?;
        Ok(doc
            .select(&sel)
            .next()
            .and_then(|el| el.value().attr("content").map(|s| s.to_string()))
            .unwrap_or_default())
    } else {
        select_first_text(doc, rule)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::novel::types::{ChapterRule, TocRule};

    fn build_source_with_book() -> BookSource {
        BookSource {
            url: "http://example.com/".into(),
            name: "示例".into(),
            comment: String::new(),
            language: "zh".into(),
            enabled: true,
            search: None,
            book: Some(crate::domain::novel::types::BookRule {
                url: String::new(),
                book_name: "h1.title".into(),
                author: "span.author".into(),
                intro: "#intro".into(),
                category: "span.cat".into(),
                cover_url: r#"meta[property="og:image"]"#.into(),
                latest_chapter: "span.latest".into(),
                last_update_time: "span.update".into(),
                status: "span.status".into(),
            }),
            toc: Some(TocRule {
                base_uri: String::new(),
                url: String::new(),
                item: "dd > a".into(),
                is_desc: false,
                pagination: false,
                next_page: String::new(),
            }),
            chapter: Some(ChapterRule {
                title: "h1".into(),
                content: "#content".into(),
                paragraph_tag_closed: false,
                paragraph_tag: String::new(),
                filter_txt: String::new(),
                filter_tag: String::new(),
                pagination: false,
                next_page: String::new(),
            }),
        }
    }

    #[test]
    fn parses_book_detail() {
        let html = r#"<html><head>
            <meta property="og:image" content="http://example.com/covers/1.jpg">
        </head><body>
            <h1 class="title">书名甲</h1>
            <span class="author">作者甲</span>
            <div id="intro">简介内容</div>
            <span class="cat">玄幻</span>
            <span class="latest">最新章节</span>
            <span class="update">2024-01-01</span>
            <span class="status">连载</span>
        </body></html>"#;
        let src = build_source_with_book();
        let detail = parse(&src, "http://example.com/book/1", html).unwrap();
        assert_eq!(detail.book_name, "书名甲");
        assert_eq!(detail.author, "作者甲");
        assert_eq!(detail.intro, "简介内容");
        assert_eq!(detail.category, "玄幻");
        assert_eq!(detail.cover_url, "http://example.com/covers/1.jpg");
        assert_eq!(detail.status, "连载");
    }

    #[test]
    fn errors_when_book_and_author_both_empty() {
        let html = "<html><body></body></html>";
        let src = build_source_with_book();
        let result = parse(&src, "http://x/", html);
        assert!(result.is_err());
    }
}
