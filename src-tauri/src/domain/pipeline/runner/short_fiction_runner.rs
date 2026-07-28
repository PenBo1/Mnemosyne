//! ═══════════════════════════════════════════════════════════════════════════
//! Short Fiction Runner - 短篇 Pipeline 编排器
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 短篇 pipeline 编排器：create_outline -> review_outline -> revise_outline ->
//! write_draft (with continue_draft retry) -> review_draft -> revise_draft ->
//! write_final_artifacts -> generate_package。
//!
//! 文件系统操作用 std::fs 同步 API（与 pipeline_runner.rs 一致），LLM 调用走 AgentEngine。
//! 封面图片生成未实现（仅落盘 cover-prompt.md）。

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Instant;

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;
use super::super::agents::short_fiction::{
    self, ShortFictionBatchDraft, ShortFictionReference, ShortFictionSalesPackage,
    SHORT_FICTION_DEFAULT_CHAPTERS, SHORT_FICTION_DEFAULT_CHARS_PER_CHAPTER,
    SHORT_FICTION_DRAFT_COMPLETION_ATTEMPTS, SHORT_FICTION_EN_DEFAULT_WORDS_PER_CHAPTER,
    SHORT_FICTION_MAX_CHAPTERS, SHORT_FICTION_MIN_CHAPTERS,
};
use super::super::types::Language;
use super::super::utils::fs_safety::{safe_segment, slugify};

// ── 本地常量 ─────────────────────────────────────────────────
// zh/en 字数边界
const SHORT_FICTION_MIN_CHARS_PER_CHAPTER: u32 = 900;
const SHORT_FICTION_MAX_CHARS_PER_CHAPTER: u32 = 1200;
const SHORT_FICTION_EN_MIN_WORDS_PER_CHAPTER: u32 = 600;
const SHORT_FICTION_EN_MAX_WORDS_PER_CHAPTER: u32 = 800;

// ── ShortFictionConfig ───────────────────────────────────────

/// 短篇 pipeline 配置
#[derive(Debug, Clone)]
pub struct ShortFictionConfig {
    pub books_dir: PathBuf,
    pub default_chapter_count: u32,
    pub default_chars_per_chapter: u32,
    pub language: Language,
}

impl Default for ShortFictionConfig {
    fn default() -> Self {
        Self {
            books_dir: PathBuf::new(),
            default_chapter_count: SHORT_FICTION_DEFAULT_CHAPTERS,
            default_chars_per_chapter: SHORT_FICTION_DEFAULT_CHARS_PER_CHAPTER,
            language: Language::Zh,
        }
    }
}

// ── ShortFictionRunOptions ──────────────────────────────────

/// 短篇运行参数
#[derive(Debug, Clone)]
pub struct ShortFictionRunOptions {
    pub story_id: Option<String>,
    pub direction: String,
    pub chapter_count: Option<u32>,
    pub chars_per_chapter: Option<u32>,
    pub language: Option<Language>,
    pub reference: Option<ShortFictionReference>,
}

// ── ShortFictionRunResult ───────────────────────────────────

/// 短篇运行结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShortFictionRunResult {
    pub story_id: String,
    pub outline_path: PathBuf,
    pub outline_review_path: PathBuf,
    pub draft_review_path: PathBuf,
    pub final_markdown_path: PathBuf,
    pub final_json_path: PathBuf,
    pub sales_package_path: PathBuf,
    pub cover_prompt_path: PathBuf,
}

// ── ShortFictionRunner ──────────────────────────────────────

/// 短篇 pipeline 编排器
pub struct ShortFictionRunner {
    config: ShortFictionConfig,
}

impl ShortFictionRunner {
    pub fn new(config: ShortFictionConfig) -> Self {
        Self { config }
    }

    /// 短篇根目录：{books_dir}/short-fiction
    fn shorts_root(&self) -> PathBuf {
        self.config.books_dir.join("short-fiction")
    }

    /// 故事目录：{books_dir}/short-fiction/{story_id}
    fn story_dir(&self, story_id: &str) -> PathBuf {
        self.shorts_root().join(story_id)
    }

