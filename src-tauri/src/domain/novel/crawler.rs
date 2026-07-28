//! ═══════════════════════════════════════════════════════════════════════════
//! 小说爬虫 - 小说下载与搜索
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 三个核心入口:
//! - `search`: 执行搜索(GET 或 POST),返回匹配书目
//! - `download`: 抓取书籍详情 → 目录 → 章节正文,合并为单个 TXT 文件
//! - `list_local`: 列出本地已下载的小说文件

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use tokio::task::JoinSet;

use crate::domain::novel::http;
use crate::domain::novel::parser::{book as book_parser, chapter as chapter_parser, search as search_parser, toc as toc_parser};
use crate::domain::novel::source;
use crate::domain::novel::types::{BookSource, LocalBookItem, SearchBookResult};
use crate::shared::error::AppError;

/// 单章节下载的最大重试次数。
const MAX_RETRY: usize = 2;
/// 并发下载章节数上限。
const MAX_CONCURRENCY: usize = 10;

/// 执行搜索。`source_name="all"` 时聚合所有未禁用且支持搜索的书源。
pub async fn search(
    sources: &[BookSource],
    source_name: &str,
    keyword: &str,
) -> Result<Vec<SearchBookResult>, AppError> {
    let start = std::time::Instant::now();
    tracing::info!(source_name, keyword_len = keyword.len(), "novel_search: enter");
    
    let targets: Vec<&BookSource> = if source_name == "all" {
        sources
            .iter()
            .filter(|s| s.enabled && s.search.as_ref().map(|x| !x.disabled).unwrap_or(false))
            .collect()
    } else {
        let s = source::find_source(sources, source_name).ok_or_else(|| {
            AppError::not_found(format!("book source `{}` not found", source_name))
        })?;
        if !s.enabled {
            return Err(AppError::invalid_input(format!(
                "book source `{}` is disabled",
                source_name
            )));
        }
        vec![s]
    };

    if targets.is_empty() {
        tracing::info!(
            result_count = 0,
            duration_ms = start.elapsed().as_millis(),
            "novel_search: exit (no targets)"
        );
        return Ok(Vec::new());
    }

    let mut tasks = JoinSet::new();
    for src in targets {
        let src = src.clone();
        let kw = keyword.to_string();
        tasks.spawn(async move { search_one(&src, &kw).await });
    }

    let mut all = Vec::new();
    while let Some(res) = tasks.join_next().await {
        match res {
            Ok(Ok(list)) => all.extend(list),
            Ok(Err(e)) => tracing::warn!(error = %e, "search one source failed"),
            Err(e) => tracing::warn!(error = %e, "search task panicked"),
        }
    }
    
    tracing::info!(
        result_count = all.len(),
        duration_ms = start.elapsed().as_millis(),
        "novel_search: exit"
    );
    Ok(all)
}

/// 在单个书源上执行搜索。
async fn search_one(src: &BookSource, keyword: &str) -> Result<Vec<SearchBookResult>, AppError> {
    let rule = src
        .search
        .as_ref()
        .ok_or_else(|| AppError::invalid_input(format!("source `{}` has no search rule", src.name)))?;

    let html = if rule.method.eq_ignore_ascii_case("post") {
        http::post_form_html(&rule.url, &rule.cookies, &rule.data, &[keyword]).await?
    } else {
        // GET 风格:若 url 含 %s 则替换为关键字(URL 编码),否则 url 本身即完整地址
        let url = if rule.url.contains("%s") {
            rule.url.replace("%s", &urlencoding::encode(keyword))
        } else {
            rule.url.clone()
        };
        http::get_html(&url, &rule.cookies).await?
    };

    let base_url = rule.url.split('?').next().unwrap_or(&rule.url);
    search_parser::parse(src, &html, base_url)
}

