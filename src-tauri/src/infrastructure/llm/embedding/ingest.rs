// 文档摄入(Ingest)—— 将 PDF / HTML / EPUB / TXT / MD 提取为纯文本。
//
// Rust 版本用 pdf-extract (基于 lopdf) + zip + scraper,覆盖主流文档格式。
//
// 安全约束:
// - 文件大小上限 18MB(防止内存爆炸)
// - 仅读取本地文件,无网络请求
// - 调用方必须先做路径校验(沙箱/白名单),本模块不做路径校验

use std::path::Path;
use crate::shared::error::AppError;

/// 最大源文件字节数(18MB)
pub const MAX_SOURCE_BYTES: usize = 18 * 1024 * 1024;

/// 文档类型(决定解析方式)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialKind {
    Pdf,
    Html,
    Epub,
    Text,
}

impl MaterialKind {
    /// 根据文件扩展名推断类型
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "pdf" => Some(Self::Pdf),
            "html" | "htm" => Some(Self::Html),
            "epub" => Some(Self::Epub),
            "txt" | "md" | "markdown" | "json" | "csv" | "tsv" | "yaml" | "yml" | "log" | "xml" => Some(Self::Text),
            _ => None,
        }
    }
}

/// 提取结果
#[derive(Debug, Clone)]
pub struct ExtractedMaterial {
    /// 文档类型
    pub kind: MaterialKind,
    /// 提取的纯文本(已 normalize)
    pub text: String,
    /// 标题(可能从 HTML <title> 或 EPUB <dc:title> 提取)
    pub title: Option<String>,
    /// 总页数(仅 PDF)
    pub total_pages: Option<usize>,
}

/// 从文件提取文本
///
/// `path` 必须是已通过沙箱校验的绝对路径
pub fn extract_text_from_file(path: &Path) -> Result<ExtractedMaterial, AppError> {
    let meta = std::fs::metadata(path)
        .map_err(|e| AppError::invalid_input(format!("Cannot read file metadata: {}", e)))?;
    if meta.len() as usize > MAX_SOURCE_BYTES {
        tracing::warn!(path = %path.display(), size = meta.len(), max = MAX_SOURCE_BYTES, "File too large, rejecting");
        return Err(AppError::invalid_input(format!(
            "File too large ({} bytes, max {})", meta.len(), MAX_SOURCE_BYTES
        )));
    }

    let ext = path.extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let kind = match MaterialKind::from_extension(ext) {
        Some(k) => k,
        None => {
            tracing::warn!(path = %path.display(), ext = %ext, "Unsupported file type");
            return Err(AppError::invalid_input(format!("Unsupported file type: {}", ext)));
        }
    };

    tracing::info!(path = %path.display(), kind = ?kind, size = meta.len(), "Extracting text from file");
    let bytes = std::fs::read(path)
        .map_err(|e| AppError::invalid_input(format!("Failed to read file: {}", e)))?;

    extract_text_from_bytes(&bytes, kind)
}

/// 从字节数组提取文本(不依赖文件系统)
pub fn extract_text_from_bytes(bytes: &[u8], kind: MaterialKind) -> Result<ExtractedMaterial, AppError> {
    match kind {
        MaterialKind::Pdf => extract_pdf(bytes),
        MaterialKind::Html => extract_html(bytes),
        MaterialKind::Epub => extract_epub(bytes),
        MaterialKind::Text => extract_text(bytes),
    }
}

// --- PDF ---

fn extract_pdf(bytes: &[u8]) -> Result<ExtractedMaterial, AppError> {
    let text = pdf_extract::extract_text_from_mem(bytes)
        .map_err(|e| AppError::internal(format!("PDF extraction failed: {}", e)))?;
    let normalized = normalize_text(&text);
    if normalized.is_empty() {
        return Err(AppError::invalid_input(
            "PDF text extraction returned no text. Scanned PDFs require OCR and are not supported."
        ));
    }
    // pdf-extract 不直接给页数,这里不强行计算
    Ok(ExtractedMaterial {
        kind: MaterialKind::Pdf,
        text: normalized,
        title: None,
        total_pages: None,
    })
}

