//! ═══════════════════════════════════════════════════════════════════════════
//! Daily Summary - 每日摘要任务
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 定时合并短期记忆到长期记忆 MEMORY.md, 衰减陈旧偏好。
//!
//! 任务执行内容:
//! 1. 列出最近 24 小时的短期记忆条目
//! 2. 为每个 agent role 汇总一条 "今日要点", 追加到对应 role 的 MEMORY.md
//! 3. 衰减 30 天未出现的 learned_preferences
//!
//! 调度: 通过 tokio::spawn 启动 interval 循环, 默认 24 小时一次。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::core::agent::engine::AgentEngine;
use crate::core::agent::identity::{identity_path, IdentityKind};
use crate::infrastructure::db::connection::Database;
use crate::infrastructure::db::stores::memory_archive::NewMemoryArchive;
use crate::infrastructure::db::stores::short_term_memory::ShortTermMemoryRow;
use crate::infrastructure::fs::data_dir::DataDir;
use crate::shared::error::AppError;

// ── 配置结构体 ──────────────────────────────────────────────────────────────

/// 每日摘要任务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailySummaryConfig {
    /// 触发间隔 (毫秒), 默认 24 小时
    pub interval_ms: u64,
    /// 衰减 cutoff (天), 默认 30
    pub stale_cutoff_days: u32,
    /// 短期记忆回看窗口 (天), 默认 1
    pub lookback_days: u32,
    /// 是否启用
    pub enabled: bool,
}

impl Default for DailySummaryConfig {
    fn default() -> Self {
        Self {
            interval_ms: 24 * 60 * 60 * 1000,
            stale_cutoff_days: 30,
            lookback_days: 1,
            enabled: true,
        }
    }
}

// ── 摘要任务 ────────────────────────────────────────────────────────────────

/// 每日摘要任务
pub struct DailySummaryTask {
    config: RwLock<DailySummaryConfig>,
    running: AtomicBool,
    handle: RwLock<Option<JoinHandle<()>>>,
}

impl DailySummaryTask {
    /// 创建摘要任务实例
    pub fn new() -> Self {
        Self {
            config: RwLock::new(DailySummaryConfig::default()),
            running: AtomicBool::new(false),
            handle: RwLock::new(None),
        }
    }

    /// 启动定时任务
    pub async fn start(
        self: &Arc<Self>,
        engine: Arc<AgentEngine>,
        db: Database,
        data_dir: DataDir,
    ) -> Result<(), AppError> {
        if self.running.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            return Ok(());
        }
        let config = self.config.read().await.clone();
        if !config.enabled {
            self.running.store(false, Ordering::SeqCst);
            tracing::info!("Daily summary task is disabled, skipping start");
            return Ok(());
        }

