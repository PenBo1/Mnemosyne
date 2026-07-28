//! ═══════════════════════════════════════════════════════════════════════════
//! Pipeline IPC 命令 - 前端交互接口
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 命令列表（前端使用 camelCase 调用）：
//! - pipeline_init_book: 初始化书籍（生成基础设定 + 落盘）
//! - pipeline_revise_foundation: 修订已有书籍的基础设定
//! - pipeline_plan_chapter: 为下一章生成 chapter memo
//! - pipeline_compose_chapter: 组装章节上下文
//! - pipeline_write_draft: 写一章草稿
//! - pipeline_audit_draft: 审计指定章节
//! - pipeline_revise_draft: 修订指定章节
//! - pipeline_write_next_chapter: 写下一章完整流程（8-agent cycle）
//! - pipeline_list_chapters: 列出书籍章节索引
//! - pipeline_get_chapter: 读取章节正文
//! - pipeline_consolidate: 压缩旧卷摘要

use std::path::PathBuf;
use std::time::Instant;

use crate::core::agent::commands::AgentState;
use crate::domain::pipeline::agents::reviser::ReviseMode;
use crate::domain::pipeline::runner::{PipelineConfig, PipelineRunner};
use crate::domain::pipeline::state::manager::StateManager;
use crate::domain::pipeline::types::{BookConfig, ChapterMeta, ChapterStatus};
use crate::domain::pipeline::utils::text_parse::count_zh_chars;
use crate::infrastructure::fs::data_dir::DataDir;
use crate::infrastructure::validation::validate_id;
use crate::shared::error::{AppError, IpcResponse};
use tauri::State;

// ── 校验工具 ─────────────────────────────────────────────────

fn validate_book_id(book_id: &str) -> Result<(), AppError> {
    validate_id(book_id, "book_id").map_err(AppError::invalid_input)
}

/// 从字符串解析修订模式（前端传 camelCase 字符串）
fn parse_revise_mode(s: &str) -> Result<ReviseMode, AppError> {
    match s.to_lowercase().as_str() {
        "auto" | "" => Ok(ReviseMode::Auto),
        "polish" => Ok(ReviseMode::Polish),
        "rewrite" => Ok(ReviseMode::Rewrite),
        "rework" => Ok(ReviseMode::Rework),
        "antidetect" | "anti-detect" => Ok(ReviseMode::AntiDetect),
        "spotfix" | "spot-fix" => Ok(ReviseMode::SpotFix),
        other => Err(AppError::invalid_input(format!(
            "未知的修订模式: {}（支持: auto/polish/rewrite/rework/antidetect/spotfix）",
            other
        ))),
    }
}

/// 构造 PipelineRunner（从 DataDir 读取 books_dir）
fn build_runner(data_dir: &DataDir) -> PipelineRunner {
    let config = PipelineConfig {
        books_dir: data_dir.books_dir(),
        ..Default::default()
    };
    PipelineRunner::new(config)
}

// ── 初始化书籍 ───────────────────────────────────────────────

/// 初始化书籍请求体（前端使用 camelCase）
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitBookRequest {
    pub book: BookConfig,
    pub genre_name: String,
    pub genre_body: String,
    #[serde(default)]
    pub external_context: Option<String>,
    #[serde(default)]
    pub author_intent: Option<String>,
}

/// 初始化书籍：生成基础设定 + 落盘 + 初始化控制文档。
#[tauri::command]
pub async fn pipeline_init_book(
    agent_state: State<'_, AgentState>,
    data_dir: State<'_, DataDir>,
    request: InitBookRequest,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(book_id = %request.book.id, "pipeline_init_book: enter");
    
    validate_book_id(&request.book.id)?;
    tracing::debug!(book_id = %request.book.id, genre = %request.genre_name, "Book ID validated");

    let runner = build_runner(&data_dir);
    runner
        .init_book(
            &agent_state.engine,
            &request.book,
            &request.genre_name,
            &request.genre_body,
            request.external_context.as_deref(),
            request.author_intent.as_deref(),
        )
        .await?;

    tracing::info!(
        book_id = %request.book.id,
        duration_ms = start.elapsed().as_millis(),
        "pipeline_init_book: exit"
    );
    Ok(IpcResponse::created(true))
}

// ── 修订基础设定 ─────────────────────────────────────────────

