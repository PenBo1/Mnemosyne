// ── Chat Attachments ───────────────────────────────────────
//
// 附件解析模块：把用户通过操作栏选择的附件（文件/wiki/章节/文本）解析为
// 可注入 LLM 上下文的文本块。
//
// 设计原则：
// - 前端只传「引用」（路径/wiki_id/章节号），Rust 端读取实际内容
// - 路径不暴露给 LLM（信任边界），只把文件内容注入上下文
// - file 类型由 Rust 端读取，路径经安全性校验（traversal + 大小上限）
// - 解析失败的附件跳过并记录日志，不阻断整个请求（fail-soft 但显式 log）

use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::shared::errors::AppError;
use crate::infrastructure::db::Database;

/// 单个附件的最大文件大小（1MB），超过则拒绝读取。
const MAX_ATTACHMENT_FILE_SIZE: u64 = 1_000_000;

/// 附件类型（与前端 attachment.ts 对齐）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AttachmentKind {
    File,
    Wiki,
    Chapter,
    Text,
}

/// 附件规格（前端传入）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentSpec {
    pub kind: AttachmentKind,
    /// file: 文件绝对路径；wiki: entry_id；chapter: chapter_number（字符串）；text: 无
    pub r#ref: String,
    /// 显示名称（前端展示用，不传给 LLM）
    pub label: String,
    /// text 类型时的文本内容；其他类型为空，由 Rust 端读取
    #[serde(default)]
    pub content: Option<String>,
}

/// 解析后的上下文块
#[derive(Debug, Clone)]
pub struct ResolvedAttachment {
    pub label: String,
    pub content: String,
}

/// 把附件列表解析为上下文文本块。
///
/// - file: 校验路径安全性后读取文件内容（仅文本，UTF-8）
/// - wiki: 按 entry_id 从 DB 读取 wiki 条目内容
/// - chapter: 按 workspace_id + chapter_number 读取章节文件
/// - text: 直接使用 content 字段
///
/// 解析失败的附件跳过并 log warn，不阻断整个请求。
pub async fn resolve_attachments(
    db: &Database,
    attachments: &[AttachmentSpec],
    workspace_path: Option<&Path>,
) -> Vec<ResolvedAttachment> {
    let mut results = Vec::with_capacity(attachments.len());
    for att in attachments {
        match att.kind {
            AttachmentKind::Text => {
                if let Some(content) = &att.content {
                    if !content.is_empty() {
                        results.push(ResolvedAttachment {
                            label: att.label.clone(),
                            content: content.clone(),
                        });
                    }
                }
            }
            AttachmentKind::File => {
                match read_file_safe(&att.r#ref) {
                    Ok(content) => results.push(ResolvedAttachment {
                        label: att.label.clone(),
                        content,
                    }),
                    Err(e) => {
                        tracing::warn!(path = %att.r#ref, error = %e, "Failed to read attachment file, skipping");
                    }
                }
            }
            AttachmentKind::Wiki => {
                match db.get_wiki_entry(&att.r#ref).await {
                    Ok(Some(entry)) => {
                        let content = format!("# {}\n\n分类: {}\n\n{}", entry.title, entry.category, entry.content);
                        results.push(ResolvedAttachment {
                            label: entry.title,
                            content,
                        });
                    }
                    Ok(None) => {
                        tracing::warn!(entry_id = %att.r#ref, "Wiki entry not found, skipping attachment");
                    }
                    Err(e) => {
                        tracing::warn!(entry_id = %att.r#ref, error = %e, "Failed to load wiki entry, skipping attachment");
                    }
                }
            }
            AttachmentKind::Chapter => {
                if let Some(ws_path) = workspace_path {
                    match resolve_chapter(ws_path, &att.r#ref) {
                        Ok(content) => results.push(ResolvedAttachment {
                            label: att.label.clone(),
                            content,
                        }),
                        Err(e) => {
                            tracing::warn!(chapter = %att.r#ref, error = %e, "Failed to read chapter file, skipping attachment");
                        }
                    }
                } else {
                    tracing::warn!(chapter = %att.r#ref, "No active workspace, cannot resolve chapter attachment");
                }
            }
        }
    }
    results
}

/// 安全读取文件：校验大小 + 路径遍历，仅读 UTF-8 文本。
fn read_file_safe(path_str: &str) -> Result<String, AppError> {
    // 路径遍历保护：拒绝包含 .. 的路径
    if path_str.contains("..") {
        return Err(AppError::invalid_input("Path traversal not allowed in attachment"));
    }

    let path = Path::new(path_str);

    // 存在性 + 是否目录
    if !path.exists() {
        return Err(AppError::not_found("Attachment file not found"));
    }
    if path.is_dir() {
        return Err(AppError::invalid_input("Attachment path is a directory, not a file"));
    }

    // 大小校验
    let metadata = std::fs::metadata(path)
        .map_err(|e| AppError::internal(format!("Failed to read file metadata: {}", e)))?;
    if metadata.len() > MAX_ATTACHMENT_FILE_SIZE {
        return Err(AppError::invalid_input(format!(
            "Attachment file too large (max {} bytes, got {})",
            MAX_ATTACHMENT_FILE_SIZE, metadata.len()
        )));
    }

    // 读取（仅 UTF-8 文本）
    std::fs::read_to_string(path)
        .map_err(|e| AppError::internal(format!("Failed to read attachment file: {}", e)))
}

/// 从 workspace 路径解析章节文件。
///
/// 章节文件约定：`<workspace>/chapters/<chapter_number>.md`
fn resolve_chapter(workspace_path: &Path, chapter_ref: &str) -> Result<String, AppError> {
    let chapter_number: u32 = chapter_ref
        .parse()
        .map_err(|_| AppError::invalid_input(format!("Invalid chapter number: {}", chapter_ref)))?;

    let chapter_file = workspace_path.join("chapters").join(format!("{}.md", chapter_number));

    if !chapter_file.exists() {
        return Err(AppError::not_found(format!("Chapter {} file not found", chapter_number)));
    }

    std::fs::read_to_string(&chapter_file)
        .map_err(|e| AppError::internal(format!("Failed to read chapter file: {}", e)))
}

/// 把解析后的附件列表格式化为可注入 system_prompt 的上下文块。
pub fn format_attachments_context(attachments: &[ResolvedAttachment]) -> String {
    if attachments.is_empty() {
        return String::new();
    }
    let mut out = String::from("\n\n─ 用户添加的上下文 ─────────────────────\n");
    for (i, att) in attachments.iter().enumerate() {
        out.push_str(&format!("\n[附件 {}] {}\n{}\n", i + 1, att.label, att.content));
    }
    out.push_str("─────────────────────────────────────────\n");
    out
}
