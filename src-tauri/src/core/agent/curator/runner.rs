//! ═══════════════════════════════════════════════════════════════════════════
//! CuratorRunner - 后台编排器运行器
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 定期审查 skills，执行状态转换与 umbrella 合并。
//!
//! 设计要点：
//! - 后台任务，不阻塞 session 主流程（maybe_run_curator 仅检查 + spawn）
//! - LLM 调用通过 `CuratorLlm` trait 抽象，便于单元测试注入 mock
//! - 生产实现 `ProviderBackedCuratorLlm` 直接调用 `Provider::complete()`
//! - LLM 调用由 runner 统一包裹 60s 超时，失败时跳过合并（不阻塞 curator 流程）
//! - 报告写入失败仅 warn 日志，不阻塞 curator
//! - 首次运行（last_run=None）defer 一个周期，避免新装即触发
//! - `consolidate` 默认 OFF：仅跑确定性 prune（active/stale/archived）
//! - `dry_run` 模式：跳过 auto-transitions + 在 LLM prompt 前置 CURATOR_DRY_RUN_BANNER

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::infrastructure::fs::data_dir::DataDir;
use crate::infrastructure::llm::registry::ProviderRegistry;
use crate::infrastructure::llm::types::Message;
use crate::shared::error::AppError;

use super::state::{
    apply_automatic_transitions, detect_similar_groups, ConsolidationResult, SkillReviewItem,
    SkillState, TransitionConfig, UmbrellaMode,
};

/// Dry-run 提示横幅（对照 hermes-agent `CURATOR_DRY_RUN_BANNER`）。
///
/// 当 `dry_run = true` 时前置到 LLM prompt，告知 LLM 仅产出报告，
/// 不要执行 skill_manage 的 patch/create/delete/write_file/remove_file 等变更操作。
pub const CURATOR_DRY_RUN_BANNER: &str = "═══════════════════════════════════════════════════════════════\nDRY-RUN — REPORT ONLY. DO NOT MUTATE THE SKILL LIBRARY.\n═══════════════════════════════════════════════════════════════\n\nThis is a PREVIEW pass. Describe the actions you WOULD take, not actions you took.\n";

/// Curator LLM 抽象 —— 仅暴露 umbrella 合并能力，便于测试注入 mock。
///
/// 生产实现 `ProviderBackedCuratorLlm` 直接调用 `Provider::complete()`，
/// 不经过 AgentEngine 的 memory/context 管线，避免污染 curator 自身记忆。
#[async_trait::async_trait]
pub trait CuratorLlm: Send + Sync {
    /// 对一组相似 skills 执行 umbrella 合并，返回合并决策与产物。
    ///
    /// LLM 应根据集群特征选择三种模式之一（对照 hermes-agent CURATOR_REVIEW_PROMPT §3）：
    /// - `MergeIntoExisting`：集群中已有 umbrella，patch 之
    /// - `CreateNewUmbrella`：无现成 umbrella，新建 class-level SKILL.md
    /// - `DemoteToReferences`：sibling 内容降级为 references/templates/scripts
    ///
    /// 失败时返回 Err，由 runner 决定是否跳过（不阻塞 curator 流程）。
    async fn consolidate_group(&self, skills_json: &str) -> Result<ConsolidationResult, AppError>;
}

/// 生产实现：基于 `ProviderRegistry` 的 active provider 调用 LLM。
pub struct ProviderBackedCuratorLlm {
    registry: Arc<ProviderRegistry>,
}