    /// 运行短篇 pipeline。
    pub async fn run(
        &self,
        engine: &AgentEngine,
        options: &ShortFictionRunOptions,
    ) -> Result<ShortFictionRunResult, AppError> {
        let start = Instant::now();
        tracing::info!(function = "ShortFictionRunner::run", story_id = ?options.story_id, "入口");

        let language = options.language.unwrap_or(self.config.language);
        let chapter_count = bounded_integer(
            options.chapter_count,
            self.config.default_chapter_count,
            "chapter_count",
            SHORT_FICTION_MIN_CHAPTERS,
            SHORT_FICTION_MAX_CHAPTERS,
        )?;
        let chars_per_chapter = if language == Language::En {
            bounded_integer(
                options.chars_per_chapter,
                SHORT_FICTION_EN_DEFAULT_WORDS_PER_CHAPTER,
                "chars_per_chapter",
                SHORT_FICTION_EN_MIN_WORDS_PER_CHAPTER,
                SHORT_FICTION_EN_MAX_WORDS_PER_CHAPTER,
            )?
        } else {
            bounded_integer(
                options.chars_per_chapter,
                self.config.default_chars_per_chapter,
                "chars_per_chapter",
                SHORT_FICTION_MIN_CHARS_PER_CHAPTER,
                SHORT_FICTION_MAX_CHARS_PER_CHAPTER,
            )?
        };

        let provided_story_id = options.story_id.as_ref().map(|s| safe_segment(s, "short"));

        if let Some(sid) = &provided_story_id {
            let final_path = self.story_dir(sid).join("final").join("full.md");
            if final_path.exists() && !self.is_failed_run(sid)? {
                tracing::info!("[short-fiction] 故事 {} 已完成，跳过", sid);
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::info!(function = "ShortFictionRunner::run", story_id = %sid, duration_ms, skipped = true, "出口");
                return Ok(self.build_run_result(sid));
            }
        }

        let result = self
            .produce(
                engine,
                options,
                language,
                chapter_count,
                chars_per_chapter,
                provided_story_id.as_deref(),
            )
            .await;

        if let (Err(e), Some(sid)) = (&result, &provided_story_id) {
            let _ = self.write_run_status(sid, "failed", None, Some(&e.to_string()));
        }

        match &result {
            Ok(r) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::info!(function = "ShortFictionRunner::run", story_id = %r.story_id, duration_ms, "出口");
            }
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::error!(function = "ShortFictionRunner::run", duration_ms, error = %e, "错误");
            }
        }
        result
    }

    /// 主流程。
    #[allow(clippy::too_many_arguments)]
    async fn produce(
        &self,
        engine: &AgentEngine,
        options: &ShortFictionRunOptions,
        language: Language,
        chapter_count: u32,
        chars_per_chapter: u32,
        provided_story_id: Option<&str>,
    ) -> Result<ShortFictionRunResult, AppError> {
        // ── 1. 大纲阶段（断点续跑检查）──
        let resumed_outline = provided_story_id
            .and_then(|sid| {
                std::fs::read_to_string(self.story_dir(sid).join("outline").join("v002.md")).ok()
            })
            .filter(|s| !s.trim().is_empty());

        let (story_id, outline_markdown) = if let (Some(sid), Some(outline)) =
            (provided_story_id, resumed_outline)
        {
            tracing::info!("[short-fiction] 从已有大纲续跑 {}", sid);
            (sid.to_string(), outline)
        } else {
            // 1a. create_outline
            tracing::info!("[short-fiction] 生成短篇大纲");
            let outline_v1 = short_fiction::create_outline(
                engine,
                &short_fiction::ShortFictionOutlineInput {
                    direction: options.direction.clone(),
                    chapter_count,
                    chars_per_chapter,
                    reference: options.reference.clone(),
                    language,
                },
            )
            .await?;

            let sid = provided_story_id
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    let seed = if outline_v1.story_title.trim().is_empty() {
                        options.direction.as_str()
                    } else {
                        outline_v1.story_title.as_str()
                    };
                    safe_segment(&slugify(seed, "short"), "short")
                });
            let base_dir = self.story_dir(&sid);

            write_text(&base_dir.join("outline").join("v001.md"), &outline_v1.raw_content)?;

            // 1b. review_outline
            tracing::info!("[short-fiction] 审核大纲");
            let outline_review = short_fiction::review_outline(
                engine,
                &short_fiction::ShortFictionOutlineReviewInput {
                    direction: options.direction.clone(),
                    outline: outline_v1.clone(),
                    reference: options.reference.clone(),
                    language,
                },
            )
            .await?;
            write_text(&base_dir.join("reviews").join("outline-v001.md"), &outline_review)?;

            // 1c. revise_outline
            tracing::info!("[short-fiction] 修订大纲");
            let outline_v2 = short_fiction::revise_outline(
                engine,
                &short_fiction::ShortFictionOutlineRevisionInput {
                    direction: options.direction.clone(),
                    outline: outline_v1,
                    review: outline_review,
                    reference: options.reference.clone(),
                    chapter_count,
                    chars_per_chapter,
                    language,
                },
            )
            .await?;
            write_text(&base_dir.join("outline").join("v002.md"), &outline_v2.raw_content)?;

            (sid, outline_v2.raw_content)
        };

        let base_dir = self.story_dir(&story_id);

        // ── 2. 草稿阶段 ──
        tracing::info!("[short-fiction] 撰写整篇草稿");
        let mut draft_v1 = short_fiction::write_draft(
            engine,
            &short_fiction::ShortFictionDraftInput {
                direction: options.direction.clone(),
                outline_markdown: outline_markdown.clone(),
                chapter_count,
                chars_per_chapter,
                language,
            },
        )
        .await?;

        // 漏章补写（最多 SHORT_FICTION_DRAFT_COMPLETION_ATTEMPTS 次）
        let mut missing = short_fiction::find_empty_chapters(&draft_v1);
        if !missing.is_empty() {
            write_draft_artifacts(&base_dir, "v001-partial", &draft_v1, language)?;
            for attempt in 1..=SHORT_FICTION_DRAFT_COMPLETION_ATTEMPTS {
                if missing.is_empty() {
                    break;
                }
                tracing::info!(
                    "[short-fiction] 补写缺失章节（第{}轮）: {:?}",
                    attempt,
                    missing
                );
                draft_v1 = short_fiction::continue_draft(
                    engine,
                    &short_fiction::ShortFictionDraftContinuationInput {
                        direction: options.direction.clone(),
                        outline_markdown: outline_markdown.clone(),
                        chapter_count,
                        chars_per_chapter,
                        draft: draft_v1,
                        language,
                    },
                )
                .await?;
                missing = short_fiction::find_empty_chapters(&draft_v1);
                if !missing.is_empty() {
                    write_draft_artifacts(&base_dir, "v001-partial", &draft_v1, language)?;
                }
            }
        }

        short_fiction::validate_draft_for_final(&draft_v1, Some(chapter_count))?;
        write_draft_artifacts(&base_dir, "v001", &draft_v1, language)?;

        // ── 3. 草稿审核 ──
        tracing::info!("[short-fiction] 审核整篇草稿");
        let draft_review = short_fiction::review_draft(
            engine,
            &short_fiction::ShortFictionDraftReviewInput {
                direction: options.direction.clone(),
                outline_markdown: outline_markdown.clone(),
                chapter_count,
                chars_per_chapter,
                draft: draft_v1.clone(),
                language,
            },
        )
        .await?;
        write_text(
            &base_dir.join("reviews").join("draft-v001.md"),
            &draft_review,
        )?;

        // ── 4. 草稿修订（失败保留 v1）──
        let mut final_draft = draft_v1.clone();
        let mut revision_warning: Option<String> = None;

        tracing::info!("[short-fiction] 修订整篇草稿");
        match short_fiction::revise_draft(
            engine,
            &short_fiction::ShortFictionDraftRevisionInput {
                direction: options.direction.clone(),
                outline_markdown: outline_markdown.clone(),
                chapter_count,
                chars_per_chapter,
                draft: draft_v1.clone(),
                review: draft_review,
                language,
            },
        )
        .await
        {
            Ok(draft_v2) => {
                match short_fiction::validate_draft_for_final(&draft_v2, Some(chapter_count)) {
                    Ok(()) => {
                        write_draft_artifacts(&base_dir, "v002", &draft_v2, language)?;
                        final_draft = draft_v2;
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        tracing::warn!("[short-fiction] 第二轮改稿校验失败，保留 v1: {}", msg);
                        revision_warning = Some(msg.clone());
                        write_revision_warning(&base_dir, &msg, language)?;
                    }
                }
            }
            Err(e) => {
                let msg = e.to_string();
                tracing::warn!("[short-fiction] 第二轮改稿失败，保留 v1: {}", msg);
                revision_warning = Some(msg.clone());
                write_revision_warning(&base_dir, &msg, language)?;
            }
        }

        // ── 5. 落盘最终产物 ──
        write_final_artifacts(&base_dir, &final_draft, language)?;

        // ── 6. 生成销售包装 ──
        tracing::info!("[short-fiction] 生成销售包装");
        let sales_package = short_fiction::generate_package(
            engine,
            &short_fiction::ShortFictionPackageInput {
                direction: options.direction.clone(),
                outline_markdown: outline_markdown.clone(),
                draft: final_draft,
                language,
            },
        )
        .await?;
        write_package_artifacts(&base_dir, &sales_package, language)?;

        // ── 7. 写完成状态（仅有 warning 时）──
        if let Some(w) = &revision_warning {
            let _ = self.write_run_status(
                &story_id,
                "complete",
                Some(&format!("revision skipped: {}", w)),
                None,
            );
        }

        Ok(self.build_run_result(&story_id))
    }

    // ── 辅助方法 ─────────────────────────────────────────────

    /// 读取 status.json，判断是否为 failed 状态。
    fn is_failed_run(&self, story_id: &str) -> Result<bool, AppError> {
        let path = self.story_dir(story_id).join("status.json");
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return Ok(false),
        };
        let parsed: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => return Ok(false),
        };
        Ok(parsed.get("status").and_then(|v| v.as_str()) == Some("failed"))
    }

    /// 写 status.json。
    fn write_run_status(
        &self,
        story_id: &str,
        status: &str,
        warning: Option<&str>,
        error: Option<&str>,
    ) -> Result<(), AppError> {
        let status_obj = RunStatus {
            status: status.to_string(),
            warning: warning.map(|s| s.to_string()),
            error: error.map(|s| s.to_string()),
            updated_at: current_iso(),
        };
        let path = self.story_dir(story_id).join("status.json");
        write_json(&path, &status_obj)
    }

    /// 构建运行结果。
    fn build_run_result(&self, story_id: &str) -> ShortFictionRunResult {
        let base = self.story_dir(story_id);
        ShortFictionRunResult {
            story_id: story_id.to_string(),
            outline_path: base.join("outline").join("v002.md"),
            outline_review_path: base.join("reviews").join("outline-v001.md"),
            draft_review_path: base.join("reviews").join("draft-v001.md"),
            final_markdown_path: base.join("final").join("full.md"),
            final_json_path: base.join("final").join("short-story.json"),
            sales_package_path: base.join("final").join("sales-package.md"),
            cover_prompt_path: base.join("final").join("cover-prompt.md"),
        }
    }
}