/// 修订已有书籍的基础设定（不动 runtime chapter state）。
#[tauri::command]
pub async fn pipeline_revise_foundation(
    agent_state: State<'_, AgentState>,
    data_dir: State<'_, DataDir>,
    book_id: String,
    feedback: String,
    genre_name: String,
    genre_body: String,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(book_id = %book_id, "pipeline_revise_foundation: enter");
    
    validate_book_id(&book_id)?;
    tracing::debug!(book_id = %book_id, "Book ID validated");

    let runner = build_runner(&data_dir);
    runner
        .revise_foundation(
            &agent_state.engine,
            &book_id,
            &feedback,
            &genre_name,
            &genre_body,
        )
        .await?;

    tracing::info!(
        book_id = %book_id,
        duration_ms = start.elapsed().as_millis(),
        "pipeline_revise_foundation: exit"
    );
    Ok(IpcResponse::ok(true))
}

// ── 规划章节 ─────────────────────────────────────────────────

/// 为下一章生成 chapter memo。
#[tauri::command]
pub async fn pipeline_plan_chapter(
    agent_state: State<'_, AgentState>,
    data_dir: State<'_, DataDir>,
    book_id: String,
) -> Result<IpcResponse<crate::domain::pipeline::runner::PlanChapterResult>, AppError> {
    let start = Instant::now();
    tracing::info!(book_id = %book_id, "pipeline_plan_chapter: enter");
    
    validate_book_id(&book_id)?;
    tracing::debug!(book_id = %book_id, "Book ID validated");

    let runner = build_runner(&data_dir);
    let result = runner.plan_chapter(&agent_state.engine, &book_id).await?;

    tracing::info!(
        book_id = %book_id,
        chapter = result.chapter_number,
        duration_ms = start.elapsed().as_millis(),
        "pipeline_plan_chapter: exit"
    );
    Ok(IpcResponse::ok(result))
}

// ── 组装上下文 ───────────────────────────────────────────────

/// 组装章节上下文（context package + rule stack）。
#[tauri::command]
pub async fn pipeline_compose_chapter(
    agent_state: State<'_, AgentState>,
    data_dir: State<'_, DataDir>,
    book_id: String,
) -> Result<IpcResponse<crate::domain::pipeline::runner::ComposeChapterResult>, AppError> {
    let start = Instant::now();
    tracing::info!(book_id = %book_id, "pipeline_compose_chapter: enter");
    
    validate_book_id(&book_id)?;
    tracing::debug!(book_id = %book_id, "Book ID validated");

    let runner = build_runner(&data_dir);
    let result = runner
        .compose_chapter(&agent_state.engine, &book_id)
        .await?;

    tracing::info!(
        book_id = %book_id,
        chapter = result.chapter_number,
        duration_ms = start.elapsed().as_millis(),
        "pipeline_compose_chapter: exit"
    );
    Ok(IpcResponse::ok(result))
}

// ── 写草稿 ───────────────────────────────────────────────────

/// 写一章草稿（不含 audit/revise）。
#[tauri::command]
pub async fn pipeline_write_draft(
    agent_state: State<'_, AgentState>,
    data_dir: State<'_, DataDir>,
    book_id: String,
    word_count_override: Option<u32>,
) -> Result<IpcResponse<crate::domain::pipeline::runner::pipeline_runner::DraftResult>, AppError> {
    let start = Instant::now();
    tracing::info!(book_id = %book_id, word_count_override = ?word_count_override, "pipeline_write_draft: enter");
    
    validate_book_id(&book_id)?;
    tracing::debug!(book_id = %book_id, "Book ID validated");

    let runner = build_runner(&data_dir);
    let result = runner
        .write_draft(&agent_state.engine, &book_id, word_count_override)
        .await?;

    tracing::info!(
        book_id = %book_id,
        chapter = result.chapter_number,
        words = result.word_count,
        duration_ms = start.elapsed().as_millis(),
        "pipeline_write_draft: exit"
    );
    Ok(IpcResponse::ok(result))
}

// ── 审计草稿 ─────────────────────────────────────────────────

/// 审计指定章节（或最新章）。
#[tauri::command]
pub async fn pipeline_audit_draft(
    agent_state: State<'_, AgentState>,
    data_dir: State<'_, DataDir>,
    book_id: String,
    chapter_number: Option<u32>,
) -> Result<IpcResponse<crate::domain::pipeline::agents::continuity::AuditResult>, AppError> {
    let start = Instant::now();
    tracing::info!(book_id = %book_id, chapter = ?chapter_number, "pipeline_audit_draft: enter");
    
    validate_book_id(&book_id)?;
    tracing::debug!(book_id = %book_id, "Book ID validated");

    let runner = build_runner(&data_dir);
    let result = runner
        .audit_draft(&agent_state.engine, &book_id, chapter_number)
        .await?;

    tracing::info!(
        book_id = %book_id,
        passed = result.passed,
        score = ?result.overall_score,
        issues = result.issues.len(),
        duration_ms = start.elapsed().as_millis(),
        "pipeline_audit_draft: exit"
    );
    Ok(IpcResponse::ok(result))
}

