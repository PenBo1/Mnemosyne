//! 持续学习系统 — 技能生命周期管理 + 可插拔记忆提供者。
//!
//! 移植自 Hermes Agent 的 `agent/curator.py` 和 `agent/memory_provider.py`。
//! - SkillCurator: 追踪技能使用，自动归档过时技能
//! - MemoryProvider: 可插拔记忆提供者抽象
//! - MemoryManager: 编排记忆提供者，管理跨会话记忆

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use async_trait::async_trait;
use crate::errors::AppError;
use crate::infra::llm::types::Message;

// ── 技能状态 ──────────────────────────────────────────────────

/// 技能生命周期状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillState {
    /// 活跃 — 最近使用过
    Active,
    /// 过时 — 超过 `stale_after_days` 未使用
    Stale,
    /// 已归档 — 超过 `archive_after_days` 未使用
    Archived,
}

impl SkillState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Stale => "stale",
            Self::Archived => "archived",
        }
    }
}

// ── 技能使用遥测 ──────────────────────────────────────────────

/// 技能使用遥测数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUsage {
    /// 技能名称
    pub name: String,
    /// 使用次数
    pub use_count: u32,
    /// 查看次数
    pub view_count: u32,
    /// 修改次数
    pub patch_count: u32,
    /// 最后活动时间 (ISO 8601)
    pub last_activity_at: Option<String>,
    /// 当前状态
    pub state: SkillState,
    /// 是否已固定（免于自动归档）
    pub pinned: bool,
    /// 创建者: "agent" | "user" | "bundled"
    pub created_by: String,
}

impl SkillUsage {
    pub fn new(name: &str, created_by: &str) -> Self {
        Self {
            name: name.to_string(),
            use_count: 0,
            view_count: 0,
            patch_count: 0,
            last_activity_at: None,
            state: SkillState::Active,
            pinned: false,
            created_by: created_by.to_string(),
        }
    }

    /// 是否为 Agent 创建的技能（策展人只管理 Agent 创建的技能）
    pub fn is_agent_created(&self) -> bool {
        self.created_by == "agent"
    }
}

// ── 策展人配置 ──────────────────────────────────────────────────

/// 策展人配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuratorConfig {
    /// 是否启用（默认 true）
    pub enabled: bool,
    /// 运行间隔（小时，默认 168 = 7 天）
    pub interval_hours: u64,
    /// 最小空闲时间（小时，默认 2）
    pub min_idle_hours: u64,
    /// 多少天未使用标记为过时（默认 30）
    pub stale_after_days: u64,
    /// 多少天未使用自动归档（默认 90）
    pub archive_after_days: u64,
}

impl Default for CuratorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_hours: 168,
            min_idle_hours: 2,
            stale_after_days: 30,
            archive_after_days: 90,
        }
    }
}

/// 策展人状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuratorState {
    /// 上次运行时间
    pub last_run_at: Option<String>,
    /// 上次运行耗时（秒）
    pub last_run_duration_secs: Option<u64>,
    /// 上次运行摘要
    pub last_run_summary: Option<String>,
    /// 是否已暂停
    pub paused: bool,
    /// 运行次数
    pub run_count: u32,
}

impl Default for CuratorState {
    fn default() -> Self {
        Self {
            last_run_at: None,
            last_run_duration_secs: None,
            last_run_summary: None,
            paused: false,
            run_count: 0,
        }
    }
}

/// 技能状态转换记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTransition {
    pub skill_name: String,
    pub from: SkillState,
    pub to: SkillState,
    pub reason: String,
}

// ── 技能策展人 ──────────────────────────────────────────────────

/// 技能策展人 — 后台技能维护编排器
///
/// 职责：
/// - 根据使用时间戳自动转换生命周期状态
/// - 只处理 Agent 创建的技能（bundled + hub 安装的不受影响）
/// - 永不删除，最大破坏操作是归档
/// - 固定的技能免于所有自动转换
pub struct SkillCurator {
    /// 技能使用数据
    usage: HashMap<String, SkillUsage>,
    /// 配置
    config: CuratorConfig,
    /// 状态
    state: CuratorState,
}

impl SkillCurator {
    pub fn new(config: CuratorConfig) -> Self {
        Self {
            usage: HashMap::new(),
            config,
            state: CuratorState::default(),
        }
    }