        let this = self.clone();
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(
                config.interval_ms.max(60_000),
            ));
            interval.tick().await;
            loop {
                interval.tick().await;
                if !this.running.load(Ordering::SeqCst) {
                    break;
                }
                let cfg = this.config.read().await.clone();
                if let Err(e) = this
                    .clone()
                    .run_once(engine.clone(), db.clone(), data_dir.clone(), &cfg)
                    .await
                {
                    tracing::error!(error = %e, "Daily summary task failed");
                }
            }
        });
        *self.handle.write().await = Some(handle);
        tracing::info!(
            interval_ms = config.interval_ms,
            "Daily summary task started"
        );
        Ok(())
    }

    /// 停止定时任务
    pub async fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(h) = self.handle.write().await.take() {
            h.abort();
        }
    }

    /// 查询任务是否正在运行
    pub async fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// 手动触发一次执行
    pub async fn trigger_once(
        self: &Arc<Self>,
        engine: Arc<AgentEngine>,
        db: Database,
        data_dir: DataDir,
    ) -> Result<DailySummaryReport, AppError> {
        let cfg = self.config.read().await.clone();
        self.clone().run_once(engine, db, data_dir, &cfg).await
    }

    /// 更新配置
    pub async fn update_config(&self, new_config: DailySummaryConfig) {
        *self.config.write().await = new_config;
    }

    /// 读取当前配置
    pub async fn config(&self) -> DailySummaryConfig {
        self.config.read().await.clone()
    }

    /// 执行一次摘要任务
    async fn run_once(
        self: Arc<Self>,
        engine: Arc<AgentEngine>,
        db: Database,
        data_dir: DataDir,
        cfg: &DailySummaryConfig,
    ) -> Result<DailySummaryReport, AppError> {
        tracing::info!("Daily summary task started");
        let start = std::time::Instant::now();
        let mut report = DailySummaryReport::default();
        let now = Utc::now();

        // 1. 衰减陈旧的 learned_preferences
        let cutoff_iso = (now - chrono::Duration::days(cfg.stale_cutoff_days as i64))
            .to_rfc3339();
        match db.decay_stale_preferences(&cutoff_iso) {
            Ok(n) => {
                tracing::info!(decayed = n, "Decayed stale learned preferences");
                report.decayed_preferences = n as i64;
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to decay stale preferences");
                report.errors.push(format!("decay preferences: {e}"));
            }
        }

        // 2. 列出最近 N 天的短期记忆
        let start_date = now - chrono::Duration::days(cfg.lookback_days as i64);
        let start_str = start_date.format("%Y-%m-%d").to_string();
        let end_str = now.format("%Y-%m-%d").to_string();

        let rows = match db.list_short_term_by_date_range(&start_str, &end_str) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to list short-term memory");
                report.errors.push(format!("list short-term: {e}"));
                return Ok(report);
            }
        };
        report.reviewed_short_term = rows.len();

        // 3. 按 agent_role 分组
        use std::collections::HashMap;
        let mut by_role: HashMap<String, Vec<_>> = HashMap::new();
        for row in rows {
            let role = row.agent_role.clone().unwrap_or_else(|| "main".to_string());
            by_role.entry(role).or_default().push(row);
        }

        // 4. 为每个 role 调用 LLM 生成 "今日要点"
        for (role, items) in by_role {
            let memory_path = identity_path(&data_dir, &role, IdentityKind::Memory);
            let mut existing = match tokio::task::spawn_blocking({
                let path = memory_path.clone();
                move || std::fs::read_to_string(&path).unwrap_or_default()
            })
            .await
            {
                Ok(content) => content,
                Err(e) => {
                    tracing::error!(error = %e, role = %role, "spawn_blocking panic while reading MEMORY.md");
                    report.errors.push(format!("read memory {role}: spawn_blocking join failed: {e}"));
                    continue;
                }
            };

            let date_header = format!("\n\n## 每日摘要 - {}\n", end_str);
            if existing.contains(&date_header) {
                tracing::debug!(role = %role, "Memory section for today already exists, skipping");
                continue;
            }

            let section = match summarize_role_memories(&engine, &role, &items, &end_str).await {
                Ok(s) => format!("{}\n", s),
                Err(e) => {
                    tracing::warn!(error = %e, role = %role, "LLM summary failed, skipping role");
                    report.errors.push(format!("summarize {role}: {e}"));
                    continue;
                }
            };

            // 膨胀控制: 超过 100KB 时导出旧内容到归档文件
            const MAX_MEMORY_SIZE: usize = 100 * 1024;
            if existing.len() + section.len() > MAX_MEMORY_SIZE {
                let outcome = tokio::task::spawn_blocking({
                    let memory_path = memory_path.clone();
                    let existing = existing.clone();
                    let end_str = end_str.to_string();
                    let role = role.clone();
                    let db = db.clone();
                    move || archive_old_memory(&memory_path, &existing, &end_str, &role, &db)
                })
                .await
                .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))?;
                match outcome {
                    Ok(outcome) => {
                        existing = outcome.trimmed_content;
                        tracing::info!(role = %role, "Archived old memory content");
                        report.errors.extend(outcome.warnings);
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e, role = %role,
                            "Failed to archive memory, skipping update"
                        );
                        report.errors.push(format!("archive {role}: {e}"));
                        continue;
                    }
                }
            }
            existing.push_str(&section);

            let write_result = tokio::task::spawn_blocking({
                let path = memory_path.clone();
                let content = existing.clone();
                move || std::fs::write(&path, &content)
            })
            .await;
            match write_result {
                Ok(Ok(())) => {
                    report.updated_roles.push(role);
                }
                Ok(Err(e)) => {
                    tracing::warn!(error = %e, role = %role, "Failed to update MEMORY.md");
                    report.errors.push(format!("update MEMORY for {role}: {e}"));
                }
                Err(e) => {
                    tracing::warn!(error = %e, role = %role, "spawn_blocking join failed");
                    report.errors.push(format!("update MEMORY for {role}: join {e}"));
                }
            }
        }

        report.duration_ms = start.elapsed().as_millis() as u64;
        tracing::info!(
            duration_ms = report.duration_ms,
            reviewed = report.reviewed_short_term,
            updated_roles = report.updated_roles.len(),
            decayed = report.decayed_preferences,
            errors = report.errors.len(),
            "Daily summary task completed"
        );
        Ok(report)
    }
}