// ── 修订草稿 ─────────────────────────────────────────────────

/// 修订指定章节（或最新章）。
#[tauri::command]
pub async fn pipeline_revise_draft(
    agent_state: State<'_, AgentState>,
    data_dir: State<'_, DataDir>,
    book_id: String,
    chapter_number: Option<u32>,
    mode: Option<String>,
) -> Result<IpcResponse<crate::domain::pipeline::runner::ReviseResult>, AppError> {
    validate_book_id(&book_id)?;

    let mode = parse_revise_mode(mode.as_deref().unwrap_or("auto"))?;

    let runner = build_runner(&data_dir);
    let result = runner
        .revise_draft(&agent_state.engine, &book_id, chapter_number, mode)
        .await?;

    tracing::info!(
        book_id = %book_id,
        chapter = result.chapter_number,
        applied = result.applied,
        status = %result.status,
        "Pipeline: 章节修订完成"
    );
    Ok(IpcResponse::ok(result))
}

// ── 写下一章完整流程 ─────────────────────────────────────────

/// 写下一章完整流程：plan → compose → write → review cycle → truth validation → persist。
#[tauri::command]
pub async fn pipeline_write_next_chapter(
    agent_state: State<'_, AgentState>,
    data_dir: State<'_, DataDir>,
    book_id: String,
    word_count_override: Option<u32>,
) -> Result<IpcResponse<crate::domain::pipeline::runner::ChapterPipelineResult>, AppError> {
    validate_book_id(&book_id)?;

    let runner = build_runner(&data_dir);
    let result = runner
        .write_next_chapter(&agent_state.engine, &book_id, word_count_override)
        .await?;

    tracing::info!(
        book_id = %book_id,
        chapter = result.chapter_number,
        words = result.word_count,
        revised = result.revised,
        status = %result.status,
        "Pipeline: 章节完整流程完成"
    );
    Ok(IpcResponse::ok(result))
}

// ── 章节索引查询 ─────────────────────────────────────────────

/// 列出书籍章节索引。
#[tauri::command]
pub async fn pipeline_list_chapters(
    data_dir: State<'_, DataDir>,
    book_id: String,
) -> Result<IpcResponse<Vec<ChapterMeta>>, AppError> {
    validate_book_id(&book_id)?;

    let book_dir = data_dir.books_dir().join(&book_id);
    let index_path = book_dir.join("chapters.json");
    if !index_path.exists() {
        return Ok(IpcResponse::ok(Vec::new()));
    }
    // 文件 I/O 卸载到阻塞线程池，避免阻塞 tokio worker
    let index = tokio::task::spawn_blocking(move || -> Result<Vec<ChapterMeta>, AppError> {
        let content = std::fs::read_to_string(&index_path)
            .map_err(|e| AppError::internal(format!("读取 chapters.json 失败: {}", e)))?;
        serde_json::from_str(&content)
            .map_err(|e| AppError::invalid_input(format!("chapters.json 解析失败: {}", e)))
    })
    .await
    .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;

    Ok(IpcResponse::ok(index))
}

// ── 读取章节正文 ─────────────────────────────────────────────

/// 章节读取结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChapterContent {
    pub chapter_number: u32,
    pub title: String,
    pub content: String,
    pub word_count: u32,
    pub status: ChapterStatus,
}

/// 读取章节正文。
#[tauri::command]
pub async fn pipeline_get_chapter(
    data_dir: State<'_, DataDir>,
    book_id: String,
    chapter_number: u32,
) -> Result<IpcResponse<ChapterContent>, AppError> {
    validate_book_id(&book_id)?;

    let book_dir = data_dir.books_dir().join(&book_id);
    let chapters_dir = book_dir.join("chapters");
    let padded = format!("{:04}", chapter_number);

    // 所有文件 I/O（目录扫描 + 章节读取 + 索引读取）卸载到阻塞线程池
    let chapter = tokio::task::spawn_blocking(move || -> Result<ChapterContent, AppError> {
        let mut chapter_path: Option<PathBuf> = None;
        if chapters_dir.exists() {
            for entry in std::fs::read_dir(&chapters_dir)? {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(&padded) && name.ends_with(".md") {
                    chapter_path = Some(entry.path());
                    break;
                }
            }
        }
        let path = chapter_path.ok_or_else(|| {
            AppError::file_not_found(format!("chapter {} of book {}", chapter_number, book_id))
        })?;
        let raw = std::fs::read_to_string(&path)?;
        let (title, content) = parse_chapter_file(&raw);
        let status = load_chapter_status(&book_dir, chapter_number)?;
        let word_count = count_zh_chars(&content);
        Ok(ChapterContent {
            chapter_number,
            title,
            content,
            word_count,
            status,
        })
    })
    .await
    .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;

    Ok(IpcResponse::ok(chapter))
}

