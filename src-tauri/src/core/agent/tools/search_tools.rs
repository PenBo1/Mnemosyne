// 书籍目录搜索工具：GrepTool（内容搜索）+ LsTool（列目录）。
//
// 与 fs_tools 的区别：根目录锁定为 DataDir.books_dir()，path 参数相对该根解析，
// 防止 agent 越权访问书籍工作区之外的文件。

use std::path::PathBuf;

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;

use crate::infrastructure::fs::data_dir::DataDir;
use crate::infrastructure::fs::fs_utils::MAX_READ_SIZE;

// ── GrepTool ──────────────────────────────────────────────

/// 在书籍目录下按正则搜索文件内容，返回匹配行。
pub struct GrepTool {
    pub data_dir: DataDir,
}

#[derive(Deserialize)]
pub struct GrepArgs {
    /// 正则表达式
    pub pattern: String,
    /// 相对 books_dir 的路径（空字符串或 "." 表示根目录）
    #[serde(default)]
    pub path: String,
    /// 可选文件扩展名过滤，如 "md" 或 "*.md"
    #[serde(default)]
    pub glob: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum GrepError {
    #[error("IO 错误: {0}")]
    Io(String),
    #[error("路径穿越被禁止")]
    PathTraversal,
    #[error("非法正则: {0}")]
    InvalidRegex(String),
}

/// 单次返回的匹配行上限，避免超大输出压垮 LLM 上下文。
const MAX_GREP_LINES: usize = 200;

impl Tool for GrepTool {
    const NAME: &'static str = "grep";

    type Error = GrepError;
    type Args = GrepArgs;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "在书籍工作区下按正则搜索文件内容，返回匹配的行（含文件相对路径与行号）。".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "正则表达式"
                    },
                    "path": {
                        "type": "string",
                        "description": "相对 books_dir 的搜索起始路径（可选，缺省为根目录）"
                    },
                    "glob": {
                        "type": "string",
                        "description": "可选文件扩展名过滤，如 \"md\" 或 \"*.md\""
                    }
                },
                "required": ["pattern"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // GrepTool 内部使用 `std::fs::read_dir` / `std::fs::read_to_string` / `std::fs::metadata`
        // 等同步 I/O，整体卸载到 `spawn_blocking` 避免阻塞 tokio runtime。
        let root = self.data_dir.books_dir();
        let root_clone = root.clone();
        let result = tokio::task::spawn_blocking(move || -> Result<serde_json::Value, GrepError> {
            let re = regex::Regex::new(&args.pattern)
                .map_err(|e| GrepError::InvalidRegex(e.to_string()))?;

            let search_root = resolve_under_root(&root_clone, &args.path)?;
            let ext_filter = args.glob.as_deref().map(normalize_glob);

            let mut matches: Vec<serde_json::Value> = Vec::new();
            collect_matches(&search_root, &root_clone, &re, ext_filter.as_deref(), &mut matches)
                .map_err(|e| GrepError::Io(e.to_string()))?;

            let total = matches.len();
            let truncated = total > MAX_GREP_LINES;
            if truncated {
                matches.truncate(MAX_GREP_LINES);
            }

            Ok(serde_json::json!({
                "matches": matches,
                "count": total,
                "truncated": truncated,
            }))
        })
        .await
        .map_err(|e| GrepError::Io(format!("spawn_blocking join failed: {}", e)))??;
        Ok(result)
    }
}

/// 递归收集匹配行
fn collect_matches(
    dir: &std::path::Path,
    root: &std::path::Path,
    re: &regex::Regex,
    ext_filter: Option<&str>,
    out: &mut Vec<serde_json::Value>,
) -> std::io::Result<()> {
    if out.len() >= MAX_GREP_LINES {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            // 跳过 snapshots 与隐藏临时目录
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name == "snapshots" || name.starts_with(".tmp") {
                    continue;
                }
            }
            collect_matches(&path, root, re, ext_filter, out)?;
        } else if ft.is_file() {
            if let Some(ext) = ext_filter {
                if !path_extension_matches(&path, ext) {
                    continue;
                }
            }
            // 仅搜索文本文件
            if !is_text_file(&path) {
                continue;
            }
            // 跳过超大文件，避免 OOM
            if std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0) > MAX_READ_SIZE as u64 {
                continue;
            }
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let rel = path.strip_prefix(root)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| path.to_string_lossy().to_string());
            for (i, line) in content.lines().enumerate() {
                if re.is_match(line) {
                    out.push(serde_json::json!({
                        "file": rel,
                        "line": i + 1,
                        "text": line,
                    }));
                    if out.len() >= MAX_GREP_LINES {
                        return Ok(());
                    }
                }
            }
        }
    }
    Ok(())
}

