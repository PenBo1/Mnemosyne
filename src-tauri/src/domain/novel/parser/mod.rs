//! HTML 解析公共工具与各解析器入口。
//!
//! 基于 `scraper` crate 提供 CSS 选择器辅助，各解析器职责：
//! - `search` → 搜索结果解析
//! - `book` → 书籍详情解析
//! - `toc` → 目录解析
//! - `chapter` → 章节正文解析

pub mod book;
pub mod chapter;
pub mod search;
pub mod toc;

use scraper::{ElementRef, Html, Selector};

use crate::shared::error::AppError;

/// 编译 CSS 选择器,失败时返回 `AppError::internal`。
pub fn make_selector(rule: &str) -> Result<Selector, AppError> {
    Selector::parse(rule).map_err(|e| {
        AppError::internal(format!("invalid CSS selector `{}`: {}", rule, e))
    })
}

/// 提取元素文本(去前后空白)。
pub fn text_of(el: &ElementRef) -> String {
    el.text().collect::<Vec<_>>().join("").trim().to_string()
}

/// 提取元素 HTML 内部片段。
pub fn html_of(el: &ElementRef) -> String {
    el.html()
}

/// 在文档中按 CSS 选择器查找,返回首个匹配元素的文本(可空)。
pub fn select_first_text(doc: &Html, rule: &str) -> Result<String, AppError> {
    let s = make_selector(rule)?;
    Ok(doc
        .select(&s)
        .next()
        .map(|el| text_of(&el))
        .unwrap_or_default())
}

/// 在文档中按 CSS 选择器查找,返回首个匹配元素的 HTML(可空)。
pub fn select_first_html(doc: &Html, rule: &str) -> Result<String, AppError> {
    let s = make_selector(rule)?;
    Ok(doc
        .select(&s)
        .next()
        .map(|el| html_of(&el))
        .unwrap_or_default())
}

/// 提取元素 `href` 属性,自动相对→绝对路径转换。空 href 返回 None。
pub fn abs_href(el: &ElementRef, base: &str) -> Option<String> {
    let raw = el.value().attr("href")?;
    if raw.is_empty() {
        return None;
    }
    resolve_url(raw, base)
}

/// 将相对 URL 解析为绝对 URL(失败时返回原值)。
pub fn resolve_url(raw: &str, base: &str) -> Option<String> {
    let base_url = url::Url::parse(base).ok()?;
    base_url.join(raw).ok().map(|u| u.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_text_via_css() {
        let doc = Html::parse_document(r#"<html><body><div class="t">Hello</div></body></html>"#);
        let txt = select_first_text(&doc, ".t").unwrap();
        assert_eq!(txt, "Hello");
    }

    #[test]
    fn returns_empty_when_no_match() {
        let doc = Html::parse_document("<html><body></body></html>");
        let txt = select_first_text(&doc, ".missing").unwrap();
        assert!(txt.is_empty());
    }

    #[test]
    fn resolves_relative_url() {
        let abs = resolve_url("/book/123/", "http://example.com/info").unwrap();
        assert_eq!(abs, "http://example.com/book/123/");
    }
}