/// 解析章节文件：首行 # 标题，其余为正文
fn parse_chapter_file(raw: &str) -> (String, String) {
    let mut lines = raw.lines();
    let first_line = lines.next().unwrap_or("").trim();
    let title = if let Some(stripped) = first_line.strip_prefix("# ") {
        stripped.to_string()
    } else {
        first_line.to_string()
    };
    let content: String = lines.collect::<Vec<_>>().join("\n");
    (title, content.trim().to_string())
}

/// 从 chapters.json 读取指定章节的状态
///
/// 文件不存在（NotFound）→ 返回 Drafted（新章节默认状态）；
/// 文件存在但解析失败 → 向上传播错误（避免静默掩盖索引损坏）。
fn load_chapter_status(book_dir: &std::path::Path, chapter_number: u32) -> Result<ChapterStatus, AppError> {
    let index_path = book_dir.join("chapters.json");
    let content = match std::fs::read_to_string(&index_path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(ChapterStatus::Drafted),
        Err(e) => return Err(AppError::file_read_error(format!("{}: {}", index_path.display(), e))),
    };
    let index: Vec<ChapterMeta> = serde_json::from_str(&content)
        .map_err(|e| AppError::invalid_format(format!("chapters.json parse: {}", e)))?;
    Ok(index
        .iter()
        .find(|c| c.number == chapter_number)
        .map(|c| c.status)
        .unwrap_or(ChapterStatus::Drafted))
}

// ── 压缩旧卷摘要 ─────────────────────────────────────────────

/// 压缩已完成卷的章节摘要为卷级叙事摘要。
#[tauri::command]
pub async fn pipeline_consolidate(
    agent_state: State<'_, AgentState>,
    data_dir: State<'_, DataDir>,
    book_id: String,
) -> Result<IpcResponse<crate::domain::pipeline::agents::consolidator::ConsolidationResult>, AppError> {
    validate_book_id(&book_id)?;

    let book_dir = data_dir.books_dir().join(&book_id);
    let result =
        crate::domain::pipeline::agents::consolidator::consolidate(&agent_state.engine, &book_dir)
            .await?;

    tracing::info!(
        book_id = %book_id,
        archived_volumes = result.archived_volumes,
        retained_chapters = result.retained_chapters,
        "Pipeline: 卷摘要压缩完成"
    );
    Ok(IpcResponse::ok(result))
}

// ── 书籍列表查询 ─────────────────────────────────────────────

/// 书籍概要（用于列表展示）
#[derive(Debug, Clone, serde::Serialize)]
pub struct BookSummary {
    pub id: String,
    pub title: String,
    pub genre: String,
    pub status: String,
    pub target_chapters: u32,
    pub chapter_word_count: u32,
    pub language: String,
    pub chapter_count: u32,
    pub created_at: String,
    pub updated_at: String,
}

/// 列出所有书籍。
#[tauri::command]
pub async fn pipeline_list_books(
    data_dir: State<'_, DataDir>,
) -> Result<IpcResponse<Vec<BookSummary>>, AppError> {
    let books_dir = data_dir.books_dir();
    if !books_dir.exists() {
        return Ok(IpcResponse::ok(Vec::new()));
    }

    // 整个目录遍历 + 每本书的文件读取都卸载到阻塞线程池
    let mut summaries = tokio::task::spawn_blocking(move || -> Result<Vec<BookSummary>, AppError> {
        let mut summaries = Vec::new();
        for entry in std::fs::read_dir(&books_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let book_id = entry.file_name().to_string_lossy().to_string();
            // 跳过 staging 临时目录
            if book_id.starts_with('.') || book_id.starts_with(".tmp") {
                continue;
            }

            let book_dir = entry.path();
            let config_path = book_dir.join("book.json");
            if !config_path.exists() {
                continue;
            }

            let config_content = match std::fs::read_to_string(&config_path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let book: BookConfig = match serde_json::from_str(&config_content) {
                Ok(b) => b,
                Err(_) => continue,
            };

            // 读取章节计数
            let chapter_count = load_chapter_count(&book_dir);

            summaries.push(BookSummary {
                id: book.id.clone(),
                title: book.title,
                genre: book.genre,
                status: format!("{:?}", book.status).to_lowercase(),
                target_chapters: book.target_chapters,
                chapter_word_count: book.chapter_word_count,
                language: format!("{:?}", book.language.unwrap_or_default()).to_lowercase(),
                chapter_count,
                created_at: book.created_at,
                updated_at: book.updated_at,
            });
        }
        Ok(summaries)
    })
    .await
    .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;

    // 按更新时间倒序
    summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(IpcResponse::ok(summaries))
}

/// 读取书籍的章节数
fn load_chapter_count(book_dir: &std::path::Path) -> u32 {
    let index_path = book_dir.join("chapters.json");
    let content = match std::fs::read_to_string(&index_path) {
        Ok(c) => c,
        Err(_) => return 0,
    };
    let index: Vec<ChapterMeta> = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return 0,
    };
    index.len() as u32
}

