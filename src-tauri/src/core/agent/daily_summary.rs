// 每日摘要任务 —— 定时合并短期记忆到长期记忆 MEMORY.md,衰减陈旧偏好。
//
// 任务执行内容:
// 1. 列出最近 24 小时的短期记忆条目
// 2. 为每个 agent role 汇总一条"今日要点",追加到对应 role 的 MEMORY.md
// 3. 衰减 30 天未出现的 learned_preferences
//
// 调度:通过 `tokio::spawn` 启动 interval 循环,默认 24 小时一次。
// 用户可通过 IPC 命令 `daily_summary_trigger` 手动触发一次。
//
// 注意:本模块是 I/O 密集型(文件读写 + DB 查询 + LLM 调用),不阻塞主线程。
// 所有错误都被 catch 并 log,不向上抛出(避免后台任务 panic)。

use std::sync::Arc;
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

/// 每日摘要任务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailySummaryConfig {
    /// 触发间隔(毫秒),默认 24 小时
    pub interval_ms: u64,
    /// 衰减 cutoff(天),默认 30
    pub stale_cutoff_days: u32,
    /// 短期记忆回看窗口(天),默认 1
    pub lookback_days: u32,
    /// 是否启用
    pub enabled: bool,
}

impl Default for DailySummaryConfig {
    fn default() -> Self {
        Self {
            interval_ms: 24 * 60 * 60 * 1000, // 24h
            stale_cutoff_days: 30,
            lookback_days: 1,
            enabled: true,
        }
    }
}

/// 每日摘要任务
pub struct DailySummaryTask {
    config: RwLock<DailySummaryConfig>,
    running: RwLock<bool>,
    handle: RwLock<Option<JoinHandle<()>>>,
}

impl DailySummaryTask {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(DailySummaryConfig::default()),
            running: RwLock::new(false),
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
        let mut running = self.running.write().await;
        if *running {
            return Ok(());
        }
        let config = self.config.read().await.clone();
        if !config.enabled {
            tracing::info!("Daily summary task is disabled, skipping start");
            return Ok(());
        }
        *running = true;