// ── LsTool ────────────────────────────────────────────────

/// 列出书籍目录下的文件与子目录。
pub struct LsTool {
    pub data_dir: DataDir,
}

#[derive(Deserialize)]
pub struct LsArgs {
    /// 相对 books_dir 的路径（空字符串或 "." 表示根目录）
    #[serde(default)]
    pub path: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LsError {
    #[error("IO 错误: {0}")]
    Io(String),
    #[error("路径穿越被禁止")]
    PathTraversal,
}

impl Tool for LsTool {
    const NAME: &'static str = "ls";

    type Error = LsError;
    type Args = LsArgs;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "列出书籍工作区下指定目录的文件与子目录。".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "相对 books_dir 的目录路径（可选，缺省为根目录）"
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let root = self.data_dir.books_dir();
        let target = resolve_under_root_ls(&root, &args.path)?;

        let mut entries: Vec<serde_json::Value> = Vec::new();
        let mut reader = tokio::fs::read_dir(&target).await
            .map_err(|e| LsError::Io(e.to_string()))?;
        while let Some(entry) = reader.next_entry().await
            .map_err(|e| LsError::Io(e.to_string()))? {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().await
                .map(|ft| ft.is_dir())
                .unwrap_or(false);
            entries.push(serde_json::json!({
                "name": name,
                "is_dir": is_dir,
            }));
        }
        entries.sort_by(|a, b| {
            let a_dir = a.get("is_dir").and_then(|v| v.as_bool()).unwrap_or(false);
            let b_dir = b.get("is_dir").and_then(|v| v.as_bool()).unwrap_or(false);
            match (a_dir, b_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => {
                    let an = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let bn = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    an.cmp(bn)
                }
            }
        });

        Ok(serde_json::json!({
            "path": args.path,
            "entries": entries,
        }))
    }
}

// ── 辅助函数 ──────────────────────────────────────────────

/// 将相对路径解析到 root 之下，并校验未越界。
/// 空路径或 "." 视为 root 本身。拒绝绝对路径与显式 `..` 穿越片段。
fn resolve_under_root(root: &PathBuf, rel: &str) -> Result<PathBuf, GrepError> {
    let trimmed = rel.trim();
    let resolved = if trimmed.is_empty() || trimmed == "." {
        root.clone()
    } else {
        if trimmed.starts_with('/') || trimmed.starts_with('\\') || trimmed.contains("..") {
            return Err(GrepError::PathTraversal);
        }
        root.join(trimmed)
    };
    if !resolved.starts_with(root) {
        return Err(GrepError::PathTraversal);
    }
    Ok(resolved)
}

/// LsTool 复用相同的路径解析逻辑（错误类型不同，单独实现避免泛型）
fn resolve_under_root_ls(root: &PathBuf, rel: &str) -> Result<PathBuf, LsError> {
    let trimmed = rel.trim();
    let resolved = if trimmed.is_empty() || trimmed == "." {
        root.clone()
    } else {
        if trimmed.starts_with('/') || trimmed.starts_with('\\') || trimmed.contains("..") {
            return Err(LsError::PathTraversal);
        }
        root.join(trimmed)
    };
    if !resolved.starts_with(root) {
        return Err(LsError::PathTraversal);
    }
    Ok(resolved)
}

/// 规范化 glob 为纯扩展名（"*.md" → "md"，"md" → "md"）
fn normalize_glob(g: &str) -> String {
    let g = g.trim();
    if let Some(stripped) = g.strip_prefix("*.") {
        return stripped.to_lowercase();
    }
    g.to_lowercase()
}

fn path_extension_matches(path: &std::path::Path, ext: &str) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case(ext))
        .unwrap_or(false)
}

fn is_text_file(path: &std::path::Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref(),
        Some("md") | Some("json") | Some("yaml") | Some("yml") | Some("txt")
    )
}