// ── 运行状态结构 ─────────────────────────────────────────────

/// status.json 结构
#[derive(serde::Serialize)]
struct RunStatus {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    warning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(rename = "updatedAt")]
    updated_at: String,
}

// ── 工具函数（模块级）───────────────────────────────────────

/// 当前 ISO 时间戳（UTC）
fn current_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// 参数边界校验。
fn bounded_integer(
    value: Option<u32>,
    fallback: u32,
    name: &str,
    min: u32,
    max: u32,
) -> Result<u32, AppError> {
    let parsed = value.unwrap_or(fallback);
    if parsed < min || parsed > max {
        return Err(AppError::invalid_input(format!(
            "{} must be an integer between {} and {}.",
            name, min, max
        )));
    }
    Ok(parsed)
}

/// 文件名安全化。
/// 危险字符 → _，空白序列 → 单个空格，trim，截断 80 字符。
fn safe_file_name(value: &str) -> String {
    let after_dangerous: String = value
        .chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '\0' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect();
    static WS_PLUS_FNAME: OnceLock<regex::Regex> = OnceLock::new();
    let after_ws = WS_PLUS_FNAME
        .get_or_init(|| regex::Regex::new(r"\s+").expect("valid ws+ regex"))
        .replace_all(&after_dangerous, " ")
        .to_string();
    let trimmed = after_ws.trim();
    let truncated: String = trimmed.chars().take(80).collect();
    if truncated.is_empty() {
        "short-fiction".to_string()
    } else {
        truncated
    }
}

