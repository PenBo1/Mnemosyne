// Pipeline Scheduler。
//
// 职责：定时调度写作循环 + 雷达扫描，带质量门控（连续失败暂停）、
// 每日章节上限、章节间冷却、重叠跳过、失败维度聚类告警。
//
// 架构：
// - Scheduler 持有 PipelineRunner + 配置，通过 tokio::spawn 运行后台 interval 循环
// - 每个 book 的 writeCycle 串行写 chaptersPerCycle 章，带重试 + 冷却
// - 失败计数 → 连续失败达阈值则暂停该书
// - 每日上限：跨所有书的章节总数

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use chrono::Local;
use dashmap::DashMap;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;

use crate::core::agent::engine::AgentEngine;
use crate::domain::pipeline::runner::{PipelineConfig, PipelineRunner};
use crate::domain::pipeline::types::{BookConfig, BookStatus};
use crate::shared::error::AppError;

// ── 配置 ─────────────────────────────────────────────────────

/// 质量门控配置
#[derive(Debug, Clone)]
pub struct QualityGates {
    /// 审计失败后立即重试的最大次数
    pub max_audit_retries: u32,
    /// 连续失败多少次后暂停该书
    pub pause_after_consecutive_failures: u32,
    /// 每次重试温度提升步长（Rust 端暂不传温度）
    pub retry_temperature_step: f32,
}

impl Default for QualityGates {
    fn default() -> Self {
        Self {
            max_audit_retries: 2,
            pause_after_consecutive_failures: 3,
            retry_temperature_step: 0.1,
        }
    }
}

/// 调度器配置
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// 写作循环的 cron 表达式（简化解析：支持 `*/N * * * *` 与 `0 */N * * *`）
    pub write_cron: String,
    /// 雷达扫描的 cron 表达式
    pub radar_cron: String,
    /// 单次循环最多并发处理的书籍数
    pub max_concurrent_books: usize,
    /// 单次循环每本书最多写多少章
    pub chapters_per_cycle: u32,
    /// 审计失败后重试前的延迟（毫秒）
    pub retry_delay_ms: u64,
    /// 章节之间的冷却时间（毫秒）
    pub cooldown_after_chapter_ms: u64,
    /// 每日跨所有书的章节总数上限
    pub max_chapters_per_day: u32,
    /// 质量门控
    pub quality_gates: QualityGates,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            write_cron: "*/30 * * * *".to_string(),
            radar_cron: "0 */6 * * *".to_string(),
            max_concurrent_books: 1,
            chapters_per_cycle: 1,
            retry_delay_ms: 5000,
            cooldown_after_chapter_ms: 10000,
            max_chapters_per_day: 20,
            quality_gates: QualityGates::default(),
        }
    }
}

// ── 事件回调 ─────────────────────────────────────────────────

/// 调度器事件（onChapterComplete/onError/onPause 回调）
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SchedulerEvent {
    /// 章节完成
    ChapterComplete {
        #[serde(rename = "bookId")]
        book_id: String,
        chapter: u32,
        status: String,
    },
    /// 发生错误
    Error {
        #[serde(rename = "bookId")]
        book_id: String,
        message: String,
    },
    /// 书籍被暂停
    Paused {
        #[serde(rename = "bookId")]
        book_id: String,
        reason: String,
    },
    /// 诊断告警（某维度失败次数 ≥ 3）
    DiagnosticAlert {
        #[serde(rename = "bookId")]
        book_id: String,
        chapter: u32,
        dimension: String,
        #[serde(rename = "failureCount")]
        failure_count: u32,
    },
}

/// 事件回调函数类型
pub type EventCallback = Arc<dyn Fn(SchedulerEvent) + Send + Sync>;

// ── 内部状态 ─────────────────────────────────────────────────

/// 已调度的后台任务
struct ScheduledTask {
    name: String,
    handle: JoinHandle<()>,
}

/// 书籍运行时状态（连续失败 / 暂停 / 失败维度）
struct BookRuntimeState {
    consecutive_failures: u32,
    failure_dimensions: HashMap<String, u32>,
}

impl BookRuntimeState {
    fn new() -> Self {
        Self {
            consecutive_failures: 0,
            failure_dimensions: HashMap::new(),
        }
    }
}

// ── Scheduler ────────────────────────────────────────────────

