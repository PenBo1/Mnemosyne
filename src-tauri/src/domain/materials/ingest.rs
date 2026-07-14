// 材料导入: 从 URL 或本地文件抓取 → 按 mime 分发解析 → 落盘 markdown + JSON 清单。
//
// 复用 infrastructure::llm::embedding::ingest 的文本提取能力(PDF / HTML / Text),
// 避免重复实现解析逻辑(符合"共享能力抽取为可复用模块,禁止 copy-paste")。
//
// 安全约束:
// - URL 仅允许 http/https(防 SSRF)
// - 请求超时 20s,响应体上限 18MB
// - 文件来源大小上限 18MB

use std::path::Path;

use crate::infrastructure::fs::data_dir::DataDir;
use crate::infrastructure::llm::embedding::ingest::{
    extract_text_from_bytes, MaterialKind, MAX_SOURCE_BYTES,
};
use crate::shared::error::AppError;

use super::types::{IngestMaterialInput, MaterialAsset};

const EXCERPT_CHARS: usize = 1600;
const FETCH_TIMEOUT_SECS: u64 = 20;

/// 导入一份材料并落盘,返回资产清单。
pub async fn ingest_material(
    data_dir: &DataDir,
    input: &IngestMaterialInput,
) -> Result<MaterialAsset, AppError> {
    let purpose = input.purpose.as_deref().unwrap_or("reference").trim().to_string();
    if purpose.is_empty() {
        return Err(AppError::invalid_input("purpose cannot be empty"));
    }

    let source = read_source(input).await?;
    let kind = resolve_kind(&source.filename, &source.mime_type, &source.bytes);

    let extracted = extract_text_from_bytes(&source.bytes, kind)
        .map_err(|e| AppError::internal(format!("Material extract failed: {}", e)))?;
    if extracted.text.is_empty() {
        return Err(AppError::invalid_input("Material extraction returned no text"));
    }

    let title = input
        .title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or(extracted.title.as_deref())
        .or_else(|| Some(source.filename.as_str()))
        .map(|s| s.chars().take(120).collect::<String>())
        .unwrap_or_else(|| "material".to_string());

    let now = chrono::Utc::now();
    let id = format!(
        "{}-{}",
        now.to_rfc3339().replace([':', '.'], "-"),
        slug(&title)
    );

    let materials_dir = data_dir.materials_dir();
    std::fs::create_dir_all(&materials_dir)
        .map_err(|e| AppError::internal(format!("Failed to create materials dir: {}", e)))?;

    let markdown = render_markdown(
        &title,
        kind_name(kind),
        &purpose,
        &source.source,
        &source.mime_type,
        extracted.total_pages,
        &extracted.text,
    );
    let markdown_filename = format!("{}.md", id);
    let manifest_filename = format!("{}.json", id);
    let markdown_path = materials_dir.join(&markdown_filename);
    let manifest_path = materials_dir.join(&manifest_filename);

    std::fs::write(&markdown_path, &markdown)
        .map_err(|_e| AppError::file_write_error(markdown_path.display().to_string()))?;

    let excerpt = safe_slice(&extracted.text, 0, EXCERPT_CHARS);
    let asset = MaterialAsset {
        id: id.clone(),
        title: title.clone(),
        kind: kind_name(kind).to_string(),
        purpose,
        source: source.source.clone(),
        mime_type: source.mime_type.clone(),
        markdown_path: markdown_filename,
        manifest_path: manifest_filename,
        char_count: extracted.text.chars().count() as u32,
        excerpt,
        total_pages: extracted.total_pages.map(|n| n as u32),
    };

    let manifest_json = serde_json::to_string_pretty(&asset)
        .map_err(|e| AppError::internal(format!("Manifest serialize failed: {}", e)))?;
    std::fs::write(&manifest_path, manifest_json)
        .map_err(|_e| AppError::file_write_error(manifest_path.display().to_string()))?;

    tracing::info!(
        material_id = %asset.id,
        kind = %asset.kind,
        char_count = asset.char_count,
        "Material ingested"
    );
    Ok(asset)
}

// ── 来源读取 ─────────────────────────────────────────────────

struct SourceBuffer {
    bytes: Vec<u8>,
    source: String,
    filename: String,
    mime_type: String,
}