/// 下载整本小说。流程:
/// 1. 抓取详情页 → 书名/作者
/// 2. 抓取目录页 → 章节列表
/// 3. 并发抓取章节正文
/// 4. 合并为 `<novels_dir>/<书名> (<作者>).txt`
///
/// 返回最终 TXT 文件的绝对路径。
pub async fn download(
    source: &BookSource,
    book_url: &str,
    novels_dir: &Path,
) -> Result<PathBuf, AppError> {
    let start = Instant::now();
    tracing::info!(function = "download", source = %source.name, book_url, "入口");

    let result = async {
        let detail_html = http::get_html(book_url, "").await?;
        let detail = book_parser::parse(source, book_url, &detail_html)?;

        let toc_html = http::get_html(book_url, "").await?;
        let chapters = toc_parser::parse(source, &toc_html, book_url)?;
        if chapters.is_empty() {
            return Err(AppError::internal("chapter list is empty (possibly anti-crawl)"));
        }

        tracing::info!(
            source = %source.name,
            book = %detail.book_name,
            chapters = chapters.len(),
            "starting download"
        );

        let source_arc = Arc::new(source.clone());
        let total = chapters.len();
        let mut tasks = JoinSet::new();
        let mut failed_chapters: Vec<String> = Vec::new();
        for ch in chapters {
            let src = source_arc.clone();
            tasks.spawn(async move {
                let mut last_err: Option<String> = None;
                for attempt in 0..=MAX_RETRY {
                    match fetch_chapter_content(&src, &ch.url).await {
                        Ok(content) => return Ok((ch.index, content)),
                        Err(e) => {
                            last_err = Some(e.to_string());
                            if attempt < MAX_RETRY {
                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            }
                        }
                    }
                }
                Err(last_err.unwrap_or_else(|| "unknown error".into()))
            });
            if tasks.len() >= MAX_CONCURRENCY {
                if let Some(res) = tasks.join_next().await {
                    handle_chapter_result(res, &mut failed_chapters)?;
                }
            }
        }

        let mut results: Vec<(usize, String)> = Vec::with_capacity(total);
        while let Some(res) = tasks.join_next().await {
            if let Some((idx, content)) = handle_chapter_result(res, &mut failed_chapters)? {
                results.push((idx, content));
            }
        }
        results.sort_by_key(|(i, _)| *i);

        if !failed_chapters.is_empty() {
            tracing::warn!(
                total = total,
                succeeded = results.len(),
                failed = failed_chapters.len(),
                "Download completed with partial failures"
            );
        }

        let filename = sanitize_filename(&format!("{} ({}).txt", detail.book_name, detail.author));
        let out_path = novels_dir.join(filename);
        let mut out = String::new();
        out.push_str(&format!("书名：{}\n", detail.book_name));
        out.push_str(&format!("作者：{}\n", detail.author));
        let intro_text = if detail.intro.is_empty() {
            "暂无".to_string()
        } else {
            strip_html(&detail.intro)
        };
        out.push_str(&format!("简介：{}\n\n", intro_text));
        for (_, content) in results {
            out.push_str(&content);
            out.push_str("\n\n");
        }

        let novels_dir_owned = novels_dir.to_path_buf();
        let out_path = tokio::task::spawn_blocking(move || -> Result<PathBuf, AppError> {
            std::fs::create_dir_all(&novels_dir_owned)
                .map_err(|e| AppError::internal(format!("failed to create novels dir: {}", e)))?;
            std::fs::write(&out_path, out)
                .map_err(|e| AppError::internal(format!("failed to write novel file: {}", e)))?;
            Ok(out_path)
        })
        .await
        .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;

        tracing::info!(path = %out_path.display(), "download complete");
        Ok(out_path)
    }.await;

    match &result {
        Ok(path) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::info!(function = "download", duration_ms, path = %path.display(), "出口");
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::error!(function = "download", duration_ms, error = %e, "错误");
        }
    }
    result
}

/// 抓取单个章节正文并按书源规则解析。
async fn fetch_chapter_content(source: &BookSource, url: &str) -> Result<String, AppError> {
    let html = http::get_html(url, "").await?;
    chapter_parser::parse(source, &html, "")
}

/// `handle_chapter_result` 把 `JoinSet::join_next` 的 `Result<Result<T, E>, JoinError>` 拍平:
/// 任务 panic 或最终失败都返回 Ok(None) 表示跳过该章节,并将失败原因累计到 `failed`。
fn handle_chapter_result(
    res: Result<Result<(usize, String), String>, tokio::task::JoinError>,
    failed: &mut Vec<String>,
) -> Result<Option<(usize, String)>, AppError> {
    match res {
        Ok(Ok(v)) => Ok(Some(v)),
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "chapter download failed after retries, skipping");
            failed.push(e);
            Ok(None)
        }
        Err(e) => {
            tracing::warn!(error = %e, "chapter task panicked, skipping");
            failed.push(format!("task panicked: {}", e));
            Ok(None)
        }
    }
}

/// 列出本地已下载的小说文件(.txt)。
pub fn list_local(novels_dir: &Path) -> Result<Vec<LocalBookItem>, AppError> {
    if !novels_dir.exists() {
        return Ok(Vec::new());
    }
    let mut items = Vec::new();
    let entries = std::fs::read_dir(novels_dir)
        .map_err(|_| AppError::file_read_error(novels_dir.display().to_string()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let size = entry
            .metadata()
            .map(|m| m.len())
            .unwrap_or(0);
        let timestamp = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        items.push(LocalBookItem {
            name,
            size,
            timestamp,
        });
    }
    // 按修改时间倒序
    items.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(items)
}

/// 简单的文件名净化:替换 Windows / POSIX 非法字符为下划线。
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

/// 去除 HTML 标签(用于详情简介在 TXT 中输出前的清理)。
fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_replaces_invalid_chars() {
        assert_eq!(sanitize_filename("a/b:c?"), "a_b_c_");
        assert_eq!(sanitize_filename("正常书名.txt"), "正常书名.txt");
    }

    #[test]
    fn strip_html_removes_tags() {
        assert_eq!(strip_html("<p>简介</p>"), "简介");
        assert_eq!(strip_html("<b>加粗</b><i>斜体</i>"), "加粗斜体");
    }

    #[test]
    fn list_local_handles_missing_dir() {
        let path = PathBuf::from("/tmp/__mnemosyne_no_such_dir__");
        let result = list_local(&path).unwrap();
        assert!(result.is_empty());
    }
}