/// Pipeline 调度器
pub struct Scheduler {
    /// 调度器配置（公开供查询）
    pub config: SchedulerConfig,
    pipeline_config: PipelineConfig,
    /// 后台任务句柄
    tasks: Mutex<Vec<ScheduledTask>>,
    /// 是否正在运行
    running: RwLock<bool>,
    /// 写作循环是否在执行中（防止重叠）
    write_cycle_in_flight: Mutex<bool>,
    /// 雷达扫描是否在执行中
    radar_scan_in_flight: Mutex<bool>,
    /// 每本书的运行时状态
    book_states: DashMap<String, Mutex<BookRuntimeState>>,
    /// 已暂停的书
    paused_books: RwLock<HashSet<String>>,
    /// 每日章节计数：日期字符串 → 计数
    daily_chapter_count: Mutex<HashMap<String, u32>>,
    /// 事件回调
    on_event: Option<EventCallback>,
}

impl Scheduler {
    /// 创建调度器。
    /// `pipeline_config` 需包含 books_dir（通常从 DataDir.books_dir() 获取）。
    pub fn new(pipeline_config: PipelineConfig, config: SchedulerConfig) -> Self {
        Self {
            config,
            pipeline_config,
            tasks: Mutex::new(Vec::new()),
            running: RwLock::new(false),
            write_cycle_in_flight: Mutex::new(false),
            radar_scan_in_flight: Mutex::new(false),
            book_states: DashMap::new(),
            paused_books: RwLock::new(HashSet::new()),
            daily_chapter_count: Mutex::new(HashMap::new()),
            on_event: None,
        }
    }

    /// 设置事件回调
    pub fn with_event_callback(mut self, callback: EventCallback) -> Self {
        self.on_event = Some(callback);
        self
    }

    /// 启动调度器：立即跑一次写作循环，然后按 cron 间隔定时触发。
    /// 需要 `self_ref`（Arc<Scheduler>）以便后台任务持有引用。
    pub async fn start(self: &Arc<Self>, engine: Arc<AgentEngine>) -> Result<(), AppError> {
        let mut running = self.running.write().await;
        if *running {
            return Ok(());
        }
        *running = true;
        drop(running);

        tracing::info!(
            write_cron = %self.config.write_cron,
            radar_cron = %self.config.radar_cron,
            "Scheduler: 启动"
        );

        // 立即触发一次写作循环
        let engine_clone = engine.clone();
        self.trigger_write_cycle(engine_clone).await;

        // 定时写作循环
        let write_interval_ms = cron_to_ms(&self.config.write_cron);
        let write_handle = spawn_interval_task(
            "write-cycle",
            write_interval_ms,
            engine.clone(),
            self.clone(),
            SchedulerKind::Write,
        );

        // 定时雷达扫描
        let radar_interval_ms = cron_to_ms(&self.config.radar_cron);
        let radar_handle = spawn_interval_task(
            "radar-scan",
            radar_interval_ms,
            engine.clone(),
            self.clone(),
            SchedulerKind::Radar,
        );

        let mut tasks = self.tasks.lock().await;
        tasks.push(ScheduledTask {
            name: "write-cycle".to_string(),
            handle: write_handle,
        });
        tasks.push(ScheduledTask {
            name: "radar-scan".to_string(),
            handle: radar_handle,
        });

        Ok(())
    }

    /// 停止调度器：取消所有后台任务。
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        if !*running {
            return;
        }
        *running = false;
        drop(running);