/// 写文本文件（自动创建父目录，末尾加换行）。
fn write_text(path: &Path, content: &str) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, format!("{}\n", content.trim_end()))?;
    Ok(())
}

/// 写 JSON 文件（pretty-print）。
fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), AppError> {
    let content = serde_json::to_string_pretty(value)?;
    write_text(path, &content)
}

/// 写草稿产物。
/// 落盘 full.md + draft.json + chapters/0001.md ...
fn write_draft_artifacts(
    base_dir: &Path,
    version: &str,
    draft: &ShortFictionBatchDraft,
    language: Language,
) -> Result<(), AppError> {
    let draft_dir = base_dir.join("drafts").join(version);
    write_text(
        &draft_dir.join("full.md"),
        &short_fiction::render_draft_markdown(draft, language),
    )?;
    write_json(&draft_dir.join("draft.json"), draft)?;
    let chapters_dir = draft_dir.join("chapters");
    for chapter in &draft.chapters {
        let filename = format!("{:04}.md", chapter.number);
        let heading =
            short_fiction::format_chapter_heading(chapter.number, &chapter.title, language);
        let content = format!("# {}\n\n{}", heading, chapter.content);
        write_text(&chapters_dir.join(&filename), &content)?;
    }
    Ok(())
}

/// 写最终产物。
/// 落盘 final/full.md + final/{title}.md + final/short-story.json + final/chapters/*.md
fn write_final_artifacts(
    base_dir: &Path,
    draft: &ShortFictionBatchDraft,
    language: Language,
) -> Result<(), AppError> {
    let final_dir = base_dir.join("final");
    let markdown = short_fiction::render_draft_markdown(draft, language);
    write_text(&final_dir.join("full.md"), &markdown)?;
    let title_file = safe_file_name(&draft.story_title);
    write_text(&final_dir.join(format!("{}.md", title_file)), &markdown)?;
    write_json(&final_dir.join("short-story.json"), draft)?;
    let chapters_dir = final_dir.join("chapters");
    for chapter in &draft.chapters {
        let filename = format!("{:04}.md", chapter.number);
        let heading =
            short_fiction::format_chapter_heading(chapter.number, &chapter.title, language);
        let content = format!("# {}\n\n{}", heading, chapter.content);
        write_text(&chapters_dir.join(&filename), &content)?;
    }
    Ok(())
}