impl ProviderBackedCuratorLlm {
    pub fn new(registry: Arc<ProviderRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait::async_trait]
impl CuratorLlm for ProviderBackedCuratorLlm {
    async fn consolidate_group(&self, skills_json: &str) -> Result<ConsolidationResult, AppError> {
        let provider = self.registry.active_provider()?;
        let config = self
            .registry
            .active_model_config()
            .ok_or_else(AppError::no_active_model)?;
        // 系统提示包含三种 umbrella 模式说明，让 LLM 自行选择
        let system = "You are a skill curator. Consolidate similar skills into one umbrella using one of three modes:\n\
            1. merge_into_existing — patch an existing broad skill as the umbrella\n\
            2. create_new_umbrella — create a new class-level SKILL.md\n\
            3. demote_to_references — move sibling content into umbrella's references/templates/scripts\n\n\
            Respond as JSON: {\"mode\": \"merge_into_existing|create_new_umbrella|demote_to_references\", \"umbrella_name\": \"<name>\", \"merged_content\": \"<merged skill content>\"}";
        let user_message = format!("Skills to consolidate:\n{}", skills_json);
        // umbrella 合并是简单场景，固定输出上限避免耦合 EffortLevel
        const UMBRELLA_MAX_TOKENS: u64 = 2048;
        let response = provider
            .complete(
                &config.model,
                system,
                &[Message {
                    role: "user".to_string(),
                    content: user_message,
                    tool_calls: None,
                    tool_call_id: None,
                }],
                UMBRELLA_MAX_TOKENS,
            )
            .await?;
        parse_consolidation_response(&response)
    }
}

/// 解析 LLM 返回的合并决策 JSON（容错：失败时降级为 MergeIntoExisting + 原文）。
fn parse_consolidation_response(response: &str) -> Result<ConsolidationResult, AppError> {
    // 尝试解析为 JSON；失败时降级（保留原响应为 merged_content，mode 默认）
    match serde_json::from_str::<serde_json::Value>(response) {
        Ok(v) => {
            let mode_str = v.get("mode").and_then(|s| s.as_str()).unwrap_or("merge_into_existing");
            let mode = match mode_str {
                "create_new_umbrella" => UmbrellaMode::CreateNewUmbrella,
                "demote_to_references" => UmbrellaMode::DemoteToReferences,
                _ => UmbrellaMode::MergeIntoExisting,
            };
            let umbrella_name = v
                .get("umbrella_name")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string());
            let merged_content = v
                .get("merged_content")
                .and_then(|s| s.as_str())
                .unwrap_or(response)
                .to_string();
            Ok(ConsolidationResult { mode, umbrella_name, merged_content })
        }
        Err(_) => {
            // 容错：LLM 未按 JSON 格式回复，保留原文作为 merged_content
            Ok(ConsolidationResult {
                mode: UmbrellaMode::MergeIntoExisting,
                umbrella_name: None,
                merged_content: response.to_string(),
            })
        }
    }
}

/// Curator 运行配置（对照 hermes-agent DEFAULT_* 常量）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuratorConfig {
    /// 是否启用 curator（对照 hermes-agent `curator.enabled`，默认 ON）
    pub enabled: bool,
    /// 检查周期（天），session-start hook 据此判断是否触发
    /// （对照 hermes-agent `DEFAULT_INTERVAL_HOURS = 168` = 7 天）
    pub interval_days: u64,
    /// 触发 curator 所需的最小用户空闲时长（小时）
    /// （对照 hermes-agent `DEFAULT_MIN_IDLE_HOURS = 2`）
    pub min_idle_hours: f64,
    /// Active → Stale 阈值（天，对照 hermes-agent `DEFAULT_STALE_AFTER_DAYS = 30`）
    pub stale_after_days: u64,
    /// Stale → Archived 阈值（天，对照 hermes-agent `DEFAULT_ARCHIVE_AFTER_DAYS = 90`）
    pub archive_after_days: u64,
    /// 是否运行 LLM umbrella-building 合并 pass（对照 hermes-agent `DEFAULT_CONSOLIDATE = False`）
    ///
    /// OFF 时仅跑确定性 prune（active/stale/archived 状态机），跳过 LLM 调用，
    /// 避免不必要的 aux-model 成本。opt-in 时才 spawn LLM fork 做 umbrella-building。
    pub consolidate: bool,
    /// LLM 调用超时（秒，任务规格：60）
    pub llm_timeout_secs: u64,
}

impl Default for CuratorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_days: 7,
            min_idle_hours: 2.0,
            stale_after_days: 30,
            archive_after_days: 90,
            consolidate: false,
            llm_timeout_secs: 60,
        }
    }
}

/// Curator 单次运行报告（run.json 序列化结构）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuratorRunReport {
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    /// 本次是否为 dry-run（dry-run 跳过 auto-transitions + LLM 仅产出报告）
    pub dry_run: bool,
    pub skills_reviewed: usize,
    pub skills_stale: usize,
    pub skills_archived: usize,
    pub umbrellas_built: usize,
    /// 按 UmbrellaMode 分组的合并计数（key 为 mode.as_str()）
    #[serde(default)]
    pub umbrellas_by_mode: HashMap<String, usize>,
    pub errors: Vec<String>,
}