async fn read_source(input: &IngestMaterialInput) -> Result<SourceBuffer, AppError> {
    match input.source_kind.as_str() {
        "url" => {
            let url = input
                .url
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| AppError::missing_field("url"))?;
            read_url(url).await
        }
        "file" => {
            let file_path = input
                .file_path
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| AppError::missing_field("filePath"))?;
            read_file(file_path, input.filename.as_deref(), input.mime_type.as_deref())
        }
        other => Err(AppError::invalid_input(format!(
            "Unsupported sourceKind: {} (expected url/file)",
            other
        ))),
    }
}

async fn read_url(url: &str) -> Result<SourceBuffer, AppError> {
    validate_url_scheme(url)?;
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; Mnemosyne/0.1)")
        .timeout(std::time::Duration::from_secs(FETCH_TIMEOUT_SECS))
        .build()
        .map_err(|e| AppError::internal(format!("HTTP client build failed: {}", e)))?;

    let resp = client
        .get(url)
        .header(
            "Accept",
            "text/html, text/plain, application/json, application/pdf, */*",
        )
        .send()
        .await
        .map_err(|e| AppError::internal(format!("Fetch failed: {}", e)))?;
    if !resp.status().is_success() {
        return Err(AppError::internal(format!(
            "Fetch failed: {} {}",
            resp.status().as_u16(),
            resp.status().canonical_reason().unwrap_or("")
        )));
    }
    let mime_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(';').next().unwrap_or("").trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| mime_from_url(url));

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| AppError::internal(format!("Read body failed: {}", e)))?
        .to_vec();
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err(AppError::invalid_input(format!(
            "Fetched material too large ({} bytes, max {})",
            bytes.len(),
            MAX_SOURCE_BYTES
        )));
    }

    let filename = filename_from_url(url);
    Ok(SourceBuffer {
        bytes,
        source: url.to_string(),
        filename,
        mime_type,
    })
}