// ── 书籍详情查询 ─────────────────────────────────────────────

/// 获取书籍详情（配置 + 章节计数）。
#[tauri::command]
pub async fn pipeline_get_book(
    data_dir: State<'_, DataDir>,
    book_id: String,
) -> Result<IpcResponse<BookConfig>, AppError> {
    validate_book_id(&book_id)?;

    let book_dir = data_dir.books_dir().join(&book_id);
    let book = StateManager::load_book_config(&book_dir)?;
    Ok(IpcResponse::ok(book))
}

// ── 真相文件读取 ─────────────────────────────────────────────

/// 真相文件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruthFileKind {
    CurrentState,
    PendingHooks,
    ChapterSummaries,
    StoryFrame,
    VolumeMap,
    BookRules,
    StyleGuide,
}

impl TruthFileKind {
    /// 返回文件在 book_dir 下的相对路径
    fn relative_path(&self) -> &str {
        match self {
            TruthFileKind::CurrentState => "story/current_state.md",
            TruthFileKind::PendingHooks => "story/pending_hooks.md",
            TruthFileKind::ChapterSummaries => "story/chapter_summaries.md",
            TruthFileKind::StoryFrame => "story/outline/story_frame.md",
            TruthFileKind::VolumeMap => "story/outline/volume_map.md",
            TruthFileKind::BookRules => "story/book_rules.md",
            TruthFileKind::StyleGuide => "story/style_guide.md",
        }
    }

    /// 从字符串解析
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "current_state" | "currentstate" => Some(TruthFileKind::CurrentState),
            "pending_hooks" | "pendinghooks" => Some(TruthFileKind::PendingHooks),
            "chapter_summaries" | "chaptersummaries" => Some(TruthFileKind::ChapterSummaries),
            "story_frame" | "storyframe" => Some(TruthFileKind::StoryFrame),
            "volume_map" | "volumemap" => Some(TruthFileKind::VolumeMap),
            "book_rules" | "bookrules" => Some(TruthFileKind::BookRules),
            "style_guide" | "styleguide" => Some(TruthFileKind::StyleGuide),
            _ => None,
        }
    }
}

/// 读取书籍的真相文件内容。
#[tauri::command]
pub async fn pipeline_read_truth_file(
    data_dir: State<'_, DataDir>,
    book_id: String,
    kind: String,
) -> Result<IpcResponse<String>, AppError> {
    validate_book_id(&book_id)?;

    let file_kind = TruthFileKind::from_str(&kind).ok_or_else(|| {
        AppError::invalid_input(format!(
            "未知的真相文件类型: {}（支持: current_state/pending_hooks/chapter_summaries/story_frame/volume_map/book_rules/style_guide）",
            kind
        ))
    })?;

    let book_dir = data_dir.books_dir().join(&book_id);
    let path = book_dir.join(file_kind.relative_path());
    if !path.exists() {
        return Ok(IpcResponse::ok(String::new()));
    }
    // 文件 I/O 卸载到阻塞线程池
    let content = tokio::task::spawn_blocking(move || {
        std::fs::read_to_string(&path)
            .map_err(|e| AppError::internal(format!("读取真相文件失败: {}", e)))
    })
    .await
    .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;

    Ok(IpcResponse::ok(content))
}

// ── 调度器控制 ───────────────────────────────────────────────

use crate::domain::pipeline::scheduler::{SchedulerConfig, SchedulerState, SchedulerEvent};

/// 调度器状态响应
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerStatusResponse {
    pub running: bool,
    pub config: SchedulerConfigSnapshot,
}

/// 调度器配置快照（用于前端展示）
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerConfigSnapshot {
    pub write_cron: String,
    pub radar_cron: String,
    pub max_concurrent_books: usize,
    pub chapters_per_cycle: u32,
    pub max_chapters_per_day: u32,
    pub retry_delay_ms: u64,
    pub cooldown_after_chapter_ms: u64,
}