    /// 记录技能使用
    pub fn record_use(&mut self, skill_name: &str) {
        let now = chrono::Utc::now().to_rfc3339();
        let entry = self.usage.entry(skill_name.to_string())
            .or_insert_with(|| SkillUsage::new(skill_name, "agent"));
        entry.use_count += 1;
        entry.last_activity_at = Some(now);
        if entry.state == SkillState::Stale || entry.state == SkillState::Archived {
            entry.state = SkillState::Active;
        }
    }

    /// 记录技能查看
    pub fn record_view(&mut self, skill_name: &str) {
        let entry = self.usage.entry(skill_name.to_string())
            .or_insert_with(|| SkillUsage::new(skill_name, "agent"));
        entry.view_count += 1;
    }

    /// 记录技能修改
    pub fn record_patch(&mut self, skill_name: &str) {
        let now = chrono::Utc::now().to_rfc3339();
        let entry = self.usage.entry(skill_name.to_string())
            .or_insert_with(|| SkillUsage::new(skill_name, "agent"));
        entry.patch_count += 1;
        entry.last_activity_at = Some(now);
    }

    /// 固定技能（免于自动归档）
    pub fn pin(&mut self, skill_name: &str) -> Result<(), AppError> {
        let entry = self.usage.get_mut(skill_name)
            .ok_or_else(|| AppError::not_found(format!("技能 '{}' 不存在", skill_name)))?;
        entry.pinned = true;
        Ok(())
    }

    /// 取消固定
    pub fn unpin(&mut self, skill_name: &str) -> Result<(), AppError> {
        let entry = self.usage.get_mut(skill_name)
            .ok_or_else(|| AppError::not_found(format!("技能 '{}' 不存在", skill_name)))?;
        entry.pinned = false;
        Ok(())
    }

    /// 应用自动状态转换
    ///
    /// 不变量：
    /// - 只处理 `created_by == "agent"` 的技能
    /// - 固定的技能跳过所有自动转换
    /// - 永不删除，只归档
    pub fn apply_automatic_transitions(&mut self) -> Vec<SkillTransition> {
        let now = chrono::Utc::now();
        let mut transitions = Vec::new();

        for (name, usage) in &mut self.usage {
            // 跳过非 Agent 创建的技能
            if !usage.is_agent_created() {
                continue;
            }
            // 跳过固定的技能
            if usage.pinned {
                continue;
            }
            // 跳过没有活动记录的技能
            let last_activity = match &usage.last_activity_at {
                Some(ts) => match chrono::DateTime::parse_from_rfc3339(ts) {
                    Ok(dt) => dt.with_timezone(&chrono::Utc),
                    Err(_) => continue,
                },
                None => continue,
            };

            let idle_days = (now - last_activity).num_days() as u64;

            match usage.state {
                SkillState::Active => {
                    if idle_days >= self.config.stale_after_days {
                        let from = usage.state;
                        usage.state = SkillState::Stale;
                        transitions.push(SkillTransition {
                            skill_name: name.clone(),
                            from,
                            to: SkillState::Stale,
                            reason: format!("{}天未使用，标记为过时", idle_days),
                        });
                    }
                }
                SkillState::Stale => {
                    if idle_days >= self.config.archive_after_days {
                        let from = usage.state;
                        usage.state = SkillState::Archived;
                        transitions.push(SkillTransition {
                            skill_name: name.clone(),
                            from,
                            to: SkillState::Archived,
                            reason: format!("{}天未使用，自动归档", idle_days),
                        });
                    }
                }
                SkillState::Archived => {}
            }
        }

        transitions
    }

    /// 检查策展人是否应该运行
    pub fn should_run(&self) -> bool {
        if !self.config.enabled || self.state.paused {
            return false;
        }
        match &self.state.last_run_at {
            Some(ts) => {
                if let Ok(last_run) = chrono::DateTime::parse_from_rfc3339(ts) {
                    let elapsed = chrono::Utc::now() - last_run.with_timezone(&chrono::Utc);
                    elapsed.num_hours() as u64 >= self.config.interval_hours
                } else {
                    true
                }
            }
            None => true,
        }
    }

    /// 标记运行完成
    pub fn mark_run_complete(&mut self, summary: &str, duration_secs: u64) {
        self.state.last_run_at = Some(chrono::Utc::now().to_rfc3339());
        self.state.last_run_duration_secs = Some(duration_secs);
        self.state.last_run_summary = Some(summary.to_string());
        self.state.run_count += 1;
    }