        let mut tasks = self.tasks.lock().await;
        for task in tasks.drain(..) {
            task.handle.abort();
            tracing::info!(task = %task.name, "Scheduler: 任务已停止");
        }
    }

    /// 是否正在运行
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// 恢复已暂停的书籍
    pub async fn resume_book(&self, book_id: &str) {
        {
            let mut paused = self.paused_books.write().await;
            paused.remove(book_id);
        }
        self.book_states.remove(book_id);
        tracing::info!(book_id = %book_id, "Scheduler: 书籍已恢复");
    }

    /// 检查书籍是否被暂停
    pub async fn is_book_paused(&self, book_id: &str) -> bool {
        self.paused_books.read().await.contains(book_id)
    }

    /// 手动触发一次写作循环（用于 IPC 命令）。
    /// 需要 `self_ref`（Arc<Scheduler>）以便后台任务持有引用。
    pub async fn trigger_write_cycle(self: &Arc<Self>, engine: Arc<AgentEngine>) {
        // 防止重叠
        let mut in_flight = self.write_cycle_in_flight.lock().await;
        if *in_flight {
            tracing::warn!("Scheduler: 写作循环仍在执行，跳过本次触发");
            return;
        }
        *in_flight = true;
        drop(in_flight);

        // 在后台执行，不阻塞调用方
        let scheduler = self.clone();
        tokio::spawn(async move {
            scheduler.clone().run_write_cycle(engine).await;
            let mut flag = scheduler.write_cycle_in_flight.lock().await;
            *flag = false;
        });
    }

    /// 手动触发一次雷达扫描。
    pub async fn trigger_radar_scan(self: &Arc<Self>, engine: Arc<AgentEngine>) {
        let mut in_flight = self.radar_scan_in_flight.lock().await;
        if *in_flight {
            tracing::warn!("Scheduler: 雷达扫描仍在执行，跳过本次触发");
            return;
        }
        *in_flight = true;
        drop(in_flight);

        let scheduler = self.clone();
        tokio::spawn(async move {
            scheduler.run_radar_scan(engine).await;
            let mut flag = scheduler.radar_scan_in_flight.lock().await;
            *flag = false;
        });
    }

    // ── 内部：写作循环 ───────────────────────────────────────

    /// 运行一次写作循环：列出活跃书籍 → 并发处理
    async fn run_write_cycle(self: Arc<Self>, engine: Arc<AgentEngine>) {
        // 每日上限检查
        if self.is_daily_cap_reached().await {
            tracing::info!(
                max = self.config.max_chapters_per_day,
                "Scheduler: 达到每日上限，跳过本次循环"
            );
            return;
        }

        // 列出活跃书籍
        let active_books = match self.list_active_books().await {
            Ok(books) => books,
            Err(e) => {
                tracing::error!(error = %e, "Scheduler: 列出活跃书籍失败");
                return;
            }
        };

        let books_to_write: Vec<(String, BookConfig)> = active_books
            .into_iter()
            .take(self.config.max_concurrent_books)
            .collect();

        if books_to_write.is_empty() {
            tracing::debug!("Scheduler: 无活跃书籍");
            return;
        }

        // 并发处理（每个书一个 task）
        let mut handles = Vec::new();
        for (book_id, book_config) in books_to_write {
            let engine_clone = engine.clone();
            let scheduler_clone = self.clone();
            handles.push(tokio::spawn(async move {
                scheduler_clone
                    .process_book(engine_clone, book_id, book_config)
                    .await;
            }));
        }
        for handle in handles {
            let _ = handle.await;
        }
    }

    /// 列出所有活跃书籍（status = Active 或 Outlining，且未被暂停）
    async fn list_active_books(&self) -> Result<Vec<(String, BookConfig)>, AppError> {
        let books_dir = self.pipeline_config.books_dir.clone();
        if !books_dir.exists() {
            return Ok(Vec::new());
        }

        let paused = self.paused_books.read().await;
        let mut result = Vec::new();

        for entry in std::fs::read_dir(&books_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let book_id = entry.file_name().to_string_lossy().to_string();
            // 跳过临时目录
            if book_id.starts_with('.') {
                continue;
            }
            if paused.contains(&book_id) {
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

            if matches!(book.status, BookStatus::Active | BookStatus::Outlining) {
                result.push((book_id, book));
            }
        }

        Ok(result)
    }

    /// 处理单本书：写 chaptersPerCycle 章，带重试 + 冷却
    async fn process_book(&self, engine: Arc<AgentEngine>, book_id: String, book_config: BookConfig) {
        for i in 0..self.config.chapters_per_cycle {
            if !*self.running.read().await {
                return;
            }
            if self.is_daily_cap_reached().await {
                return;
            }
            if self.is_book_paused(&book_id).await {
                return;
            }

            // 章节间冷却（第一章跳过）
            if i > 0 && self.config.cooldown_after_chapter_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.config.cooldown_after_chapter_ms)).await;
            }

            let success = self.write_one_chapter(engine.clone(), &book_id, &book_config).await;
            if !success {
                // 失败重试（在重试限制内）
                let failures = self.get_consecutive_failures(&book_id).await;
                if failures <= self.config.quality_gates.max_audit_retries && self.config.retry_delay_ms > 0 {
                    tracing::warn!(
                        book_id = %book_id,
                        delay_ms = self.config.retry_delay_ms,
                        "Scheduler: 等待后重试"
                    );
                    tokio::time::sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
                    let retry_success = self.write_one_chapter(engine.clone(), &book_id, &book_config).await;
                    if !retry_success {
                        break; // 第二次失败则停止该书本轮
                    }
                } else {
                    break;
                }
            }
        }
    }

    /// 写一章。返回 true 表示审计通过（ready-for-review）。
    async fn write_one_chapter(&self, engine: Arc<AgentEngine>, book_id: &str, _book_config: &BookConfig) -> bool {
        let runner = PipelineRunner::new(self.pipeline_config.clone());

        match runner.write_next_chapter(&engine, book_id, None).await {
            Ok(result) => {
                if result.status == "ready-for-review" {
                    // 成功：清除失败计数
                    self.reset_consecutive_failures(book_id).await;
                    self.record_chapter_written().await;

                    tracing::info!(
                        book_id = %book_id,
                        chapter = result.chapter_number,
                        words = result.word_count,
                        "Scheduler: 章节写作完成"
                    );
                    self.emit_event(SchedulerEvent::ChapterComplete {
                        book_id: book_id.to_string(),
                        chapter: result.chapter_number,
                        status: result.status.clone(),
                    });
                    return true;
                }

                // 审计失败：收集问题类别
                let issue_categories: Vec<String> = result
                    .audit_result
                    .issues
                    .iter()
                    .map(|i| i.category.clone())
                    .collect();

                self.handle_audit_failure(book_id, result.chapter_number, &issue_categories).await;
                self.emit_event(SchedulerEvent::ChapterComplete {
                    book_id: book_id.to_string(),
                    chapter: result.chapter_number,
                    status: result.status.clone(),
                });
                false
            }
            Err(e) => {
                tracing::error!(book_id = %book_id, error = %e, "Scheduler: 写章失败");
                self.emit_event(SchedulerEvent::Error {
                    book_id: book_id.to_string(),
                    message: e.to_string(),
                });
                self.handle_audit_failure(book_id, 0, &[]).await;
                false
            }
        }
    }

    // ── 内部：质量门控 ───────────────────────────────────────

    /// 处理审计失败：增加失败计数、追踪维度聚类、必要时暂停
    async fn handle_audit_failure(&self, book_id: &str, chapter_number: u32, issue_categories: &[String]) {
        let book_state = self
            .book_states
            .entry(book_id.to_string())
            .or_insert_with(|| Mutex::new(BookRuntimeState::new()));

        let mut state = book_state.lock().await;
        state.consecutive_failures += 1;
        let failures = state.consecutive_failures;

        // 追踪失败维度
        for cat in issue_categories {
            *state.failure_dimensions.entry(cat.clone()).or_insert(0) += 1;
        }

        // 检查维度聚类（任一维度失败 ≥ 3）
        let mut alerts: Vec<(String, u32)> = Vec::new();
        for (dim, &count) in &state.failure_dimensions {
            if count >= 3 {
                alerts.push((dim.clone(), count));
            }
        }
        drop(state);

        for (dimension, count) in alerts {
            tracing::warn!(
                book_id = %book_id,
                dimension = %dimension,
                count = count,
                "Scheduler: 诊断告警（维度失败聚类）"
            );
            self.emit_event(SchedulerEvent::DiagnosticAlert {
                book_id: book_id.to_string(),
                chapter: chapter_number,
                dimension,
                failure_count: count,
            });
        }

        let gates = &self.config.quality_gates;
        if failures <= gates.max_audit_retries {
            tracing::warn!(
                book_id = %book_id,
                failures = failures,
                max_retries = gates.max_audit_retries,
                "Scheduler: 审计失败，将重试"
            );
            return;
        }

        // 达到暂停阈值
        if failures >= gates.pause_after_consecutive_failures {
            let mut paused = self.paused_books.write().await;
            paused.insert(book_id.to_string());
            let reason = format!(
                "连续 {} 次审计失败（阈值: {}）",
                failures, gates.pause_after_consecutive_failures
            );
            tracing::error!(book_id = %book_id, reason = %reason, "Scheduler: 书籍已暂停");
            self.emit_event(SchedulerEvent::Paused {
                book_id: book_id.to_string(),
                reason,
            });
        }
    }

    // ── 内部：每日上限 ───────────────────────────────────────

    /// 检查是否达到每日上限
    async fn is_daily_cap_reached(&self) -> bool {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let count = self.daily_chapter_count.lock().await;
        let current = count.get(&today).copied().unwrap_or(0);
        current >= self.config.max_chapters_per_day
    }

    /// 记录一章已写
    async fn record_chapter_written(&self) {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let mut count = self.daily_chapter_count.lock().await;
        let current = count.entry(today.clone()).or_insert(0);
        *current += 1;

        // 清理旧日期
        let old_keys: Vec<String> = count.keys().filter(|k| *k != &today).cloned().collect();
        for key in old_keys {
            count.remove(&key);
        }
    }

    // ── 内部：失败计数工具 ───────────────────────────────────

    async fn get_consecutive_failures(&self, book_id: &str) -> u32 {
        let book_state = self
            .book_states
            .entry(book_id.to_string())
            .or_insert_with(|| Mutex::new(BookRuntimeState::new()));
        let state = book_state.lock().await;
        state.consecutive_failures
    }

    async fn reset_consecutive_failures(&self, book_id: &str) {
        if let Some(book_state) = self.book_states.get(book_id) {
            let mut state = book_state.lock().await;
            state.consecutive_failures = 0;
            state.failure_dimensions.clear();
        }
    }

    // ── 内部：雷达扫描 ───────────────────────────────────────

    /// 运行雷达扫描
    async fn run_radar_scan(&self, engine: Arc<AgentEngine>) {
        // 复用 radar domain 的 scan 函数
        use crate::domain::radar::agent;

        match agent::scan(&engine, None).await {
            Ok(outcome) => {
                tracing::info!(
                    recommendations = outcome.result.recommendations.len(),
                    "Scheduler: 雷达扫描完成"
                );
            }
            Err(e) => {
                tracing::error!(error = %e, "Scheduler: 雷达扫描失败");
                self.emit_event(SchedulerEvent::Error {
                    book_id: "radar".to_string(),
                    message: e.to_string(),
                });
            }
        }
    }

    // ── 内部：事件分发 ───────────────────────────────────────

    fn emit_event(&self, event: SchedulerEvent) {
        if let Some(callback) = &self.on_event {
            callback(event);
        }
    }
}

