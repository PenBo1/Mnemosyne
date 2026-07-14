//! 目录解析。
//!
//! 输入目录页 HTML,按书源 `toc.item` 选择器圈定章节链接列表。
//! 首版不实现目录分页(主流书源目录都是单页全量)。

use scraper::Html;

use crate::domain::novel::parser::{abs_href, make_selector, text_of};
use crate::domain::novel::types::{BookSource, ChapterInfo};
use crate::shared::error::AppError;

/// 解析目录页,返回按源站顺序排列的章节列表。
///
/// `toc.is_desc=true` 时反转顺序。
pub fn parse(source: &BookSource, html: &str, base_url: &str) -> Result<Vec<ChapterInfo>, AppError> {
    let rule = source
        .toc
        .as_ref()
        .ok_or_else(|| AppError::invalid_input(format!("source `{}` has no toc rule", source.name)))?;

    let doc = Html::parse_document(html);
    let item_sel = make_selector(&rule.item)?;

    let mut chapters: Vec<ChapterInfo> = doc
        .select(&item_sel)
        .enumerate()
        .filter_map(|(i, el)| {
            let title = text_of(&el);
            let url = abs_href(&el, base_url)?;
            if url.is_empty() || title.is_empty() {
                return None;
            }
            Some(ChapterInfo {
                title,
                url,
                index: i + 1,
            })
        })
        .collect();

    if rule.is_desc {
        chapters.reverse();
        // 反转后重新分配序号,保持 1..N 连续
        for (i, ch) in chapters.iter_mut().enumerate() {
            ch.index = i + 1;
        }
    }

    Ok(chapters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::novel::types::{BookRule, ChapterRule, SearchRule};

    fn build_source(is_desc: bool) -> BookSource {
        BookSource {
            url: "http://example.com/".into(),
            name: "示例".into(),
            comment: String::new(),
            language: "zh".into(),
            enabled: true,
            search: Some(SearchRule {
                disabled: false,
                url: String::new(),
                method: "get".into(),
                data: String::new(),
                cookies: String::new(),
                result: String::new(),
                book_name: String::new(),
                author: String::new(),
                category: String::new(),
                word_count: String::new(),
                status: String::new(),
                latest_chapter: String::new(),
                last_update_time: String::new(),
                pagination: false,
                next_page: String::new(),
            }),
            book: Some(BookRule {
                url: String::new(),
                book_name: String::new(),
                author: String::new(),
                intro: String::new(),
                category: String::new(),
                cover_url: String::new(),
                latest_chapter: String::new(),
                last_update_time: String::new(),
                status: String::new(),
            }),
            toc: Some(crate::domain::novel::types::TocRule {
                base_uri: String::new(),
                url: String::new(),
                item: "dl > dd > a".into(),
                is_desc,
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
    fn parses_toc_in_order() {
        let html = r#"<html><body>
            <dl>
                <dd><a href="/c1">第一章</a></dd>
                <dd><a href="/c2">第二章</a></dd>
                <dd><a href="/c3">第三章</a></dd>
            </dl>
        </body></html>"#;
        let src = build_source(false);
        let chapters = parse(&src, html, "http://example.com/book/1").unwrap();
        assert_eq!(chapters.len(), 3);
        assert_eq!(chapters[0].title, "第一章");
        assert_eq!(chapters[0].url, "http://example.com/c1");
        assert_eq!(chapters[2].index, 3);
    }

    #[test]
    fn reverses_when_is_desc() {
        let html = r#"<html><body>
            <dl>
                <dd><a href="/c1">第一章</a></dd>
                <dd><a href="/c2">第二章</a></dd>
            </dl>
        </body></html>"#;
        let src = build_source(true);
        let chapters = parse(&src, html, "http://example.com/").unwrap();
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, "第二章");
        assert_eq!(chapters[0].index, 1);
        assert_eq!(chapters[1].title, "第一章");
    }

    #[test]
    fn skips_empty_links() {
        let html = r#"<html><body>
            <dl>
                <dd><a href="">空链接</a></dd>
                <dd><a href="/c1">第一章</a></dd>
            </dl>
        </body></html>"#;
        let src = build_source(false);
        let chapters = parse(&src, html, "http://example.com/").unwrap();
        assert_eq!(chapters.len(), 1);
    }
}