    /// 获取技能使用数据
    pub fn get_usage(&self, skill_name: &str) -> Option<&SkillUsage> {
        self.usage.get(skill_name)
    }

    /// 获取所有技能使用数据
    pub fn all_usage(&self) -> &HashMap<String, SkillUsage> {
        &self.usage
    }

    /// Import constraint lessons from LessonTracker as skill updates
    pub fn import_lessons(&mut self, lessons: Vec<crate::domain::agents::lesson_tracker::ConstraintLesson>) {
        for lesson in lessons {
            let skill_name = format!("lesson_{}", lesson.agent_role);
            let existing = self.usage.entry(skill_name.clone())
                .or_insert_with(|| SkillUsage::new(&skill_name, "system"));

            existing.use_count += 1;
            existing.last_activity_at = Some(chrono::Utc::now().to_rfc3339());
            existing.state = SkillState::Active;

            tracing::info!(
                skill = %skill_name,
                lesson = %lesson.lesson,
                "Imported constraint lesson as skill update"
            );
        }
    }

    /// 获取配置
    pub fn config(&self) -> &CuratorConfig {
        &self.config
    }

    /// 获取状态
    pub fn state(&self) -> &CuratorState {
        &self.state
    }
}

// ── 记忆提供者特征 ──────────────────────────────────────────

/// 可插拔记忆提供者抽象基类
///
/// 生命周期（由 MemoryManager 调用）：
/// - `initialize()`: 会话启动时调用一次
/// - `system_prompt_block()`: 系统提示组装时调用
/// - `prefetch()`: 每轮 API 调用前召回相关上下文
/// - `sync_turn()`: 每轮结束后异步写入
/// - `shutdown()`: 干净退出
#[async_trait]
pub trait MemoryProvider: Send + Sync {
    /// 提供者名称
    fn name(&self) -> &str;

    /// 是否可用（已配置、有凭证、已就绪）
    fn is_available(&self) -> bool;

    /// 初始化（每个会话调用一次）
    fn initialize(&mut self, session_id: &str) -> Result<(), AppError>;

    /// 系统提示块 — 静态信息注入系统提示
    fn system_prompt_block(&self) -> String {
        String::new()
    }

    /// 预取 — 每轮调用前召回相关上下文
    fn prefetch(&self, query: &str) -> String {
        let _ = query;
        String::new()
    }

    /// 同步轮次 — 每轮结束后写入
    fn sync_turn(&mut self, _messages: &[Message]) -> Result<(), AppError> {
        Ok(())
    }

    /// 关闭
    fn shutdown(&mut self) {}
}

// ── 内置记忆条目 ──────────────────────────────────────────────

/// 内置记忆条目（始终可用，不依赖外部提供者）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinMemoryEntry {
    /// 唯一 ID
    pub id: String,
    /// 内容
    pub content: String,
    /// 分类: "character" | "plot" | "setting" | "style" | "fact"
    pub category: String,
    /// 章节号
    pub chapter: Option<u32>,
    /// 时间戳
    pub timestamp: String,
    /// 标签
    pub tags: Vec<String>,
}

// ── 记忆管理器 ──────────────────────────────────────────────────