// ── 辅助：cron 解析 ──────────────────────────────────────────

enum SchedulerKind {
    Write,
    Radar,
}

/// 生成一个定时循环任务（自由函数，接收 Arc<Scheduler> 以满足 Send 约束）
fn spawn_interval_task(
    name: &str,
    interval_ms: u64,
    engine: Arc<AgentEngine>,
    scheduler: Arc<Scheduler>,
    kind: SchedulerKind,
) -> JoinHandle<()> {
    let name = name.to_string();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(interval_ms.max(1000)));
        interval.tick().await; // 跳过首次立即触发
        loop {
            interval.tick().await;
            if !*scheduler.running.read().await {
                break;
            }
            match kind {
                SchedulerKind::Write => {
                    scheduler.trigger_write_cycle(engine.clone()).await;
                }
                SchedulerKind::Radar => {
                    scheduler.trigger_radar_scan(engine.clone()).await;
                }
            }
        }
        tracing::debug!(task = %name, "Scheduler: 定时任务已退出");
    })
}

/// 将 cron 表达式转换为毫秒间隔（简化版）
fn cron_to_ms(cron: &str) -> u64 {
    let parts: Vec<&str> = cron.split_whitespace().collect();
    if parts.len() < 5 {
        return 24 * 60 * 60 * 1000; // 默认 1 天
    }

    let minute = parts[0];
    let hour = parts[1];

    // "*/N * * * *" → 每 N 分钟
    if let Some(num_str) = minute.strip_prefix("*/") {
        if let Ok(interval) = num_str.parse::<u64>() {
            return interval * 60 * 1000;
        }
    }

    // "0 */N * * *" → 每 N 小时
    if let Some(num_str) = hour.strip_prefix("*/") {
        if let Ok(interval) = num_str.parse::<u64>() {
            return interval * 60 * 60 * 1000;
        }
    }

    // 固定时间 → 每日
    24 * 60 * 60 * 1000
}