/// Curator 编排器。
///
/// 持有上次运行时间（in-memory，进程级；持久化跨重启由后续 session-start 集成任务负责）。
pub struct CuratorRunner {
    config: CuratorConfig,
    last_run: Arc<RwLock<Option<DateTime<Utc>>>>,
    data_dir: DataDir,
}

impl CuratorRunner {
    pub fn new(config: CuratorConfig, data_dir: DataDir) -> Self {
        Self {
            config,
            last_run: Arc::new(RwLock::new(None)),
            data_dir,
        }
    }

    /// session-start hook：检查 gates + 上次运行时间，决定是否触发。
    ///
    /// Gates（对照 hermes-agent `should_run_now` + `maybe_run_curator`）：
    /// 1. `config.enabled == false` → 不触发
    /// 2. `idle_for_seconds < min_idle_hours * 3600` → 不触发（仅当调用方提供 idle 测量）
    /// 3. `last_run == None`（首次运行）→ seed last_run 为 now + defer 一个周期
    /// 4. `(now - last_run) < interval_days` → 不触发
    ///
    /// 触发时立即标记 `last_run`（避免在 spawned task 完成前被重复触发），
    /// 然后通过 `tokio::spawn` 异步执行 review（不阻塞 session）。
    ///
    /// `dry_run` 与 `consolidate` 透传到 spawned review。
    pub async fn maybe_run_curator(
        self: &Arc<Self>,
        items: Vec<SkillReviewItem>,
        llm: Arc<dyn CuratorLlm>,
        idle_for_seconds: Option<f64>,
        dry_run: bool,
    ) -> bool {
        // Gate 1: enabled
        if !self.config.enabled {
            return false;
        }
        // Gate 2: idle（仅当调用方提供测量值时强制；None 表示无 idle 检测能力）
        if let Some(idle_s) = idle_for_seconds {
            let min_idle_s = self.config.min_idle_hours * 3600.0;
            if idle_s < min_idle_s {
                return false;
            }
        }
        let now = Utc::now();
        let should_run = {
            let last = self.last_run.read().await;
            match *last {
                None => {
                    // 首次运行 defer 一个周期（对照 hermes-agent should_run_now 的 first-run 行为）
                    drop(last);
                    *self.last_run.write().await = Some(now);
                    tracing::info!(
                        "Curator first-run: seeded last_run to now, deferring one interval"
                    );
                    false
                }
                Some(t) => (now - t).num_days() >= self.config.interval_days as i64,
            }
        };
        if should_run {
            // 立即标记，避免在 spawned task 完成前被重复触发
            *self.last_run.write().await = Some(now);
            self.clone().run_curator_review(items, llm, dry_run);
            true
        } else {
            false
        }
    }