impl From<&SchedulerConfig> for SchedulerConfigSnapshot {
    fn from(c: &SchedulerConfig) -> Self {
        Self {
            write_cron: c.write_cron.clone(),
            radar_cron: c.radar_cron.clone(),
            max_concurrent_books: c.max_concurrent_books,
            chapters_per_cycle: c.chapters_per_cycle,
            max_chapters_per_day: c.max_chapters_per_day,
            retry_delay_ms: c.retry_delay_ms,
            cooldown_after_chapter_ms: c.cooldown_after_chapter_ms,
        }
    }
}

/// 启动调度器。
#[tauri::command]
pub async fn pipeline_scheduler_start(
    scheduler_state: State<'_, SchedulerState>,
) -> Result<IpcResponse<bool>, AppError> {
    scheduler_state.start().await?;
    tracing::info!("Pipeline scheduler: 已启动");
    Ok(IpcResponse::ok(true))
}

/// 停止调度器。
#[tauri::command]
pub async fn pipeline_scheduler_stop(
    scheduler_state: State<'_, SchedulerState>,
) -> Result<IpcResponse<bool>, AppError> {
    scheduler_state.stop().await;
    tracing::info!("Pipeline scheduler: 已停止");
    Ok(IpcResponse::ok(true))
}

/// 查询调度器状态。
#[tauri::command]
pub async fn pipeline_scheduler_status(
    scheduler_state: State<'_, SchedulerState>,
) -> Result<IpcResponse<SchedulerStatusResponse>, AppError> {
    let running = scheduler_state.is_running().await;
    let config_snapshot = SchedulerConfigSnapshot::from(&scheduler_state.scheduler().config);
    Ok(IpcResponse::ok(SchedulerStatusResponse {
        running,
        config: config_snapshot,
    }))
}

/// 手动触发一次写作循环。
#[tauri::command]
pub async fn pipeline_scheduler_trigger_write(
    scheduler_state: State<'_, SchedulerState>,
) -> Result<IpcResponse<bool>, AppError> {
    scheduler_state
        .scheduler()
        .trigger_write_cycle(scheduler_state.engine().clone())
        .await;
    tracing::info!("Pipeline scheduler: 手动触发写作循环");
    Ok(IpcResponse::ok(true))
}

/// 手动触发一次雷达扫描。
#[tauri::command]
pub async fn pipeline_scheduler_trigger_radar(
    scheduler_state: State<'_, SchedulerState>,
) -> Result<IpcResponse<bool>, AppError> {
    scheduler_state
        .scheduler()
        .trigger_radar_scan(scheduler_state.engine().clone())
        .await;
    tracing::info!("Pipeline scheduler: 手动触发雷达扫描");
    Ok(IpcResponse::ok(true))
}

/// 恢复已暂停的书籍。
#[tauri::command]
pub async fn pipeline_scheduler_resume_book(
    scheduler_state: State<'_, SchedulerState>,
    book_id: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_book_id(&book_id)?;
    scheduler_state.scheduler().resume_book(&book_id).await;
    tracing::info!(book_id = %book_id, "Pipeline scheduler: 书籍已恢复");
    Ok(IpcResponse::ok(true))
}

/// 检查书籍是否被暂停。
#[tauri::command]
pub async fn pipeline_scheduler_is_book_paused(
    scheduler_state: State<'_, SchedulerState>,
    book_id: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_book_id(&book_id)?;
    let paused = scheduler_state.scheduler().is_book_paused(&book_id).await;
    Ok(IpcResponse::ok(paused))
}

/// 订阅调度器事件（未实现：事件分发通过 Tauri event 系统）。
#[tauri::command]
pub async fn pipeline_scheduler_subscribe(
    _scheduler_state: State<'_, SchedulerState>,
) -> Result<IpcResponse<Vec<SchedulerEvent>>, AppError> {
    // 事件订阅应通过 Tauri 的 app_handle.emit 实现，当前未落地。
    // 不再静默返回空列表，显式报 not_implemented 避免调用方误以为成功。
    Err(AppError::not_implemented(
        "pipeline_scheduler_subscribe 尚未实现，事件分发请通过 Tauri event 系统",
    ))
}

// ═══════════════════════════════════════════════════════════════════════
//  Phase 6 扩展命令：短篇 pipeline + 同人导入 + 剧本/分镜 + 互动电影
// ═══════════════════════════════════════════════════════════════════════

use crate::domain::pipeline::agents::fanfic_canon_importer::{self, FanficCanonOutput};
use crate::domain::pipeline::agents::script_storyboard::ScriptTargetFormat;
use crate::domain::pipeline::runner::script_storyboard_runner::{
    self, InteractiveFilmCreationRunOptions, ScriptCreationRunOptions,
    StoryboardCreationRunOptions,
};
use crate::domain::pipeline::runner::short_fiction_runner::{
    ShortFictionConfig, ShortFictionRunOptions, ShortFictionRunner,
};
use crate::domain::pipeline::types::{FanficMode, Language};