/// 调用 LLM 将一个 role 的多条短期记忆压缩为简短每日要点
async fn summarize_role_memories(
    engine: &AgentEngine,
    role: &str,
    items: &[ShortTermMemoryRow],
    date: &str,
) -> Result<String, AppError> {
    let transcript = items
        .iter()
        .map(|item| {
            let summary: String = if item.summary.chars().count() > 500 {
                let truncated: String = item.summary.chars().take(500).collect();
                format!("{}...", truncated)
            } else if item.summary.is_empty() {
                "(无摘要)".to_string()
            } else {
                item.summary.clone()
            };
            format!(
                "- [{}] session `{}`: {}",
                item.entry_date, item.session_id, summary
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let system_prompt = "你是记忆摘要助手。请把下面这个 agent role 在近期多条短期记忆中的要点, 压缩成 3-5 句话的每日摘要。突出关键事件、教训与模式, 不要罗列每个 session。直接输出摘要文本, 不要加 Markdown 标题。";
    let user_message = format!(
        "Role: {}\n日期: {}\n条目数: {}\n\n短期记忆:\n{}",
        role,
        date,
        items.len(),
        transcript
    );

    let summary = engine.prompt_once(system_prompt, &user_message).await?;
    Ok(format!("## 每日摘要 - {}\n\n{}", date, summary.trim()))
}

/// 归档操作结果
struct ArchiveOutcome {
    /// 裁剪后的内容
    trimmed_content: String,
    /// 非致命警告
    warnings: Vec<String>,
}

/// 当 MEMORY.md 超过大小限制时, 导出旧内容到归档文件
fn archive_old_memory(
    memory_path: &std::path::Path,
    existing: &str,
    today: &str,
    role: &str,
    db: &Database,
) -> Result<ArchiveOutcome, AppError> {
    const KEEP_DAYS: i64 = 7;
    let cutoff_date = chrono::Utc::now().date_naive() - chrono::Duration::days(KEEP_DAYS);

    let parts: Vec<&str> = existing.split("\n\n## 每日摘要 - ").collect();
    let header = parts[0].to_string();
    let mut kept: Vec<String> = Vec::new();
    let mut archived: Vec<String> = Vec::new();
    let mut archived_dates: Vec<String> = Vec::new();

    for part in parts.iter().skip(1) {
        let date_end = part.find('\n').unwrap_or(part.len());
        let date_str = &part[..date_end];
        let section = format!("\n\n## 每日摘要 - {}", part);
        match chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            Ok(d) if d < cutoff_date => {
                archived.push(section);
                archived_dates.push(date_str.to_string());
            }
            Ok(_) => kept.push(section),
            Err(e) => {
                tracing::warn!(
                    role = %role,
                    date_str = %date_str,
                    error = %e,
                    "Malformed date in MEMORY.md section, keeping instead of archiving"
                );
                kept.push(section);
            }
        }
    }

    let mut warnings: Vec<String> = Vec::new();

    if !archived.is_empty() {
        let archive_name = format!("MEMORY.archive.{}.md", today);
        let archive_path = memory_path.with_file_name(&archive_name);
        let archive_content = format!(
            "# Memory Archive\n\n本文件由 daily_summary 任务于 {} 自动导出, 包含 7 天前的 MEMORY.md 摘要段落。\n\n{}",
            today,
            archived.join("")
        );
        std::fs::write(&archive_path, &archive_content).map_err(|e| {
            AppError::internal(format!(
                "Failed to write archive {}: {}",
                archive_path.display(),
                e
            ))
        })?;

        let content_size = archive_content.len() as i64;
        let content_summary = build_archive_summary(&archive_content, 200);
        let (date_range_start, date_range_end) = compute_date_range(&archived_dates);
        let now = chrono::Utc::now();
        let row = NewMemoryArchive {
            role,
            archive_file: &archive_name,
            archived_at: now.timestamp(),
            content_size,
            content_summary: &content_summary,
            date_range_start: &date_range_start,
            date_range_end: &date_range_end,
            created_at: &now.to_rfc3339(),
        };
        if let Err(e) = db.insert_archive(&row) {
            tracing::warn!(
                error = %e, role = role,
                "Archive file written but DB metadata insert failed"
            );
            warnings.push(format!("archive {role} DB metadata: {e}"));
        }
    }

    let mut new_content = header;
    new_content.push_str(&kept.join(""));
    Ok(ArchiveOutcome {
        trimmed_content: new_content,
        warnings,
    })
}

/// 从归档内容中提取摘要
fn build_archive_summary(content: &str, max_chars: usize) -> String {
    content
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_chars)
        .collect()
}

/// 从日期列表中取最早/最晚日期
fn compute_date_range(dates: &[String]) -> (String, String) {
    if dates.is_empty() {
        return (String::new(), String::new());
    }
    let mut min = dates[0].as_str();
    let mut max = dates[0].as_str();
    for d in dates.iter().skip(1) {
        if d.as_str() < min {
            min = d.as_str();
        }
        if d.as_str() > max {
            max = d.as_str();
        }
    }
    (min.to_string(), max.to_string())
}

impl Default for DailySummaryTask {
    fn default() -> Self {
        Self::new()
    }
}

// ── 执行报告 ────────────────────────────────────────────────────────────────

/// 摘要任务执行结果
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct DailySummaryReport {
    /// 审查的短期记忆条数
    pub reviewed_short_term: usize,
    /// 衰减的学习偏好数量
    pub decayed_preferences: i64,
    /// 更新的 agent role 列表
    pub updated_roles: Vec<String>,
    /// 执行耗时 (毫秒)
    pub duration_ms: u64,
    /// 执行过程中的错误
    pub errors: Vec<String>,
}

// ── Tauri State 包装 ────────────────────────────────────────────────────────

/// 每日摘要任务的 Tauri State 容器
pub struct DailySummaryState {
    pub task: Arc<DailySummaryTask>,
    pub engine: Arc<AgentEngine>,
    pub db: Database,
    pub data_dir: DataDir,
}

impl DailySummaryState {
    /// 创建 State 容器
    pub fn new(engine: AgentEngine, db: Database, data_dir: DataDir) -> Self {
        Self {
            task: Arc::new(DailySummaryTask::new()),
            engine: Arc::new(engine),
            db,
            data_dir,
        }
    }
}