        let this = self.clone();
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(
                config.interval_ms.max(60_000), // 最低 1 分钟
            ));
            interval.tick().await; // 跳过首次立即触发
            loop {
                interval.tick().await;
                if !*this.running.read().await {
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
        let mut running = self.running.write().await;
        *running = false;
        if let Some(h) = self.handle.write().await.take() {
            h.abort();
        }
    }

    /// 查询任务是否正在运行
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// 手动触发一次执行(用于 IPC 调用)
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

        // 1. 衰减陈旧的 learned_preferences(cutoff = N 天前的 ISO 时间)
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

        // 2. 列出最近 N 天的短期记忆,按 agent_role 聚合
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

        // 4. 为每个 role 调用 LLM 生成"今日要点",追加到 MEMORY.md
        for (role, items) in by_role {
            let memory_path = identity_path(&data_dir, &role, IdentityKind::Memory);
            let mut existing = std::fs::read_to_string(&memory_path).unwrap_or_default();

            let date_header = format!("\n\n## 每日摘要 - {}\n", end_str);
            if existing.contains(&date_header) {
                // 已有当日摘要,跳过(避免重复追加)
                tracing::debug!(role = %role, "Memory section for today already exists, skipping");
                continue;
            }

            // 调用 LLM 压缩 N 天的短期记忆为简短要点
            let section = match summarize_role_memories(&engine, &role, &items, &end_str).await {
                Ok(s) => format!("{}\n", s),
                Err(e) => {
                    tracing::warn!(error = %e, role = %role, "LLM summary failed, skipping role");
                    report.errors.push(format!("summarize {role}: {e}"));
                    continue;
                }
            };

            // 膨胀控制:超过 100KB 时导出旧内容到归档文件,只保留最近 7 天
            const MAX_MEMORY_SIZE: usize = 100 * 1024; // 100KB
            if existing.len() + section.len() > MAX_MEMORY_SIZE {
                match archive_old_memory(&memory_path, &existing, &end_str, &role, &db) {
                    Ok(outcome) => {
                        existing = outcome.trimmed_content;
                        tracing::info!(role = %role, "Archived old memory content");
                        // DB 元数据写入失败等非致命警告,追加到 report
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

            if let Err(e) = std::fs::write(&memory_path, &existing) {
                tracing::warn!(error = %e, role = %role, "Failed to update MEMORY.md");
                report.errors.push(format!("update MEMORY for {role}: {e}"));
            } else {
                report.updated_roles.push(role);
            }
        }

        // 5. 衰减/更新 skill_usage_stats 中超过 90 天未使用的 skill(标记为 deprecated 由前端计算)
        // 这一步不需要额外操作,因为 maturity() 是从 last_used_at 计算的

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

/// 调用 LLM 将一个 role 的多条短期记忆压缩为简短每日要点。
///
/// 输入:N 条 ShortTermMemoryRow.summary
/// 输出:形如 `## 每日摘要 - YYYY-MM-DD\n\n<3-5 句话摘要>` 的段落,用于追加到 MEMORY.md
///
/// 失败策略:LLM 调用失败时返回 Err,由调用方决定是否跳过(不写空摘要,对齐 no silent fallback)。
async fn summarize_role_memories(
    engine: &AgentEngine,
    role: &str,
    items: &[ShortTermMemoryRow],
    date: &str,
) -> Result<String, AppError> {
    // 拼装输入:每条短期记忆一行,截断到 500 字符避免 token 爆炸
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

    let system_prompt = "你是记忆摘要助手。请把下面这个 agent role 在近期多条短期记忆中的要点,压缩成 3-5 句话的每日摘要。突出关键事件、教训与模式,不要罗列每个 session。直接输出摘要文本,不要加 Markdown 标题。";
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

/// 当 MEMORY.md 超过大小限制时,导出旧内容到归档文件,只保留最近 7 天。
///
/// - 归档文件路径:`MEMORY.archive.<YYYY-MM-DD>.md`(与 MEMORY.md 同目录)
/// - 保留策略:头部模板 + 最近 7 天的 `## 每日摘要 - YYYY-MM-DD` 段落
/// - 导出:7 天前的所有 `## 每日摘要` 段落(追加写入归档文件)
/// - 元数据:归档文件写入成功后,向 memory_archives 表插入索引记录
///   (role/archive_file/archived_at/content_size/content_summary/date_range)
///
/// ISO 日期字符串可按字典序比较,等价于时间序比较。
///
/// 返回 `ArchiveOutcome`:
/// - `trimmed_content`: 用于替换 MEMORY.md 的新内容(头部 + 近 7 天段落)
/// - `warnings`: 非致命错误(如 DB 元数据写入失败),调用方应追加到 report.errors
///
/// 文件写入失败 → 返回 Err(调用方跳过 MEMORY.md 更新)。
/// DB 元数据写入失败 → 返回 Ok + warning(MEMORY.md 仍可正常裁剪,
///   前端 list_archives 会少一条,但不阻塞主流程)。
fn archive_old_memory(
    memory_path: &std::path::Path,
    existing: &str,
    today: &str,
    role: &str,
    db: &Database,
) -> Result<ArchiveOutcome, AppError> {
    const KEEP_DAYS: i64 = 7;
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(KEEP_DAYS))
        .format("%Y-%m-%d")
        .to_string();

    // 按 "\n\n## 每日摘要 - " 分割:第一段是头部模板,后续每段以 "YYYY-MM-DD\n..." 开头
    let parts: Vec<&str> = existing.split("\n\n## 每日摘要 - ").collect();
    let header = parts[0].to_string();
    let mut kept: Vec<String> = Vec::new();
    let mut archived: Vec<String> = Vec::new();
    let mut archived_dates: Vec<String> = Vec::new();

    for part in parts.iter().skip(1) {
        // part 以 "YYYY-MM-DD\n..." 开头,取日期前缀
        let date_end = part.find('\n').unwrap_or(part.len());
        let date_str = &part[..date_end];
        let section = format!("\n\n## 每日摘要 - {}", part);
        if date_str >= cutoff.as_str() {
            kept.push(section);
        } else {
            archived.push(section);
            archived_dates.push(date_str.to_string());
        }
    }

    let mut warnings: Vec<String> = Vec::new();

    // 写归档文件 + DB 元数据(只在有旧内容时)
    if !archived.is_empty() {
        let archive_name = format!("MEMORY.archive.{}.md", today);
        let archive_path = memory_path.with_file_name(&archive_name);
        let archive_content = format!(
            "# Memory Archive\n\n本文件由 daily_summary 任务于 {} 自动导出,包含 7 天前的 MEMORY.md 摘要段落。\n\n{}",
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

        // 写 DB 元数据到 memory_archives 表
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

    // 重建主文件:头部 + 保留的段落
    let mut new_content = header;
    new_content.push_str(&kept.join(""));
    Ok(ArchiveOutcome {
        trimmed_content: new_content,
        warnings,
    })
}

/// 归档操作结果
struct ArchiveOutcome {
    /// 裁剪后的 MEMORY.md 内容(头部 + 近 7 天段落)
    trimmed_content: String,
    /// 非致命警告(如 DB 元数据写入失败),调用方应追加到 report.errors
    warnings: Vec<String>,
}

/// 从归档内容中提取摘要:去掉 markdown 标题行和空行,取前 max_chars 字符。
///
/// 中文字符按 Unicode scalar 计数(1 字 = 1 char),与前端期望一致。
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

/// 从归档段落对应的日期列表中取最早/最晚日期。
///
/// 日期格式:YYYY-MM-DD(字典序 = 时间序)。空列表返回两个空字符串。
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

/// 摘要任务执行结果
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct DailySummaryReport {
    /// 审查的短期记忆条数
    pub reviewed_short_term: usize,
    /// 衰减的学习偏好数量
    pub decayed_preferences: i64,
    /// 更新的 agent role 列表
    pub updated_roles: Vec<String>,
    /// 执行耗时(毫秒)
    pub duration_ms: u64,
    /// 执行过程中的错误(不阻塞任务)
    pub errors: Vec<String>,
}

// ── Tauri State 包装 ────────────────────────────────────────

/// 每日摘要任务的 Tauri State 容器。
///
/// 持有 `Arc<DailySummaryTask>` + 必要依赖(engine/db/data_dir),
/// 供 IPC 命令调用。任务本身是可选启动的(默认不启动,需用户在设置页开启)。
pub struct DailySummaryState {
    pub task: Arc<DailySummaryTask>,
    pub engine: Arc<AgentEngine>,
    pub db: Database,
    pub data_dir: DataDir,
}

impl DailySummaryState {
    pub fn new(engine: AgentEngine, db: Database, data_dir: DataDir) -> Self {
        Self {
            task: Arc::new(DailySummaryTask::new()),
            engine: Arc::new(engine),
            db,
            data_dir,
        }
    }
}