// ── 短篇 pipeline ─────────────────────────────────────────────

/// 运行短篇 pipeline（6-agent 串行：大纲→审纲→修订→正文→审稿→修订→打包）。
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri IPC 命令，参数由前端逐项传入
pub async fn pipeline_short_fiction_run(
    agent_state: State<'_, AgentState>,
    data_dir: State<'_, DataDir>,
    story_id: Option<String>,
    direction: String,
    chapter_count: Option<u32>,
    chars_per_chapter: Option<u32>,
    language: Option<String>,
    reference: Option<String>,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    if direction.trim().is_empty() {
        return Err(AppError::invalid_input("direction 不能为空"));
    }
    let lang = parse_language(language.as_deref());
    let config = ShortFictionConfig {
        books_dir: data_dir.books_dir(),
        language: lang,
        ..Default::default()
    };
    let runner = ShortFictionRunner::new(config);
    let options = ShortFictionRunOptions {
        story_id,
        direction,
        chapter_count,
        chars_per_chapter,
        language: Some(lang),
        reference: reference.map(|text| crate::domain::pipeline::agents::short_fiction::ShortFictionReference { text }),
    };
    let result = runner.run(&agent_state.engine, &options).await?;
    Ok(IpcResponse::ok(serde_json::to_value(&result).map_err(|e| AppError::internal(e.to_string()))?))
}

// ── 同人 canonical 导入 ──────────────────────────────────────

/// 从原作素材文本导入 canonical 信息（5 section + 分块编译）。
#[tauri::command]
pub async fn pipeline_fanfic_import(
    agent_state: State<'_, AgentState>,
    source_text: String,
    source_name: String,
    fanfic_mode: Option<String>,
) -> Result<IpcResponse<FanficCanonOutput>, AppError> {
    if source_text.trim().is_empty() {
        return Err(AppError::invalid_input("source_text 不能为空"));
    }
    if source_name.trim().is_empty() {
        return Err(AppError::invalid_input("source_name 不能为空"));
    }
    let mode = parse_fanfic_mode(fanfic_mode.as_deref());
    let output = fanfic_canon_importer::import_from_text(
        &agent_state.engine,
        &source_text,
        &source_name,
        mode,
    )
    .await?;
    Ok(IpcResponse::ok(output))
}

// ── 剧本/分镜 pipeline ───────────────────────────────────────

/// 运行剧本创作（生成 script-spec.md + script.md）。
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri IPC 命令，参数由前端逐项传入
pub async fn pipeline_script_run(
    agent_state: State<'_, AgentState>,
    data_dir: State<'_, DataDir>,
    title: String,
    instruction: String,
    target_format: Option<String>,
    source_text: Option<String>,
    source_path: Option<String>,
    requirements: Option<String>,
    episode_count: Option<u32>,
    episode_duration: Option<String>,
    language: Option<String>,
    project_id: Option<String>,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    if title.trim().is_empty() {
        return Err(AppError::invalid_input("title 不能为空"));
    }
    let lang = parse_language(language.as_deref());
    let fmt = parse_target_format(target_format.as_deref());
    let options = ScriptCreationRunOptions {
        project_root: data_dir.books_dir(),
        title,
        instruction,
        source_kind: None,
        target_format: Some(fmt),
        source_text,
        source_path,
        requirements,
        episode_count,
        episode_duration,
        language: Some(lang),
        project_id,
        out_dir: None,
    };
    let result = script_storyboard_runner::run_script_creation(&agent_state.engine, &options).await?;
    Ok(IpcResponse::ok(serde_json::to_value(&result).map_err(|e| AppError::internal(e.to_string()))?))
}

/// 运行分镜创作（生成 storyboard-spec.md + storyboard.md + image-prompts.md + assets.json）。
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri IPC 命令，参数由前端逐项传入
pub async fn pipeline_storyboard_run(
    agent_state: State<'_, AgentState>,
    data_dir: State<'_, DataDir>,
    title: String,
    instruction: String,
    visual_style: Option<String>,
    aspect_ratio: Option<String>,
    granularity: Option<String>,
    max_shots: Option<u32>,
    source_text: Option<String>,
    source_path: Option<String>,
    requirements: Option<String>,
    language: Option<String>,
    project_id: Option<String>,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    if title.trim().is_empty() {
        return Err(AppError::invalid_input("title 不能为空"));
    }
    let lang = parse_language(language.as_deref());
    let options = StoryboardCreationRunOptions {
        project_root: data_dir.books_dir(),
        title,
        instruction,
        source_kind: None,
        source_text,
        source_path,
        requirements,
        visual_style,
        aspect_ratio,
        granularity,
        max_shots,
        language: Some(lang),
        project_id,
        out_dir: None,
    };
    let result = script_storyboard_runner::run_storyboard_creation(&agent_state.engine, &options).await?;
    Ok(IpcResponse::ok(serde_json::to_value(&result).map_err(|e| AppError::internal(e.to_string()))?))
}