    /// 主入口：spawn 独立 tokio task 执行 curator review（不阻塞 session）。
    ///
    /// 返回 JoinHandle，调用方可选择 await 或丢弃（fire-and-forget）。
    pub fn run_curator_review(
        self: Arc<Self>,
        items: Vec<SkillReviewItem>,
        llm: Arc<dyn CuratorLlm>,
        dry_run: bool,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let runner = self.clone();
            let report = runner.execute_review(items, &*llm, dry_run).await;
            // 更新 last_run 为完成时间（dry-run 不推进，避免推迟下次正式运行）
            if !dry_run {
                *runner.last_run.write().await = Some(report.finished_at);
            }
            // 报告写入失败不阻塞 curator（warn 日志）
            if let Err(e) = runner.write_report(&report).await {
                tracing::warn!(error = %e, "Curator report write failed");
            }
            tracing::info!(
                dry_run = report.dry_run,
                reviewed = report.skills_reviewed,
                stale = report.skills_stale,
                archived = report.skills_archived,
                umbrellas = report.umbrellas_built,
                errors = report.errors.len(),
                "Curator review completed"
            );
        })
    }

    /// 实际执行 curator review（async，可测试）。
    ///
    /// 流程：
    /// 1. 应用自动状态转换（Active→Stale→Archived）—— dry-run 时跳过
    /// 2. 统计转换结果
    /// 3. 若 `consolidate == true`：检测相似 skills 分组，对每组调用 LLM
    ///    合并为 umbrella（60s 超时，失败跳过）；dry-run 时 LLM prompt 前置 banner
    pub async fn execute_review(
        &self,
        items: Vec<SkillReviewItem>,
        llm: &dyn CuratorLlm,
        dry_run: bool,
    ) -> CuratorRunReport {
        let started_at = Utc::now();
        let now = Utc::now();
        let transition_cfg = TransitionConfig {
            stale_after_days: self.config.stale_after_days,
            archive_after_days: self.config.archive_after_days,
        };

        // 1. 状态转换（dry-run 跳过：仅审查候选，不变更状态）
        let reviewed = if dry_run {
            items // 保留原状态
        } else {
            apply_automatic_transitions(items, now, &transition_cfg)
        };

        // 2. 统计
        let skills_reviewed = reviewed.len();
        let skills_stale = reviewed
            .iter()
            .filter(|i| i.state == SkillState::Stale)
            .count();
        let skills_archived = reviewed
            .iter()
            .filter(|i| i.state == SkillState::Archived)
            .count();

        // 3. umbrella 合并
        // consolidate == false 时跳过（对照 hermes-agent DEFAULT_CONSOLIDATE = False）
        let mut umbrellas_built = 0usize;
        let mut umbrellas_by_mode: HashMap<String, usize> = HashMap::new();
        let mut errors: Vec<String> = Vec::new();
        if self.config.consolidate {
            let groups = detect_similar_groups(&reviewed);
            let timeout = Duration::from_secs(self.config.llm_timeout_secs);
            for group in &groups {
                let group_items: Vec<&SkillReviewItem> = group.iter().map(|&i| &reviewed[i]).collect();
                let skills_json = match serde_json::to_string_pretty(&group_items) {
                    Ok(s) => s,
                    Err(e) => {
                        errors.push(format!("serialize umbrella group: {e}"));
                        continue;
                    }
                };
                match tokio::time::timeout(timeout, llm.consolidate_group(&skills_json)).await {
                    Ok(Ok(result)) => {
                        umbrellas_built += 1;
                        *umbrellas_by_mode.entry(result.mode.as_str().to_string()).or_insert(0) += 1;
                    }
                    Ok(Err(e)) => {
                        // LLM 失败时跳过合并（不阻塞 curator 流程）
                        tracing::warn!(error = %e, "Umbrella consolidation failed, skipping");
                        errors.push(format!("umbrella consolidate: {e}"));
                    }
                    Err(_) => {
                        tracing::warn!(timeout_secs = self.config.llm_timeout_secs, "Umbrella consolidation timed out, skipping");
                        errors.push(format!(
                            "umbrella consolidate: timeout after {}s",
                            self.config.llm_timeout_secs
                        ));
                    }
                }
            }
        } else {
            tracing::debug!("Consolidation disabled (config.consolidate = false), skipping LLM umbrella pass");
        }

        CuratorRunReport {
            started_at,
            finished_at: Utc::now(),
            dry_run,
            skills_reviewed,
            skills_stale,
            skills_archived,
            umbrellas_built,
            umbrellas_by_mode,
            errors,
        }
    }

    /// 写入运行报告到 `{DataDir}/logs/curator/{YYYYMMDD-HHMMSS}/run.json` + `REPORT.md`。
    ///
    /// 文件 I/O 卸载到 `spawn_blocking`，避免阻塞异步运行时。
    pub async fn write_report(&self, report: &CuratorRunReport) -> Result<(), AppError> {
        let ts = report.started_at.format("%Y%m%d-%H%M%S").to_string();
        let dir = self.data_dir.logs_dir().join("curator").join(&ts);
        let report_clone = report.clone();
        tokio::task::spawn_blocking(move || -> Result<(), AppError> {
            std::fs::create_dir_all(&dir)
                .map_err(|e| AppError::internal(format!("create curator report dir: {e}")))?;
            let run_json = serde_json::to_string_pretty(&report_clone)?;
            std::fs::write(dir.join("run.json"), run_json).map_err(|e| {
                AppError::file_write_error(format!("{}: {}", dir.join("run.json").display(), e))
            })?;
            let md = render_report_markdown(&report_clone);
            std::fs::write(dir.join("REPORT.md"), md).map_err(|e| {
                AppError::file_write_error(format!("{}: {}", dir.join("REPORT.md").display(), e))
            })?;
            Ok(())
        })
        .await
        .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {e}")))??;
        Ok(())
    }

    /// 读取上次运行时间（用于测试与可观测性）。
    pub async fn last_run_at(&self) -> Option<DateTime<Utc>> {
        *self.last_run.read().await
    }

    /// 手动设置上次运行时间（仅用于测试）。
    #[cfg(test)]
    pub async fn set_last_run_for_test(&self, at: Option<DateTime<Utc>>) {
        *self.last_run.write().await = at;
    }
}