fn read_file(
    file_path: &str,
    filename_override: Option<&str>,
    mime_override: Option<&str>,
) -> Result<SourceBuffer, AppError> {
    let path = Path::new(file_path);
    let meta = std::fs::metadata(path)
        .map_err(|e| AppError::file_not_found(format!("{}: {}", file_path, e)))?;
    if meta.len() as usize > MAX_SOURCE_BYTES {
        return Err(AppError::invalid_input(format!(
            "File too large ({} bytes, max {})",
            meta.len(),
            MAX_SOURCE_BYTES
        )));
    }
    let bytes = std::fs::read(path)
        .map_err(|_e| AppError::file_read_error(file_path))?;
    let filename = filename_override
        .map(str::to_string)
        .or_else(|| {
            path.file_name()
                .and_then(|s| s.to_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "material".to_string());
    let mime_type = mime_override
        .map(str::to_string)
        .unwrap_or_else(|| mime_from_filename(&filename));
    Ok(SourceBuffer {
        bytes,
        source: file_path.to_string(),
        filename,
        mime_type,
    })
}

// ── 类型识别 ─────────────────────────────────────────────────

fn resolve_kind(filename: &str, mime_type: &str, _bytes: &[u8]) -> MaterialKind {
    if is_pdf(filename, mime_type) {
        return MaterialKind::Pdf;
    }
    if is_html(filename, mime_type) {
        return MaterialKind::Html;
    }
    MaterialKind::Text
}

fn is_pdf(filename: &str, mime_type: &str) -> bool {
    mime_type.contains("pdf") || has_extension(filename, &["pdf"])
}

fn is_html(filename: &str, mime_type: &str) -> bool {
    mime_type.contains("html") || has_extension(filename, &["html", "htm"])
}

fn has_extension(filename: &str, exts: &[&str]) -> bool {
    let lower = filename.to_lowercase();
    exts.iter().any(|e| lower.ends_with(&format!(".{}", e)))
}

fn kind_name(kind: MaterialKind) -> &'static str {
    match kind {
        MaterialKind::Html => "webpage",
        MaterialKind::Pdf => "pdf",
        MaterialKind::Text | MaterialKind::Epub => "text",
    }
}

fn mime_from_filename(filename: &str) -> String {
    let lower = filename.to_lowercase();
    if lower.ends_with(".pdf") {
        "application/pdf".into()
    } else if lower.ends_with(".html") || lower.ends_with(".htm") {
        "text/html".into()
    } else if lower.ends_with(".json") {
        "application/json".into()
    } else if lower.ends_with(".md") || lower.ends_with(".markdown") {
        "text/markdown".into()
    } else if lower.ends_with(".csv") {
        "text/csv".into()
    } else {
        "text/plain".into()
    }
}

fn mime_from_url(url: &str) -> String {
    let path = url::Url::parse(url)
        .ok()
        .and_then(|u| {
            u.path_segments()
                .and_then(|mut s| s.next_back().map(str::to_string))
        })
        .unwrap_or_default();
    if path.is_empty() {
        "text/html".into()
    } else {
        mime_from_filename(&path)
    }
}

fn filename_from_url(url: &str) -> String {
    let parsed = match url::Url::parse(url) {
        Ok(u) => u,
        Err(_) => return "index".into(),
    };
    let last = parsed
        .path_segments()
        .and_then(|mut s| s.next_back())
        .unwrap_or("");
    if last.is_empty() {
        parsed
            .host_str()
            .map(str::to_string)
            .unwrap_or_else(|| "index".into())
    } else {
        last.to_string()
    }
}

fn validate_url_scheme(url: &str) -> Result<(), AppError> {
    let parsed = url::Url::parse(url)
        .map_err(|e| AppError::invalid_input(format!("Invalid URL: {}", e)))?;
    match parsed.scheme() {
        "http" | "https" => Ok(()),
        s => Err(AppError::invalid_input(format!(
            "URL scheme '{}' not allowed (http/https only)",
            s
        ))),
    }
}

// ── markdown 渲染 ───────────────────────────────────────────

fn render_markdown(
    title: &str,
    kind: &str,
    purpose: &str,
    source: &str,
    mime_type: &str,
    total_pages: Option<usize>,
    text: &str,
) -> String {
    let mut lines: Vec<String> = vec![
        format!("# {}", title),
        String::new(),
        "## Metadata".into(),
        format!("- kind: {}", kind),
        format!("- purpose: {}", purpose),
        format!("- source: {}", source),
        format!("- mime_type: {}", mime_type),
    ];
    if let Some(pages) = total_pages {
        lines.push(format!("- total_pages: {}", pages));
    }
    lines.push(format!("- char_count: {}", text.chars().count()));
    lines.push(String::new());
    lines.push("## Extracted content".into());
    lines.push(text.to_string());
    lines.push(String::new());
    lines.join("\n")
}

/// 按字符边界安全切片,避免切断多字节 UTF-8 字符。
fn safe_slice(s: &str, start: usize, len: usize) -> String {
    let end = start.saturating_add(len);
    let take: String = s.chars().skip(start).take(len).collect();
    let _ = end; // 仅用于语义表达,实际由 chars 迭代器保证边界
    take
}

/// slug: 小写化,非字母数字字符替换为 `-`,去首尾 `-`,截断 80 字符,空时回退 "material"。
fn slug(value: &str) -> String {
    let re = regex::Regex::new(r"[^\p{L}\p{N}]+").expect("slug regex");
    let slugified = re.replace_all(value.trim(), "-");
    let lower = slugified.to_lowercase();
    let trimmed = lower.trim_matches('-');
    let result: String = trimmed.chars().take(80).collect();
    if result.is_empty() {
        "material".into()
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_handles_chinese_and_punctuation() {
        assert_eq!(slug("Hello, World!"), "hello-world");
        assert_eq!(slug("三体：地球往事"), "三体-地球往事");
        assert_eq!(slug("   "), "material");
    }

    #[test]
    fn resolve_kind_dispatches_by_mime() {
        assert_eq!(resolve_kind("a.pdf", "application/pdf", &[]), MaterialKind::Pdf);
        assert_eq!(resolve_kind("a.html", "text/html", &[]), MaterialKind::Html);
        assert_eq!(resolve_kind("a.txt", "text/plain", &[]), MaterialKind::Text);
    }

    #[test]
    fn validate_url_scheme_rejects_file() {
        assert!(validate_url_scheme("file:///etc/passwd").is_err());
        assert!(validate_url_scheme("https://example.com").is_ok());
    }

    #[test]
    fn safe_slice_respects_char_boundary() {
        let s = "你好世界hello";
        let excerpt = safe_slice(s, 0, 6);
        assert_eq!(excerpt, "你好世界he");
    }

    #[test]
    fn render_markdown_contains_metadata() {
        let md = render_markdown(
            "Title",
            "webpage",
            "reference",
            "https://x.com",
            "text/html",
            None,
            "body text",
        );
        assert!(md.contains("# Title"));
        assert!(md.contains("- kind: webpage"));
        assert!(md.contains("body text"));
    }
}
