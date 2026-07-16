//! 章节正文解析。
//!
//! 处理流程:
//! 1. 按 `chapter.content` CSS 选择器圈定正文元素,取其内部 HTML
//! 2. 应用 `chapter.filter_txt` 正则(管道分隔的多模式)去除广告文本
//! 3. 移除 `chapter.filter_tag` 匹配的元素(逗号分隔的 CSS 选择器)
//! 4. 按 `chapter.paragraph_tag_closed` 决定段落切分方式:
//!    - `true`: 提取所有 `<p>...</p>` 内的文本作为段落
//!    - `false`: 用 `chapter.paragraph_tag` 正则(如 `<br>+`)切分,每段用 `<p>` 包裹后提取
//! 5. 转换为带首行缩进的纯文本,前置章节标题

use regex::Regex;
use scraper::{Html, Selector};

use crate::domain::novel::parser::select_first_html;
use crate::domain::novel::types::BookSource;
use crate::shared::error::AppError;

/// 全角空格,用于段落首行缩进两字符。
const INDENT: &str = "\u{3000}\u{3000}";

/// 解析章节正文,返回带标题前缀的纯文本。
pub fn parse(source: &BookSource, html: &str, title: &str) -> Result<String, AppError> {
    let rule = source
        .chapter
        .as_ref()
        .ok_or_else(|| AppError::invalid_input(format!("source `{}` has no chapter rule", source.name)))?;

    let doc = Html::parse_document(html);
    let content_html = select_first_html(&doc, &rule.content)?;
    if content_html.is_empty() {
        return Err(AppError::internal("chapter content is empty"));
    }

    // 1) filter_txt:管道分隔的多模式正则,替换为空字符串
    let mut html_str = content_html;
    if !rule.filter_txt.is_empty() {
        let pattern = format!("({})", rule.filter_txt);
        if let Ok(re) = Regex::new(&pattern) {
            html_str = re.replace_all(&html_str, "").to_string();
        }
    }

    // 2) filter_tag:逗号分隔的 CSS 选择器,逐个移除匹配元素
    if !rule.filter_tag.is_empty() {
        for sel_str in rule.filter_tag.split(',') {
            let sel_str = sel_str.trim();
            if sel_str.is_empty() {
                continue;
            }
            if let Ok(sel) = Selector::parse(sel_str) {
                let frag = Html::parse_fragment(&html_str);
                let mut to_remove: Vec<String> = Vec::new();
                for el in frag.select(&sel) {
                    to_remove.push(el.html());
                }
                for r in to_remove {
                    html_str = html_str.replace(&r, "");
                }
            }
        }
    }

    // 3) 解码 HTML 实体 + 转为带段落缩进的纯文本
    let paragraphs = if rule.paragraph_tag_closed {
        extract_p_paragraphs(&html_str)
    } else {
        extract_br_paragraphs(&html_str, &rule.paragraph_tag)
    };

    let mut out = String::new();
    out.push_str(title);
    out.push_str("\n\n");
    for p in paragraphs {
        let p = p.trim();
        if p.is_empty() {
            continue;
        }
        out.push_str(INDENT);
        out.push_str(p);
        out.push('\n');
    }
    Ok(out)
}

/// 提取 `<p>段落</p>` 形式的段落(段落闭合模式)。
fn extract_p_paragraphs(html: &str) -> Vec<String> {
    let re = match Regex::new(r"(?s)<p[^>]*>(.*?)</p>") {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    re.captures_iter(html)
        .filter_map(|c| c.get(1).map(|m| decode_html_entities(m.as_str())))
        .collect()
}

/// 用 `paragraph_tag` 正则(如 `<br>+`)切分文本,每个非空片段作为一个段落(段落不闭合模式)。
fn extract_br_paragraphs(html: &str, paragraph_tag: &str) -> Vec<String> {
    let tag = if paragraph_tag.is_empty() { "<br>+" } else { paragraph_tag };
    let re = match Regex::new(tag) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    re.split(html)
        .map(strip_tags)
        .map(|s| decode_html_entities(&s))
        .collect()
}

/// 去除所有 HTML 标签,保留纯文本(用于 `<br>` 切分后的残片)。
fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// 解码常见 HTML 实体，保留语义。
fn decode_html_entities(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::novel::types::{BookRule, TocRule};

    fn build_source(closed: bool, filter_txt: &str, filter_tag: &str, paragraph_tag: &str) -> BookSource {
        BookSource {
            url: "http://example.com/".into(),
            name: "示例".into(),
            comment: String::new(),
            language: "zh".into(),
            enabled: true,
            search: None,
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
            toc: Some(TocRule {
                base_uri: String::new(),
                url: String::new(),
                item: "a".into(),
                is_desc: false,
                pagination: false,
                next_page: String::new(),
            }),
            chapter: Some(crate::domain::novel::types::ChapterRule {
                title: "h1".into(),
                content: "#content".into(),
                paragraph_tag_closed: closed,
                paragraph_tag: paragraph_tag.into(),
                filter_txt: filter_txt.into(),
                filter_tag: filter_tag.into(),
                pagination: false,
                next_page: String::new(),
            }),
        }
    }

    #[test]
    fn parses_br_separated_paragraphs() {
        let html = r#"<html><body><div id="content">第一段<br><br>第二段<br><br>广告文本</div></body></html>"#;
        let src = build_source(false, "", "", "<br>+");
        let txt = parse(&src, html, "第一章 测试").unwrap();
        assert!(txt.starts_with("第一章 测试\n\n"));
        assert!(txt.contains("第一段"));
        assert!(txt.contains("第二段"));
    }

    #[test]
    fn parses_p_separated_paragraphs() {
        let html = r#"<html><body><div id="content"><p>段落甲</p><p>段落乙</p></div></body></html>"#;
        let src = build_source(true, "", "", "");
        let txt = parse(&src, html, "第一章").unwrap();
        assert!(txt.contains("段落甲"));
        assert!(txt.contains("段落乙"));
    }

    #[test]
    fn applies_filter_txt_regex() {
        let html = r#"<html><body><div id="content">正文<br>请记住本站域名：abc.com</div></body></html>"#;
        let src = build_source(false, "请记住本站域名：.+?com", "", "<br>+");
        let txt = parse(&src, html, "第一章").unwrap();
        assert!(!txt.contains("abc.com"));
        assert!(txt.contains("正文"));
    }

    #[test]
    fn removes_filter_tag_elements() {
        let html = r#"<html><body><div id="content">正文<br><script>alert(1)</script><div class="ad">广告</div></div></body></html>"#;
        let src = build_source(false, "", "script, .ad", "<br>+");
        let txt = parse(&src, html, "第一章").unwrap();
        assert!(!txt.contains("alert"));
        assert!(!txt.contains("广告"));
    }

    #[test]
    fn decodes_common_entities() {
        let html = r#"<html><body><div id="content"><p>A&nbsp;B&amp;C</p></div></body></html>"#;
        let src = build_source(true, "", "", "");
        let txt = parse(&src, html, "T").unwrap();
        assert!(txt.contains("A B"));
        assert!(txt.contains("&"));
    }
}