// ── 调度器状态（用于 Tauri State 管理）────────────────────────

/// 调度器运行时状态（由 Tauri State 持有）
pub struct SchedulerState {
    inner: Arc<Scheduler>,
    engine: Arc<AgentEngine>,
}

impl SchedulerState {
    /// 创建调度器状态
    pub fn new(pipeline_config: PipelineConfig, config: SchedulerConfig, engine: AgentEngine) -> Self {
        Self {
            inner: Arc::new(Scheduler::new(pipeline_config, config)),
            engine: Arc::new(engine),
        }
    }

    /// 获取调度器引用
    pub fn scheduler(&self) -> &Arc<Scheduler> {
        &self.inner
    }

    /// 获取引擎引用
    pub fn engine(&self) -> &Arc<AgentEngine> {
        &self.engine
    }

    /// 启动调度器
    pub async fn start(&self) -> Result<(), AppError> {
        self.inner.start(self.engine.clone()).await
    }

    /// 停止调度器
    pub async fn stop(&self) {
        self.inner.stop().await
    }

    /// 是否正在运行
    pub async fn is_running(&self) -> bool {
        self.inner.is_running().await
    }
}

// ── 单元测试 ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cron_to_ms_every_n_minutes() {
        assert_eq!(cron_to_ms("*/5 * * * *"), 5 * 60 * 1000);
        assert_eq!(cron_to_ms("*/30 * * * *"), 30 * 60 * 1000);
    }

    #[test]
    fn cron_to_ms_every_n_hours() {
        assert_eq!(cron_to_ms("0 */6 * * *"), 6 * 60 * 60 * 1000);
        assert_eq!(cron_to_ms("0 */12 * * *"), 12 * 60 * 60 * 1000);
    }

    #[test]
    fn cron_to_ms_fixed_time_defaults_to_daily() {
        assert_eq!(cron_to_ms("0 9 * * *"), 24 * 60 * 60 * 1000);
    }

    #[test]
    fn cron_to_ms_invalid_defaults_to_daily() {
        assert_eq!(cron_to_ms("invalid"), 24 * 60 * 60 * 1000);
        assert_eq!(cron_to_ms("*/5"), 24 * 60 * 60 * 1000);
    }

    #[test]
    fn quality_gates_default() {
        let gates = QualityGates::default();
        assert_eq!(gates.max_audit_retries, 2);
        assert_eq!(gates.pause_after_consecutive_failures, 3);
        assert!((gates.retry_temperature_step - 0.1).abs() < 0.001);
    }

    #[test]
    fn scheduler_config_default() {
        let config = SchedulerConfig::default();
        assert_eq!(config.max_concurrent_books, 1);
        assert_eq!(config.chapters_per_cycle, 1);
        assert_eq!(config.max_chapters_per_day, 20);
        assert_eq!(config.retry_delay_ms, 5000);
        assert_eq!(config.cooldown_after_chapter_ms, 10000);
    }

    #[test]
    fn book_runtime_state_new_initializes_zero() {
        let state = BookRuntimeState::new();
        assert_eq!(state.consecutive_failures, 0);
        assert!(state.failure_dimensions.is_empty());
    }

    #[test]
    fn scheduler_event_serializes_chapter_complete() {
        let event = SchedulerEvent::ChapterComplete {
            book_id: "book-1".to_string(),
            chapter: 5,
            status: "ready-for-review".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"chapterComplete\""), "json was: {}", json);
        assert!(json.contains("\"bookId\":\"book-1\""), "json was: {}", json);
        assert!(json.contains("\"chapter\":5"), "json was: {}", json);
    }

    #[test]
    fn scheduler_event_serializes_paused() {
        let event = SchedulerEvent::Paused {
            book_id: "book-2".to_string(),
            reason: "连续 3 次审计失败".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"paused\""), "json was: {}", json);
        assert!(json.contains("\"bookId\":\"book-2\""), "json was: {}", json);
    }

    #[test]
    fn scheduler_event_serializes_diagnostic_alert() {
        let event = SchedulerEvent::DiagnosticAlert {
            book_id: "book-3".to_string(),
            chapter: 10,
            dimension: "OOC检查".to_string(),
            failure_count: 4,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"diagnosticAlert\""), "json was: {}", json);
        assert!(json.contains("OOC检查"), "json was: {}", json);
        assert!(json.contains("\"failureCount\":4"), "json was: {}", json);
    }
}