/// 运行互动影游创作（生成 5 section Markdown + assets.json，不含 StoryGraph）。
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri IPC 命令，参数由前端逐项传入
pub async fn pipeline_interactive_film_run(
    agent_state: State<'_, AgentState>,
    data_dir: State<'_, DataDir>,
    title: String,
    instruction: String,
    target_audience: Option<String>,
    budget: Option<String>,
    episode_count: Option<u32>,
    reference_mode: Option<String>,
    source_text: Option<String>,
    source_path: Option<String>,
    requirements: Option<String>,
    language: Option<String>,
    project_id: Option<String>,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    if title.trim().is_empty() {
        return Err(AppError::invalid_input("title 不能为空"));
    }
    let lang = parse_language(language.as_deref());
    let options = InteractiveFilmCreationRunOptions {
        project_root: data_dir.books_dir(),
        title,
        instruction,
        source_kind: None,
        source_text,
        source_path,
        requirements,
        target_audience,
        episode_count,
        episode_duration: None,
        budget,
        reference_mode,
        language: Some(lang),
        project_id,
        out_dir: None,
    };
    let result = script_storyboard_runner::run_interactive_film_creation(&agent_state.engine, &options).await?;
    Ok(IpcResponse::ok(serde_json::to_value(&result).map_err(|e| AppError::internal(e.to_string()))?))
}

// ── 互动电影 StoryGraph ──────────────────────────────────────

/// 校验 StoryGraph（4 error 级 + 9 issue 级）。
#[tauri::command]
pub async fn pipeline_story_graph_validate(
    graph: crate::domain::pipeline::interactive_film::graph_schema::StoryGraph,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    let errors = crate::domain::pipeline::interactive_film::validation::review_story_graph(&graph);
    Ok(IpcResponse::ok(serde_json::to_value(&errors).map_err(|e| AppError::internal(e.to_string()))?))
}

/// 枚举 StoryGraph 的所有可玩路径（DFS + 状态去重）。
#[tauri::command]
pub async fn pipeline_story_graph_paths(
    graph: crate::domain::pipeline::interactive_film::graph_schema::StoryGraph,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    let result = crate::domain::pipeline::interactive_film::paths::enumerate_runtime_paths(&graph);
    Ok(IpcResponse::ok(serde_json::to_value(&result).map_err(|e| AppError::internal(e.to_string()))?))
}

/// 应用 StoryGraphDelta（upsert/remove 语义）。
#[tauri::command]
pub async fn pipeline_story_graph_apply_delta(
    mut graph: crate::domain::pipeline::interactive_film::graph_schema::StoryGraph,
    delta: crate::domain::pipeline::interactive_film::delta::StoryGraphDelta,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    crate::domain::pipeline::interactive_film::delta::apply_story_graph_delta(&mut graph, &delta)?;
    Ok(IpcResponse::ok(serde_json::to_value(&graph).map_err(|e| AppError::internal(e.to_string()))?))
}

// ── Phase 6 辅助解析函数 ─────────────────────────────────────

/// 解析语言参数
fn parse_language(s: Option<&str>) -> Language {
    match s.map(|x| x.to_lowercase()).as_deref() {
        Some("en") => Language::En,
        _ => Language::Zh,
    }
}

/// 解析同人模式
fn parse_fanfic_mode(s: Option<&str>) -> FanficMode {
    match s.map(|x| x.to_lowercase()).as_deref() {
        Some("au") => FanficMode::Au,
        Some("ooc") => FanficMode::Ooc,
        Some("cp") => FanficMode::Cp,
        _ => FanficMode::Canon,
    }
}

/// 解析剧本目标格式
fn parse_target_format(s: Option<&str>) -> ScriptTargetFormat {
    match s.map(|x| x.to_lowercase()).as_deref() {
        Some("vertical_short_drama") => ScriptTargetFormat::VerticalShortDrama,
        Some("screenplay") => ScriptTargetFormat::Screenplay,
        Some("audio_drama") => ScriptTargetFormat::AudioDrama,
        Some("interactive_script") => ScriptTargetFormat::InteractiveScript,
        _ => ScriptTargetFormat::GeneralScript,
    }
}