/// 记忆管理器 — 编排记忆提供者，管理跨会话记忆
pub struct MemoryManager {
    /// 当前活跃提供者（只有一个）
    active_provider: Option<Box<dyn MemoryProvider>>,
    /// 内置记忆条目
    builtin_entries: Vec<BuiltinMemoryEntry>,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            active_provider: None,
            builtin_entries: Vec::new(),
        }
    }

    /// 设置活跃提供者
    pub fn set_provider(&mut self, provider: Box<dyn MemoryProvider>) {
        self.active_provider = Some(provider);
    }

    /// 预取相关上下文
    pub fn prefetch(&self, query: &str) -> String {
        let mut result = String::new();

        // 从内置记忆搜索
        let builtin_results = self.search_builtin(query, 5);
        if !builtin_results.is_empty() {
            result.push_str("## 相关记忆\n\n");
            for entry in builtin_results {
                result.push_str(&format!("- [{}] {}\n", entry.category, entry.content));
            }
            result.push('\n');
        }

        // 从外部提供者预取
        if let Some(ref provider) = self.active_provider {
            let external = provider.prefetch(query);
            if !external.is_empty() {
                result.push_str(&external);
            }
        }

        result
    }

    /// 同步轮次到外部提供者
    pub fn sync_turn(&mut self, messages: &[Message]) -> Result<(), AppError> {
        if let Some(ref mut provider) = self.active_provider {
            provider.sync_turn(messages)?;
        }
        Ok(())
    }

    /// 添加内置记忆条目
    pub fn add_builtin_entry(&mut self, entry: BuiltinMemoryEntry) {
        self.builtin_entries.push(entry);
    }

    /// 搜索内置记忆
    pub fn search_builtin(&self, query: &str, top_k: usize) -> Vec<&BuiltinMemoryEntry> {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored: Vec<(&BuiltinMemoryEntry, f64)> = self.builtin_entries
            .iter()
            .map(|entry| {
                let content_lower = entry.content.to_lowercase();
                let score: f64 = query_terms
                    .iter()
                    .map(|term| {
                        content_lower.matches(term).count() as f64
                            / (content_lower.len() as f64 + 1.0)
                    })
                    .sum();
                (entry, score)
            })
            .filter(|(_, score)| *score > 0.0)
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(top_k).map(|(e, _)| e).collect()
    }

    /// 格式化所有记忆为提示注入文本
    pub fn format_context(&self) -> String {
        let mut result = String::new();

        if !self.builtin_entries.is_empty() {
            result.push_str("## 持久记忆\n\n");
            for entry in &self.builtin_entries {
                result.push_str(&format!(
                    "- [{}] {}{}\n",
                    entry.category,
                    entry.content,
                    entry.chapter.map(|c| format!(" (第{}章)", c)).unwrap_or_default()
                ));
            }
            result.push('\n');
        }

        if let Some(ref provider) = self.active_provider {
            let block = provider.system_prompt_block();
            if !block.is_empty() {
                result.push_str(&block);
            }
        }

        result
    }

    /// 关闭所有提供者
    pub fn shutdown(&mut self) {
        if let Some(ref mut provider) = self.active_provider {
            provider.shutdown();
        }
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_curator_record_use() {
        let config = CuratorConfig::default();
        let mut curator = SkillCurator::new(config);
        curator.record_use("test-skill");
        let usage = curator.get_usage("test-skill").unwrap();
        assert_eq!(usage.use_count, 1);
        assert_eq!(usage.state, SkillState::Active);
    }

    #[test]
    fn test_skill_curator_pin_unpin() {
        let config = CuratorConfig::default();
        let mut curator = SkillCurator::new(config);
        curator.record_use("test-skill");
        curator.pin("test-skill").unwrap();
        assert!(curator.get_usage("test-skill").unwrap().pinned);
        curator.unpin("test-skill").unwrap();
        assert!(!curator.get_usage("test-skill").unwrap().pinned);
    }

    #[test]
    fn test_skill_curator_should_run() {
        let config = CuratorConfig { enabled: false, ..Default::default() };
        let curator = SkillCurator::new(config);
        assert!(!curator.should_run());

        let config = CuratorConfig::default();
        let curator = SkillCurator::new(config);
        assert!(curator.should_run()); // never run before
    }

    #[test]
    fn test_memory_manager_builtin_search() {
        let mut manager = MemoryManager::new();
        manager.add_builtin_entry(BuiltinMemoryEntry {
            id: "1".into(),
            content: "主角李明是一个程序员".into(),
            category: "character".into(),
            chapter: Some(1),
            timestamp: "2026-01-01T00:00:00Z".into(),
            tags: vec!["主角".into()],
        });
        let results = manager.search_builtin("李明", 5);
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("李明"));
    }

    #[test]
    fn test_memory_manager_format_context() {
        let mut manager = MemoryManager::new();
        manager.add_builtin_entry(BuiltinMemoryEntry {
            id: "1".into(),
            content: "世界观：修仙世界".into(),
            category: "setting".into(),
            chapter: None,
            timestamp: "2026-01-01T00:00:00Z".into(),
            tags: vec![],
        });
        let ctx = manager.format_context();
        assert!(ctx.contains("持久记忆"));
        assert!(ctx.contains("修仙世界"));
    }
}