// --- HTML ---

fn extract_html(bytes: &[u8]) -> Result<ExtractedMaterial, AppError> {
    let raw = std::str::from_utf8(bytes)
        .map_err(|_| AppError::invalid_input("HTML file is not valid UTF-8"))?;
    let html = scraper::Html::parse_document(raw);
    let title = html_title(&html);
    let text = html_to_text(&html);
    let normalized = normalize_text(&text);
    if normalized.is_empty() {
        return Err(AppError::invalid_input("HTML extraction returned no text"));
    }
    Ok(ExtractedMaterial {
        kind: MaterialKind::Html,
        text: normalized,
        title,
        total_pages: None,
    })
}

fn html_title(html: &scraper::Html) -> Option<String> {
    use scraper::Selector;
    let sel = Selector::parse("title").ok()?;
    html.select(&sel).next()?.text().collect::<String>().trim().to_string().into()
}

fn html_to_text(html: &scraper::Html) -> String {
    use scraper::Selector;
    // 简单实现:遍历 body,遇到 script/style/noscript 跳过其子树,
    // 块级元素后补换行。
    let body_sel = Selector::parse("body").unwrap();
    let body = match html.select(&body_sel).next() {
        Some(b) => b,
        None => return html.root_element().text().collect::<String>(),
    };
    let mut buf = String::new();
    collect_text_skip(body, &mut buf);
    buf
}

fn collect_text_skip(node: scraper::ElementRef, buf: &mut String) {
    for child in node.children() {
        if child.value().is_element() {
            if let Some(elem_ref) = scraper::ElementRef::wrap(child) {
                let tag = elem_ref.value().name();
                // 跳过 script / style / noscript 子树
                if matches!(tag, "script" | "style" | "noscript") {
                    continue;
                }
                collect_text_skip(elem_ref, buf);
                if is_block_tag(tag) {
                    buf.push('\n');
                }
                continue;
            }
        }
        if let Some(text) = child.value().as_text() {
            buf.push_str(text);
        }
    }
}

fn is_block_tag(tag: &str) -> bool {
    matches!(tag, "p" | "div" | "br" | "section" | "article" | "header" | "footer" | "nav" | "aside" | "main" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "ul" | "ol" | "li" | "table" | "tr" | "pre" | "blockquote")
}

// --- EPUB ---

fn extract_epub(bytes: &[u8]) -> Result<ExtractedMaterial, AppError> {
    use std::io::Cursor;
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| AppError::invalid_input(format!("Invalid EPUB (not a zip): {}", e)))?;

    // 1. 读 container.xml 找 OPF 路径
    let container_xml = read_zip_file(&mut archive, "META-INF/container.xml")?;
    let opf_path = extract_opf_path(&container_xml)
        .ok_or_else(|| AppError::invalid_input("EPUB container.xml missing OPF path"))?;

    // 2. 读 OPF
    let opf = read_zip_file(&mut archive, &opf_path)?;
    let title = extract_dc_title(&opf);
    let manifest = parse_opf_manifest(&opf);
    let spine = parse_opf_spine(&opf);

    let base_dir = opf_path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");

    // 3. 按 spine 顺序提取每个章节
    let mut chapters_text = Vec::new();
    for idref in &spine {
        let href = match manifest.get(idref) {
            Some(h) => h,
            None => continue,
        };
        let chapter_path = normalize_zip_path(base_dir, href);
        let html = match read_zip_file(&mut archive, &chapter_path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let parsed = scraper::Html::parse_document(&html);
        let text = html_to_text(&parsed);
        let normalized = normalize_text(&text);
        if !normalized.is_empty() {
            chapters_text.push(normalized);
        }
    }

    if chapters_text.is_empty() {
        return Err(AppError::invalid_input("EPUB contains no readable spine chapters"));
    }

    let text = chapters_text.join("\n\n");
    Ok(ExtractedMaterial {
        kind: MaterialKind::Epub,
        text,
        title,
        total_pages: None,
    })
}