/// 写销售包装产物。
/// 落盘 final/sales-package.json + final/sales-package.md + final/cover-prompt.md
fn write_package_artifacts(
    base_dir: &Path,
    sales_package: &ShortFictionSalesPackage,
    language: Language,
) -> Result<(), AppError> {
    let final_dir = base_dir.join("final");
    write_json(&final_dir.join("sales-package.json"), sales_package)?;

    let headings = match language {
        Language::En => ("## Synopsis", "## Selling Points", "## Cover Prompt"),
        Language::Zh => ("## 简介", "## 卖点", "## 封面提示词"),
    };
    let mut md: Vec<String> = vec![
        format!("# {}", sales_package.title),
        String::new(),
        headings.0.to_string(),
        String::new(),
        sales_package.intro.clone(),
        String::new(),
        headings.1.to_string(),
        String::new(),
    ];
    for point in &sales_package.selling_points {
        md.push(format!("- {}", point));
    }
    md.push(String::new());
    md.push(headings.2.to_string());
    md.push(String::new());
    md.push(sales_package.cover_prompt.clone());
    write_text(&final_dir.join("sales-package.md"), &md.join("\n"))?;

    let cover_prompt = if sales_package.cover_prompt.trim().is_empty() {
        "(empty)".to_string()
    } else {
        sales_package.cover_prompt.clone()
    };
    write_text(&final_dir.join("cover-prompt.md"), &cover_prompt)?;
    Ok(())
}

/// 写修订失败警告。
/// 落盘 reviews/draft-v002-warning.md
fn write_revision_warning(
    base_dir: &Path,
    warning: &str,
    language: Language,
) -> Result<(), AppError> {
    let content = match language {
        Language::En => format!(
            "# Second revision not adopted\n\nThe system refused to overwrite the complete first draft with an incomplete or unparsable revision.\n\n## Reason\n\n{}",
            warning
        ),
        Language::Zh => format!(
            "# 第二轮改稿未采用\n\n系统没有用不完整或解析失败的改稿覆盖完整首稿。\n\n## 原因\n\n{}",
            warning
        ),
    };
    write_text(
        &base_dir.join("reviews").join("draft-v002-warning.md"),
        &content,
    )
}