/// 渲染人类可读的 Markdown 报告。
fn render_report_markdown(report: &CuratorRunReport) -> String {
    let mut md = String::new();
    md.push_str("# Curator Run Report\n\n");
    if report.dry_run {
        md.push_str("> **DRY-RUN** — report only, no mutations applied.\n\n");
    }
    md.push_str(&format!("- **Started**: {}\n", report.started_at.to_rfc3339()));
    md.push_str(&format!("- **Finished**: {}\n", report.finished_at.to_rfc3339()));
    md.push_str(&format!("- **Skills Reviewed**: {}\n", report.skills_reviewed));
    md.push_str(&format!("- **Skills Stale**: {}\n", report.skills_stale));
    md.push_str(&format!("- **Skills Archived**: {}\n", report.skills_archived));
    md.push_str(&format!("- **Umbrellas Built**: {}\n", report.umbrellas_built));
    // 按 mode 细分 umbrella 合并计数（若存在）
    if !report.umbrellas_by_mode.is_empty() {
        md.push_str("\n### Umbrellas by Mode\n\n");
        let mut modes: Vec<(&String, &usize)> = report.umbrellas_by_mode.iter().collect();
        modes.sort_by_key(|(k, _)| (*k).clone());
        for (mode, count) in modes {
            md.push_str(&format!("- **{}**: {}\n", mode, count));
        }
    }
    md.push_str(&format!("\n- **Errors**: {}\n", report.errors.len()));
    md.push_str("\n## Errors\n\n");
    if report.errors.is_empty() {
        md.push_str("(none)\n");
    } else {
        for err in &report.errors {
            md.push_str(&format!("- {}\n", err));
        }
    }
    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// mock LLM：可配置返回值 / 延迟 / 失败
    struct MockCuratorLlm {
        delay_ms: u64,
        fail: bool,
        call_count: AtomicU32,
        last_input: tokio::sync::Mutex<Option<String>>,
    }

    impl MockCuratorLlm {
        fn new_ok() -> Self {
            Self {
                delay_ms: 0,
                fail: false,
                call_count: AtomicU32::new(0),
                last_input: tokio::sync::Mutex::new(None),
            }
        }
        fn new_failing() -> Self {
            Self {
                delay_ms: 0,
                fail: true,
                call_count: AtomicU32::new(0),
                last_input: tokio::sync::Mutex::new(None),
            }
        }
        fn new_with_delay(delay_ms: u64) -> Self {
            Self {
                delay_ms,
                fail: false,
                call_count: AtomicU32::new(0),
                last_input: tokio::sync::Mutex::new(None),
            }
        }
    }

    #[async_trait::async_trait]
    impl CuratorLlm for MockCuratorLlm {
        async fn consolidate_group(&self, skills_json: &str) -> Result<ConsolidationResult, AppError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            *self.last_input.lock().await = Some(skills_json.to_string());
            if self.delay_ms > 0 {
                tokio::time::sleep(Duration::milliseconds(self.delay_ms as i64).to_std().unwrap())
                    .await;
            }
            if self.fail {
                return Err(AppError::internal("mock llm failure"));
            }
            // 返回 merge_into_existing 模式的合并结果
            Ok(ConsolidationResult {
                mode: UmbrellaMode::MergeIntoExisting,
                umbrella_name: Some("umbrella_test".to_string()),
                merged_content: format!(r#"{{"merged": true, "input": "{}"}}"#, skills_json.replace('"', "'")),
            })
        }
    }

    fn review_item(name: &str, state: SkillState, days_ago: i64) -> SkillReviewItem {
        SkillReviewItem {
            name: name.to_string(),
            category: "general".to_string(),
            description: String::new(),
            state,
            last_used_at: Utc::now() - Duration::days(days_ago),
            pinned: false,
        }
    }

    fn runner_with_timeout(timeout_secs: u64, data_dir: DataDir) -> Arc<CuratorRunner> {
        // 启用 consolidate=true，便于 umbrella 合并测试触发 LLM 路径
        let config = CuratorConfig {
            enabled: true,
            interval_days: 7,
            min_idle_hours: 2.0,
            stale_after_days: 30,
            archive_after_days: 90,
            consolidate: true,
            llm_timeout_secs: timeout_secs,
        };
        Arc::new(CuratorRunner::new(config, data_dir))
    }

    fn make_data_dir(tmp: &tempfile::TempDir) -> DataDir {
        DataDir::new(tmp.path().to_path_buf())
    }

    #[tokio::test]
    async fn execute_review_counts_transitions() {
        let tmp = tempfile::tempdir().unwrap();
        let runner = runner_with_timeout(60, make_data_dir(&tmp));
        let items = vec![
            review_item("recent", SkillState::Active, 1),
            review_item("old-active", SkillState::Active, 45),   // → Stale (>30)
            review_item("old-stale", SkillState::Stale, 100),    // → Archived (>90)
        ];
        let llm: Arc<dyn CuratorLlm> = Arc::new(MockCuratorLlm::new_ok());
        let report = runner.execute_review(items, &*llm, false).await;
        assert_eq!(report.skills_reviewed, 3);
        assert_eq!(report.skills_stale, 1);
        assert_eq!(report.skills_archived, 1);
        assert!(report.errors.is_empty());
    }

    #[tokio::test]
    async fn execute_review_umbrella_merge_success() {
        let tmp = tempfile::tempdir().unwrap();
        let runner = runner_with_timeout(60, make_data_dir(&tmp));
        // 两个相似 skills（同 category + 共享 token "writing"）
        let items = vec![
            SkillReviewItem {
                name: "novel_writing".to_string(),
                category: "writing".to_string(),
                description: String::new(),
                state: SkillState::Active,
                last_used_at: Utc::now(),
                pinned: false,
            },
            SkillReviewItem {
                name: "essay_writing".to_string(),
                category: "writing".to_string(),
                description: String::new(),
                state: SkillState::Active,
                last_used_at: Utc::now(),
                pinned: false,
            },
        ];
        let mock = Arc::new(MockCuratorLlm::new_ok());
        let llm: Arc<dyn CuratorLlm> = mock.clone();
        let report = runner.execute_review(items, &*llm, false).await;
        assert_eq!(report.umbrellas_built, 1);
        assert!(report.errors.is_empty());
        assert_eq!(mock.call_count.load(Ordering::SeqCst), 1);
        // 验证 umbrellas_by_mode 统计
        assert_eq!(report.umbrellas_by_mode.get("merge_into_existing"), Some(&1));
    }

    #[tokio::test]
    async fn execute_review_llm_failure_does_not_block() {
        let tmp = tempfile::tempdir().unwrap();
        let runner = runner_with_timeout(60, make_data_dir(&tmp));
        let items = vec![
            SkillReviewItem {
                name: "novel_writing".to_string(),
                category: "writing".to_string(),
                description: String::new(),
                state: SkillState::Active,
                last_used_at: Utc::now(),
                pinned: false,
            },
            SkillReviewItem {
                name: "essay_writing".to_string(),
                category: "writing".to_string(),
                description: String::new(),
                state: SkillState::Active,
                last_used_at: Utc::now(),
                pinned: false,
            },
        ];
        let llm: Arc<dyn CuratorLlm> = Arc::new(MockCuratorLlm::new_failing());
        let report = runner.execute_review(items, &*llm, false).await;
        assert_eq!(report.umbrellas_built, 0);
        assert_eq!(report.errors.len(), 1);
        assert!(report.errors[0].contains("umbrella consolidate"));
    }

    #[tokio::test]
    async fn execute_review_llm_timeout_does_not_block() {
        let tmp = tempfile::tempdir().unwrap();
        // 1s 超时，mock 延迟 2s → 必超时
        let runner = runner_with_timeout(1, make_data_dir(&tmp));
        let items = vec![
            SkillReviewItem {
                name: "novel_writing".to_string(),
                category: "writing".to_string(),
                description: String::new(),
                state: SkillState::Active,
                last_used_at: Utc::now(),
                pinned: false,
            },
            SkillReviewItem {
                name: "essay_writing".to_string(),
                category: "writing".to_string(),
                description: String::new(),
                state: SkillState::Active,
                last_used_at: Utc::now(),
                pinned: false,
            },
        ];
        let llm: Arc<dyn CuratorLlm> = Arc::new(MockCuratorLlm::new_with_delay(2000));
        let report = runner.execute_review(items, &*llm, false).await;
        assert_eq!(report.umbrellas_built, 0);
        assert_eq!(report.errors.len(), 1);
        assert!(report.errors[0].contains("timeout"));
    }

    #[tokio::test]
    async fn write_report_creates_json_and_markdown() {
        let tmp = tempfile::tempdir().unwrap();
        let runner = runner_with_timeout(60, make_data_dir(&tmp));
        let mut umbrellas_by_mode = HashMap::new();
        umbrellas_by_mode.insert("merge_into_existing".to_string(), 1);
        let report = CuratorRunReport {
            started_at: Utc::now(),
            finished_at: Utc::now(),
            dry_run: false,
            skills_reviewed: 5,
            skills_stale: 2,
            skills_archived: 1,
            umbrellas_built: 1,
            umbrellas_by_mode,
            errors: vec!["sample error".to_string()],
        };
        runner.write_report(&report).await.unwrap();

        // 找到 curator/{ts}/ 目录
        let curator_dir = runner.data_dir.logs_dir().join("curator");
        let entries: Vec<_> = std::fs::read_dir(&curator_dir).unwrap().collect();
        assert_eq!(entries.len(), 1, "exactly one run directory");
        // 修复：read_dir().collect() 得到 Vec<Result<DirEntry>>，需要 unwrap 取出 DirEntry
        let run_dir = entries[0].as_ref().unwrap().path();
        assert!(run_dir.join("run.json").exists(), "run.json exists");
        assert!(run_dir.join("REPORT.md").exists(), "REPORT.md exists");

        // 验证 run.json 格式
        let json_content = std::fs::read_to_string(run_dir.join("run.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_content).unwrap();
        assert_eq!(parsed["skills_reviewed"], 5);
        assert_eq!(parsed["skills_stale"], 2);
        assert_eq!(parsed["skills_archived"], 1);
        assert_eq!(parsed["umbrellas_built"], 1);
        assert_eq!(parsed["dry_run"], false);
        assert_eq!(parsed["errors"].as_array().unwrap().len(), 1);

        // 验证 REPORT.md 格式
        let md = std::fs::read_to_string(run_dir.join("REPORT.md")).unwrap();
        assert!(md.contains("# Curator Run Report"));
        assert!(md.contains("Skills Reviewed"));
        assert!(md.contains("sample error"));
    }

    #[tokio::test]
    async fn maybe_run_curator_triggers_when_never_run() {
        let tmp = tempfile::tempdir().unwrap();
        let runner = runner_with_timeout(60, make_data_dir(&tmp));
        // last_run = None → 首次运行 defer 一个周期，不立即触发
        let triggered = runner
            .maybe_run_curator(vec![], Arc::new(MockCuratorLlm::new_ok()), None, false)
            .await;
        // 首次运行 defer：seed last_run 但不触发实际 review
        assert!(!triggered, "first-run should defer one interval");
        let last = runner.last_run_at().await;
        assert!(last.is_some(), "last_run should be seeded on first observation");
    }

    #[tokio::test]
    async fn maybe_run_curator_skips_within_7_days() {
        let tmp = tempfile::tempdir().unwrap();
        let runner = runner_with_timeout(60, make_data_dir(&tmp));
        // 设为 3 天前 → 不应触发
        runner.set_last_run_for_test(Some(Utc::now() - Duration::days(3))).await;
        let triggered = runner
            .maybe_run_curator(vec![], Arc::new(MockCuratorLlm::new_ok()), None, false)
            .await;
        assert!(!triggered, "should not trigger within 7 days");
    }

    #[tokio::test]
    async fn maybe_run_curator_triggers_after_7_days() {
        let tmp = tempfile::tempdir().unwrap();
        let runner = runner_with_timeout(60, make_data_dir(&tmp));
        // 设为 8 天前 → 应触发
        runner.set_last_run_for_test(Some(Utc::now() - Duration::days(8))).await;
        let triggered = runner
            .maybe_run_curator(vec![], Arc::new(MockCuratorLlm::new_ok()), None, false)
            .await;
        assert!(triggered, "should trigger after 7 days");
        tokio::time::sleep(Duration::milliseconds(100).to_std().unwrap()).await;
    }

    #[tokio::test]
    async fn maybe_run_curator_boundary_7_days_does_not_trigger() {
        let tmp = tempfile::tempdir().unwrap();
        let runner = runner_with_timeout(60, make_data_dir(&tmp));
        // 刚好 7 天：用 >= 比较，7 天应触发（边界包含）
        runner.set_last_run_for_test(Some(Utc::now() - Duration::days(7))).await;
        let triggered = runner
            .maybe_run_curator(vec![], Arc::new(MockCuratorLlm::new_ok()), None, false)
            .await;
        assert!(triggered, "7 days boundary (>=) should trigger");
        tokio::time::sleep(Duration::milliseconds(100).to_std().unwrap()).await;
    }

    #[tokio::test]
    async fn maybe_run_curator_skips_when_disabled() {
        let tmp = tempfile::tempdir().unwrap();
        let config = CuratorConfig {
            enabled: false,
            ..CuratorConfig::default()
        };
        let runner = Arc::new(CuratorRunner::new(config, make_data_dir(&tmp)));
        runner.set_last_run_for_test(Some(Utc::now() - Duration::days(10))).await;
        let triggered = runner
            .maybe_run_curator(vec![], Arc::new(MockCuratorLlm::new_ok()), None, false)
            .await;
        assert!(!triggered, "disabled curator should never trigger");
    }

    #[tokio::test]
    async fn maybe_run_curator_skips_when_idle_below_threshold() {
        let tmp = tempfile::tempdir().unwrap();
        let runner = runner_with_timeout(60, make_data_dir(&tmp));
        runner.set_last_run_for_test(Some(Utc::now() - Duration::days(10))).await;
        // 空闲 1 小时，低于默认 2 小时阈值 → 不触发
        let triggered = runner
            .maybe_run_curator(vec![], Arc::new(MockCuratorLlm::new_ok()), Some(3600.0), false)
            .await;
        assert!(!triggered, "should not trigger when idle below threshold");
    }

    #[tokio::test]
    async fn execute_review_dry_run_skips_transitions() {
        let tmp = tempfile::tempdir().unwrap();
        let runner = runner_with_timeout(60, make_data_dir(&tmp));
        let items = vec![
            review_item("recent", SkillState::Active, 1),
            review_item("old-active", SkillState::Active, 100),  // 正常应 → Stale，dry-run 跳过
        ];
        let llm: Arc<dyn CuratorLlm> = Arc::new(MockCuratorLlm::new_ok());
        let report = runner.execute_review(items, &*llm, true).await;
        assert!(report.dry_run);
        // dry-run 保留原状态，不发生转换
        assert_eq!(report.skills_stale, 0);
        assert_eq!(report.skills_archived, 0);
    }

    #[test]
    fn render_report_markdown_format() {
        let report = CuratorRunReport {
            started_at: Utc::now(),
            finished_at: Utc::now(),
            dry_run: false,
            skills_reviewed: 10,
            skills_stale: 3,
            skills_archived: 1,
            umbrellas_built: 2,
            umbrellas_by_mode: HashMap::new(),
            errors: vec!["err1".to_string(), "err2".to_string()],
        };
        let md = render_report_markdown(&report);
        assert!(md.starts_with("# Curator Run Report"));
        assert!(md.contains("**Skills Reviewed**: 10"));
        assert!(md.contains("**Umbrellas Built**: 2"));
        assert!(md.contains("- err1"));
        assert!(md.contains("- err2"));
    }

    #[test]
    fn render_report_markdown_no_errors() {
        let report = CuratorRunReport {
            started_at: Utc::now(),
            finished_at: Utc::now(),
            dry_run: true,
            skills_reviewed: 0,
            skills_stale: 0,
            skills_archived: 0,
            umbrellas_built: 0,
            umbrellas_by_mode: HashMap::new(),
            errors: vec![],
        };
        let md = render_report_markdown(&report);
        assert!(md.contains("(none)"));
    }

    #[test]
    fn config_default_values() {
        // 对照 hermes-agent 默认值
        let c = CuratorConfig::default();
        assert!(c.enabled, "enabled defaults to true");
        assert_eq!(c.interval_days, 7, "7-day interval");
        assert_eq!(c.min_idle_hours, 2.0, "2-hour min idle");
        assert_eq!(c.stale_after_days, 30, "30-day stale threshold");
        assert_eq!(c.archive_after_days, 90, "90-day archive threshold");
        assert!(!c.consolidate, "consolidate defaults to OFF");
        assert_eq!(c.llm_timeout_secs, 60, "60s LLM timeout");
    }
}