fn read_zip_file<R: std::io::Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    name: &str,
) -> Result<String, AppError> {
    let mut file = archive.by_name(name)
        .map_err(|e| AppError::invalid_input(format!("EPUB entry '{}' not found: {}", name, e)))?;
    let mut buf = String::new();
    std::io::Read::read_to_string(&mut file, &mut buf)
        .map_err(|e| AppError::internal(format!("Failed to read EPUB entry '{}': {}", name, e)))?;
    Ok(buf)
}

fn extract_opf_path(container_xml: &str) -> Option<String> {
    // 用 r##"..."## 避免 ["'] 内部 " 紧跟 # 时被误判为 raw string 结束符
    let re = regex::Regex::new(r##"rootfile[^>]+full-path=["']([^"']+)["']"##).unwrap();
    re.captures(container_xml).and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

fn extract_dc_title(opf: &str) -> Option<String> {
    let re = regex::Regex::new(r"(?i)<dc:title[^>]*>([\s\S]*?)</dc:title>").unwrap();
    let raw = re.captures(opf).and_then(|c| c.get(1).map(|m| m.as_str().to_string()))?;
    Some(decode_xml_entities(&raw).trim().to_string())
}

fn parse_opf_manifest(opf: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let item_re = regex::Regex::new(r"(?i)<item\b([^>]+)>").unwrap();
    let attr_re = regex::Regex::new(r##"(?i)(id|href)\s*=\s*["']([^"']+)["']"##).unwrap();

    for cap in item_re.captures_iter(opf) {
        let attrs = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let mut id = None;
        let mut href = None;
        for attr_cap in attr_re.captures_iter(attrs) {
            let key = attr_cap.get(1).map(|m| m.as_str().to_lowercase()).unwrap_or_default();
            let value = attr_cap.get(2).map(|m| m.as_str()).unwrap_or("");
            match key.as_str() {
                "id" => id = Some(value.to_string()),
                "href" => href = Some(decode_xml_entities(value)),
                _ => {}
            }
        }
        if let (Some(id), Some(href)) = (id, href) {
            map.insert(id, href);
        }
    }
    map
}

fn parse_opf_spine(opf: &str) -> Vec<String> {
    let spine_re = regex::Regex::new(r"(?is)<spine\b[\s\S]*?</spine>").unwrap();
    let spine = match spine_re.find(opf) {
        Some(m) => m.as_str(),
        None => return Vec::new(),
    };
    let itemref_re = regex::Regex::new(r##"(?i)<itemref\b[^>]*idref\s*=\s*["']([^"']+)["']"##).unwrap();
    itemref_re.captures_iter(spine)
        .filter_map(|c| c.get(1).map(|m| decode_xml_entities(m.as_str())))
        .collect()
}

fn normalize_zip_path(base: &str, href: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if !base.is_empty() {
        parts.extend(base.split('/'));
    }
    parts.extend(href.split('/'));
    let mut result: Vec<&str> = Vec::new();
    for p in parts {
        if p.is_empty() || p == "." {
            continue;
        }
        if p == ".." {
            result.pop();
            continue;
        }
        result.push(p);
    }
    result.join("/")
}

fn decode_xml_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

// --- TXT ---

fn extract_text(bytes: &[u8]) -> Result<ExtractedMaterial, AppError> {
    let raw = std::str::from_utf8(bytes)
        .map_err(|_| AppError::invalid_input("Text file is not valid UTF-8"))?;
    Ok(ExtractedMaterial {
        kind: MaterialKind::Text,
        text: normalize_text(raw),
        title: None,
        total_pages: None,
    })
}

// --- 公共工具 ---

fn normalize_text(s: &str) -> String {
    let decoded = decode_xml_entities(s);
    let unified = decoded.replace("\r\n", "\n").replace("\r", "\n");
    // 去掉行尾空白
    let no_trailing_ws: String = unified
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    // 多个连续空行压缩为单个(用 regex,因为 std::str::replace 是字面替换)
    let re = regex::Regex::new(r"\n{3,}").unwrap();
    let collapsed = re.replace_all(&no_trailing_ws, "\n\n");
    collapsed.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_kind_from_extension() {
        assert_eq!(MaterialKind::from_extension("pdf"), Some(MaterialKind::Pdf));
        assert_eq!(MaterialKind::from_extension("PDF"), Some(MaterialKind::Pdf));
        assert_eq!(MaterialKind::from_extension("html"), Some(MaterialKind::Html));
        assert_eq!(MaterialKind::from_extension("epub"), Some(MaterialKind::Epub));
        assert_eq!(MaterialKind::from_extension("md"), Some(MaterialKind::Text));
        assert_eq!(MaterialKind::from_extension("unknown"), None);
    }

    #[test]
    fn text_extraction_returns_utf8() {
        let bytes = b"hello\n\nworld";
        let result = extract_text_from_bytes(bytes, MaterialKind::Text).unwrap();
        assert_eq!(result.kind, MaterialKind::Text);
        assert!(result.text.contains("hello"));
        assert!(result.text.contains("world"));
    }

    #[test]
    fn text_extraction_rejects_invalid_utf8() {
        let bytes = &[0xff, 0xfe, 0xfd];
        let result = extract_text_from_bytes(bytes, MaterialKind::Text);
        assert!(result.is_err());
    }

    #[test]
    fn html_extraction_strips_tags() {
        let html = b"<html><head><title>Test</title></head><body><p>Hello <b>world</b></p></body></html>";
        let result = extract_text_from_bytes(html, MaterialKind::Html).unwrap();
        assert_eq!(result.title.as_deref(), Some("Test"));
        assert!(result.text.contains("Hello"));
        assert!(result.text.contains("world"));
        // script/style 不应出现
        assert!(!result.text.contains("script"));
    }

    #[test]
    fn decode_xml_entities_basic() {
        assert_eq!(decode_xml_entities("a &amp; b"), "a & b");
        assert_eq!(decode_xml_entities("&lt;tag&gt;"), "<tag>");
        assert_eq!(decode_xml_entities("&quot;hi&quot;"), "\"hi\"");
    }

    #[test]
    fn normalize_zip_path_handles_parent() {
        assert_eq!(normalize_zip_path("OEBPS", "../cover.xhtml"), "cover.xhtml");
        assert_eq!(normalize_zip_path("", "OEBPS/ch1.xhtml"), "OEBPS/ch1.xhtml");
        assert_eq!(normalize_zip_path("OEBPS/sub", "./ch1.xhtml"), "OEBPS/sub/ch1.xhtml");
    }

    #[test]
    fn parse_opf_manifest_extracts_items() {
        let opf = r#"
            <metadata><dc:title>Test Book</dc:title></metadata>
            <manifest>
                <item id="ch1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
                <item id="ch2" href="ch2.xhtml" media-type="application/xhtml+xml"/>
            </manifest>
        "#;
        let manifest = parse_opf_manifest(opf);
        assert_eq!(manifest.get("ch1").map(|s| s.as_str()), Some("ch1.xhtml"));
        assert_eq!(manifest.get("ch2").map(|s| s.as_str()), Some("ch2.xhtml"));
    }

    #[test]
    fn parse_opf_spine_extracts_idrefs() {
        let opf = r#"<spine toc="ncx"><itemref idref="ch1"/><itemref idref="ch2"/></spine>"#;
        let spine = parse_opf_spine(opf);
        assert_eq!(spine, vec!["ch1".to_string(), "ch2".to_string()]);
    }

    #[test]
    fn extract_dc_title_works() {
        let opf = r#"<metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title id="t1">My Book</dc:title></metadata>"#;
        assert_eq!(extract_dc_title(opf).as_deref(), Some("My Book"));
    }

    #[test]
    fn extract_opf_path_from_container() {
        let container = r#"<?xml version="1.0"?>
            <container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
                <rootfiles>
                    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
                </rootfiles>
            </container>"#;
        assert_eq!(extract_opf_path(container).as_deref(), Some("OEBPS/content.opf"));
    }
}
