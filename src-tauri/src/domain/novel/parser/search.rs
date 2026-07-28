//! ═══════════════════════════════════════════════════════════════════════════
//! Novel Parser Search - 搜索结果解析
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 输入搜索页 HTML,按书源的 `search.result` 选择器圈定结果列表,
//! 再用 `search.bookName`/`author`/`latestChapter` 等子选择器逐条提取字段。

use scraper::Html;

use crate::domain::novel::parser::{abs_href, make_selector, text_of};
use crate::domain::novel::types::{BookSource, SearchBookResult};
use crate::shared::error::AppError;

/// 解析搜索结果页,返回匹配关键字的书目列表。
///
/// 首页与下一页结果合并的策略暂未实现(分页搜索首版省略,见模块文档)。
pub fn parse(source: &BookSource, html: &str, base_url: &str) -> Result<Vec<SearchBookResult>, AppError> {
    let rule = source
        .search
        .as_ref()
        .ok_or_else(|| AppError::invalid_input(format!("source `{}` has no search rule", source.name)))?;

    let doc = Html::parse_document(html);
    let list_sel = make_selector(&rule.result)?;
    let book_name_sel = make_selector(&rule.book_name)?;
    let author_sel = make_selector(&rule.author)?;
    let category_sel = make_selector(&rule.category)?;
    let latest_chapter_sel = make_selector(&rule.latest_chapter)?;
    let last_update_sel = make_selector(&rule.last_update_time)?;
    let status_sel = make_selector(&rule.status)?;
    let word_count_sel = make_selector(&rule.word_count)?;

    let mut out = Vec::new();
    for el in doc.select(&list_sel) {
        let book_name = match el.select(&book_name_sel).next() {
            Some(n) => text_of(&n),
            None => continue, // 书名为空的记录跳过
        };
        let url = el
            .select(&book_name_sel)
            .next()
            .and_then(|n| abs_href(&n, base_url))
            .unwrap_or_default();
        let author = el.select(&author_sel).next().map(|n| text_of(&n)).unwrap_or_default();
        let category = el.select(&category_sel).next().map(|n| text_of(&n)).unwrap_or_default();
        let latest_chapter = el
            .select(&latest_chapter_sel)
            .next()
            .map(|n| text_of(&n))
            .unwrap_or_default();
        let last_update_time = el
            .select(&last_update_sel)
            .next()
            .map(|n| text_of(&n))
            .unwrap_or_default();
        let status = el.select(&status_sel).next().map(|n| text_of(&n)).unwrap_or_default();
        let word_count = el
            .select(&word_count_sel)
            .next()
            .map(|n| text_of(&n))
            .unwrap_or_default();

        out.push(SearchBookResult {
            book_name,
            author,
            url,
            category,
            word_count,
            status,
            latest_chapter,
            last_update_time,
            source_name: source.name.clone(),
            source_url: source.url.clone(),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::novel::types::{ChapterRule, SearchRule};

    fn build_source() -> BookSource {
        BookSource {
            url: "http://example.com/".into(),
            name: "示例源".into(),
            comment: String::new(),
            language: "zh".into(),
            enabled: true,
            search: Some(SearchRule {
                disabled: false,
                url: "http://example.com/search".into(),
                method: "get".into(),
                data: String::new(),
                cookies: String::new(),
                result: ".result > li".into(),
                book_name: "a.book".into(),
                author: "span.author".into(),
                category: "span.cat".into(),
                word_count: "span.words".into(),
                status: "span.status".into(),
                latest_chapter: "span.latest".into(),
                last_update_time: "span.update".into(),
                pagination: false,
                next_page: String::new(),
            }),
            book: None,
            toc: None,
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
    fn parses_search_results() {
        let html = r#"<html><body>
            <ul class="result">
                <li>
                    <a class="book" href="/book/1">书甲</a>
                    <span class="author">作者A</span>
                    <span class="cat">玄幻</span>
                    <span class="latest">最新章</span>
                    <span class="update">2024-01-01</span>
                </li>
                <li>
                    <a class="book" href="/book/2">书乙</a>
                    <span class="author">作者B</span>
                </li>
            </ul>
        </body></html>"#;
        let src = build_source();
        let results = parse(&src, html, "http://example.com/search").unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].book_name, "书甲");
        assert_eq!(results[0].author, "作者A");
        assert_eq!(results[0].url, "http://example.com/book/1");
        assert_eq!(results[1].latest_chapter, "");
    }

    #[test]
    fn skips_items_without_book_name() {
        let html = r#"<html><body>
            <ul class="result">
                <li><span class="author">无书名</span></li>
                <li><a class="book" href="/x">有书名</a></li>
            </ul>
        </body></html>"#;
        let src = build_source();
        let results = parse(&src, html, "http://example.com/").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].book_name, "有书名");
    }

    #[test]
    fn errors_when_no_search_rule() {
        let mut src = build_source();
        src.search = None;
        let result = parse(&src, "<html></html>", "http://x/");
        assert!(result.is_err());
    }
}