// ── 测试 ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::pipeline::agents::short_fiction::{
        ShortFictionBatchDraft, ShortFictionChapter, ShortFictionSalesPackage,
    };
    use tempfile::TempDir;

    // ── safe_file_name ──

    #[test]
    fn safe_file_name_replaces_dangerous_chars() {
        assert_eq!(safe_file_name("a/b:c"), "a_b_c");
    }

    #[test]
    fn safe_file_name_collapses_whitespace() {
        assert_eq!(safe_file_name("hello   world"), "hello world");
    }

    #[test]
    fn safe_file_name_empty_returns_fallback() {
        assert_eq!(safe_file_name(""), "short-fiction");
    }

    // ── bounded_integer ──

    #[test]
    fn bounded_integer_uses_fallback_when_none() {
        assert_eq!(bounded_integer(None, 12, "x", 1, 20).unwrap(), 12);
    }

    #[test]
    fn bounded_integer_uses_value_when_some() {
        assert_eq!(bounded_integer(Some(15), 12, "x", 1, 20).unwrap(), 15);
    }

    #[test]
    fn bounded_integer_rejects_below_min() {
        assert!(bounded_integer(Some(0), 12, "x", 1, 20).is_err());
    }

    #[test]
    fn bounded_integer_rejects_above_max() {
        assert!(bounded_integer(Some(25), 12, "x", 1, 20).is_err());
    }

    // ── write_text / write_json ──

    #[test]
    fn write_text_creates_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("a").join("b").join("c.md");
        write_text(&path, "hello").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello\n");
    }

    #[test]
    fn write_text_trims_trailing_whitespace() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.md");
        write_text(&path, "hello   \n\n").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello\n");
    }

    #[test]
    fn write_json_serializes_pretty() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("data.json");
        let value = serde_json::json!({"key": "value", "num": 42});
        write_json(&path, &value).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"key\""));
        assert!(content.contains("\"value\""));
        assert!(content.contains("\"num\""));
        assert!(content.contains("42"));
        // pretty-print 包含换行
        assert!(content.contains('\n'));
    }

    // ── build_run_result ──

    #[test]
    fn build_run_result_constructs_paths() {
        let tmp = TempDir::new().unwrap();
        let config = ShortFictionConfig {
            books_dir: tmp.path().to_path_buf(),
            ..Default::default()
        };
        let runner = ShortFictionRunner::new(config);
        let result = runner.build_run_result("my-story");
        assert!(result
            .outline_path
            .ends_with("short-fiction/my-story/outline/v002.md"));
        assert!(result
            .outline_review_path
            .ends_with("short-fiction/my-story/reviews/outline-v001.md"));
        assert!(result
            .draft_review_path
            .ends_with("short-fiction/my-story/reviews/draft-v001.md"));
        assert!(result
            .final_markdown_path
            .ends_with("short-fiction/my-story/final/full.md"));
        assert!(result
            .final_json_path
            .ends_with("short-fiction/my-story/final/short-story.json"));
        assert!(result
            .sales_package_path
            .ends_with("short-fiction/my-story/final/sales-package.md"));
        assert!(result
            .cover_prompt_path
            .ends_with("short-fiction/my-story/final/cover-prompt.md"));
    }

    // ── is_failed_run / write_run_status ──

    #[test]
    fn is_failed_run_reads_status_json() {
        let tmp = TempDir::new().unwrap();
        let config = ShortFictionConfig {
            books_dir: tmp.path().to_path_buf(),
            ..Default::default()
        };
        let runner = ShortFictionRunner::new(config);

        // 写 status=failed
        runner.write_run_status("test-story", "failed", None, Some("boom")).unwrap();
        assert!(runner.is_failed_run("test-story").unwrap());

        // 写 status=complete
        runner.write_run_status("test-story", "complete", None, None).unwrap();
        assert!(!runner.is_failed_run("test-story").unwrap());
    }

    #[test]
    fn is_failed_run_returns_false_when_missing() {
        let tmp = TempDir::new().unwrap();
        let config = ShortFictionConfig {
            books_dir: tmp.path().to_path_buf(),
            ..Default::default()
        };
        let runner = ShortFictionRunner::new(config);
        assert!(!runner.is_failed_run("no-such-story").unwrap());
    }

    #[test]
    fn is_failed_run_returns_false_on_invalid_json() {
        let tmp = TempDir::new().unwrap();
        let config = ShortFictionConfig {
            books_dir: tmp.path().to_path_buf(),
            ..Default::default()
        };
        let runner = ShortFictionRunner::new(config);
        let story_dir = runner.story_dir("bad-story");
        std::fs::create_dir_all(&story_dir).unwrap();
        std::fs::write(story_dir.join("status.json"), "not json").unwrap();
        assert!(!runner.is_failed_run("bad-story").unwrap());
    }

    #[test]
    fn write_run_status_includes_updated_at() {
        let tmp = TempDir::new().unwrap();
        let config = ShortFictionConfig {
            books_dir: tmp.path().to_path_buf(),
            ..Default::default()
        };
        let runner = ShortFictionRunner::new(config);
        runner
            .write_run_status("s1", "complete", Some("revision skipped: test"), None)
            .unwrap();
        let raw = std::fs::read_to_string(runner.story_dir("s1").join("status.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed.get("status").and_then(|v| v.as_str()), Some("complete"));
        assert!(parsed.get("updatedAt").is_some());
        assert_eq!(
            parsed.get("warning").and_then(|v| v.as_str()),
            Some("revision skipped: test")
        );
        // error 字段应被 skip
        assert!(parsed.get("error").is_none());
    }

    // ── write_draft_artifacts ──

    #[test]
    fn write_draft_artifacts_creates_structure() {
        let tmp = TempDir::new().unwrap();
        let draft = ShortFictionBatchDraft {
            story_title: "测试".to_string(),
            opening_hook: None,
            chapters: vec![ShortFictionChapter {
                number: 1,
                title: "第一章".to_string(),
                content: "正文内容".to_string(),
                char_count: 4,
            }],
            raw_content: String::new(),
        };
        write_draft_artifacts(tmp.path(), "v001", &draft, Language::Zh).unwrap();

        let draft_dir = tmp.path().join("drafts").join("v001");
        assert!(draft_dir.join("full.md").exists());
        assert!(draft_dir.join("draft.json").exists());
        assert!(draft_dir.join("chapters").join("0001.md").exists());

        let chapter =
            std::fs::read_to_string(draft_dir.join("chapters").join("0001.md")).unwrap();
        assert!(chapter.contains("第1章"));
        assert!(chapter.contains("正文内容"));

        let full_md = std::fs::read_to_string(draft_dir.join("full.md")).unwrap();
        assert!(full_md.contains("# 测试"));
    }

    #[test]
    fn write_draft_artifacts_writes_multiple_chapters() {
        let tmp = TempDir::new().unwrap();
        let draft = ShortFictionBatchDraft {
            story_title: "多章测试".to_string(),
            opening_hook: Some("钩子".to_string()),
            chapters: vec![
                ShortFictionChapter {
                    number: 1,
                    title: "起".to_string(),
                    content: "内容一".to_string(),
                    char_count: 3,
                },
                ShortFictionChapter {
                    number: 2,
                    title: "承".to_string(),
                    content: "内容二".to_string(),
                    char_count: 3,
                },
            ],
            raw_content: String::new(),
        };
        write_draft_artifacts(tmp.path(), "v002", &draft, Language::Zh).unwrap();

        let chapters_dir = tmp.path().join("drafts").join("v002").join("chapters");
        assert!(chapters_dir.join("0001.md").exists());
        assert!(chapters_dir.join("0002.md").exists());
    }

    // ── write_final_artifacts ──

    #[test]
    fn write_final_artifacts_creates_all_files() {
        let tmp = TempDir::new().unwrap();
        let draft = ShortFictionBatchDraft {
            story_title: "最终测试".to_string(),
            opening_hook: None,
            chapters: vec![ShortFictionChapter {
                number: 1,
                title: "终章".to_string(),
                content: "最终内容".to_string(),
                char_count: 4,
            }],
            raw_content: String::new(),
        };
        write_final_artifacts(tmp.path(), &draft, Language::Zh).unwrap();

        let final_dir = tmp.path().join("final");
        assert!(final_dir.join("full.md").exists());
        assert!(final_dir.join("最终测试.md").exists());
        assert!(final_dir.join("short-story.json").exists());
        assert!(final_dir.join("chapters").join("0001.md").exists());

        let json =
            std::fs::read_to_string(final_dir.join("short-story.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            parsed.get("story_title").and_then(|v| v.as_str()),
            Some("最终测试")
        );
    }

    // ── write_package_artifacts ──

    #[test]
    fn write_package_artifacts_writes_all_files_zh() {
        let tmp = TempDir::new().unwrap();
        let pkg = ShortFictionSalesPackage {
            title: "测试标题".to_string(),
            intro: "简介内容".to_string(),
            selling_points: vec!["卖点1".to_string(), "卖点2".to_string()],
            cover_prompt: "封面提示".to_string(),
            raw_content: String::new(),
        };
        write_package_artifacts(tmp.path(), &pkg, Language::Zh).unwrap();

        let final_dir = tmp.path().join("final");
        assert!(final_dir.join("sales-package.json").exists());
        assert!(final_dir.join("sales-package.md").exists());
        assert!(final_dir.join("cover-prompt.md").exists());

        let md = std::fs::read_to_string(final_dir.join("sales-package.md")).unwrap();
        assert!(md.contains("# 测试标题"));
        assert!(md.contains("## 简介"));
        assert!(md.contains("简介内容"));
        assert!(md.contains("## 卖点"));
        assert!(md.contains("- 卖点1"));
        assert!(md.contains("- 卖点2"));
        assert!(md.contains("## 封面提示词"));
        assert!(md.contains("封面提示"));

        let cover = std::fs::read_to_string(final_dir.join("cover-prompt.md")).unwrap();
        assert_eq!(cover, "封面提示\n");
    }

    #[test]
    fn write_package_artifacts_writes_all_files_en() {
        let tmp = TempDir::new().unwrap();
        let pkg = ShortFictionSalesPackage {
            title: "Test Title".to_string(),
            intro: "Synopsis text".to_string(),
            selling_points: vec!["Point 1".to_string()],
            cover_prompt: "Cover prompt text".to_string(),
            raw_content: String::new(),
        };
        write_package_artifacts(tmp.path(), &pkg, Language::En).unwrap();

        let md = std::fs::read_to_string(
            tmp.path().join("final").join("sales-package.md"),
        )
        .unwrap();
        assert!(md.contains("# Test Title"));
        assert!(md.contains("## Synopsis"));
        assert!(md.contains("## Selling Points"));
        assert!(md.contains("- Point 1"));
        assert!(md.contains("## Cover Prompt"));
    }

    #[test]
    fn write_package_artifacts_empty_cover_prompt_uses_placeholder() {
        let tmp = TempDir::new().unwrap();
        let pkg = ShortFictionSalesPackage {
            title: "测试".to_string(),
            intro: String::new(),
            selling_points: vec![],
            cover_prompt: "   ".to_string(),
            raw_content: String::new(),
        };
        write_package_artifacts(tmp.path(), &pkg, Language::Zh).unwrap();

        let cover =
            std::fs::read_to_string(tmp.path().join("final").join("cover-prompt.md")).unwrap();
        assert_eq!(cover, "(empty)\n");
    }

    // ── write_revision_warning ──

    #[test]
    fn write_revision_warning_writes_zh() {
        let tmp = TempDir::new().unwrap();
        write_revision_warning(tmp.path(), "测试原因", Language::Zh).unwrap();
        let content =
            std::fs::read_to_string(tmp.path().join("reviews").join("draft-v002-warning.md"))
                .unwrap();
        assert!(content.contains("第二轮改稿未采用"));
        assert!(content.contains("测试原因"));
    }

    #[test]
    fn write_revision_warning_writes_en() {
        let tmp = TempDir::new().unwrap();
        write_revision_warning(tmp.path(), "test reason", Language::En).unwrap();
        let content =
            std::fs::read_to_string(tmp.path().join("reviews").join("draft-v002-warning.md"))
                .unwrap();
        assert!(content.contains("Second revision not adopted"));
        assert!(content.contains("test reason"));
    }

    // ── current_iso ──

    #[test]
    fn current_iso_returns_valid_iso() {
        let iso = current_iso();
        // RFC3339 格式包含 'T' 分隔符
        assert!(iso.contains('T'), "ISO 时间戳应包含 T: {}", iso);
    }
}
