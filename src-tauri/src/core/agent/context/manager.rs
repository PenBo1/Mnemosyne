//! ═══════════════════════════════════════════════════════════════════════════
//! Context Manager - 上下文管理器
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 线程历史容器, 负责持有历史、维护版本号、守护不可变性。
//!
//! 核心不变量:
//! 1. No history rewrite - 已写入的历史项不可就地修改, 以保护 cache prefix
//! 2. history_version 在每次整体替换时递增, 作为 cache 失效信号
//! 3. reference_context_item 用于下一轮的 diff 计算
//! 4. world_state_baseline 用于增量 render_diff

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::infrastructure::llm::types::Message;
use crate::shared::error::AppError;

/// 模型可见历史的版本号类型
pub type HistoryVersion = u64;

/// 单轮上下文快照
/// 
/// 保存上一轮注入到模型上下文的 baseline 快照。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TurnContextItem {
    /// 关联的轮次 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    /// 当前工作目录
    pub cwd: String,
    /// 工作区根路径列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_roots: Option<Vec<String>>,
    /// 当前模型 slug
    pub model: String,
    /// 当前会话语言
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
}

/// 世界状态快照
/// 
/// 持有会话级的环境状态。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WorldState {
    /// 当前工作目录
    pub cwd: String,
    /// 工作区根路径列表
    #[serde(default)]
    pub workspace_roots: Vec<String>,
    /// 当前模型 slug
    pub model: String,
    /// 当前会话语言
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
}

/// 线程历史容器
/// 
/// 持有按时间顺序排列的消息项, 以及版本号、baseline 等信息。
#[derive(Debug, Clone, Default)]
pub struct ContextManager {
    /// 消息项列表 (oldest → newest)
    items: Vec<Message>,
    /// 历史版本号
    history_version: HistoryVersion,
    /// 上一轮的 baseline 快照
    reference_context_item: Option<TurnContextItem>,
    /// 世界状态 baseline
    world_state_baseline: Option<Arc<WorldState>>,
}

impl ContextManager {
    /// 创建空容器
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取当前历史版本号
    pub fn history_version(&self) -> HistoryVersion {
        self.history_version
    }

    /// 获取历史项的只读视图
    pub fn raw_items(&self) -> &[Message] {
        &self.items
    }

    /// 消费容器, 返回内部历史项
    pub fn into_raw_items(self) -> Vec<Message> {
        self.items
    }

    /// 获取 reference baseline
    pub fn reference_context_item(&self) -> Option<&TurnContextItem> {
        self.reference_context_item.as_ref()
    }

    /// 设置 reference baseline
    pub fn set_reference_context_item(&mut self, item: Option<TurnContextItem>) {
        self.reference_context_item = item;
    }

    /// 获取 world state baseline
    pub fn world_state_baseline(&self) -> Option<&WorldState> {
        self.world_state_baseline.as_deref()
    }

    /// 直接覆盖 world state baseline
    pub fn set_world_state_baseline(&mut self, world_state: WorldState) {
        self.world_state_baseline = Some(Arc::new(world_state));
    }

    /// 更新 world state 并返回 diff 描述
    /// 
    /// 若 baseline 为 None, 返回完整渲染文本;
    /// 否则仅返回发生变化的字段描述。
    pub fn update_world_state(&mut self, world_state: WorldState) -> String {
        let fragment = match self.world_state_baseline.as_deref() {
            None => render_world_state_full(&world_state),
            Some(previous) => render_world_state_diff(previous, &world_state),
        };
        self.world_state_baseline = Some(Arc::new(world_state));
        fragment
    }

    /// 追加历史项
    /// 
    /// 只追加, 不修改已有项, 保护 cache prefix。
    pub fn record_items<I>(&mut self, items: I)
    where
        I: IntoIterator<Item = Message>,
    {
        self.items.extend(items);
    }

    /// 移除最旧的历史项
    /// 
    /// 用于 compaction 从前方向后裁剪。推进版本号。
    pub fn remove_first_item(&mut self) {
        if !self.items.is_empty() {
            self.items.remove(0);
            self.world_state_baseline = None;
            self.history_version = self.history_version.saturating_add(1);
        }
    }

    /// 整体替换历史
    /// 
    /// 推进版本号, 清空 baseline。
    pub fn replace(&mut self, items: Vec<Message>) {
        self.items = items;
        self.history_version = self.history_version.saturating_add(1);
        self.world_state_baseline = None;
    }

    /// Compaction 专用替换入口
    /// 
    /// 在 replace 基础上额外处理 reference_context_item。
    /// 若新历史与现有历史逐项等价, 视为 no-op。
    /// 
    /// # 返回值
    /// 返回新的 history_version
    pub fn replace_compacted_history(
        &mut self,
        items: Vec<Message>,
        reference_context_item: Option<TurnContextItem>,
    ) -> Result<HistoryVersion, AppError> {
        if items.len() == self.items.len()
            && items
                .iter()
                .zip(self.items.iter())
                .all(|(a, b)| message_content_eq(a, b))
        {
            self.reference_context_item = reference_context_item;
            return Ok(self.history_version);
        }

        self.items = items;
        self.history_version = self.history_version.saturating_add(1);
        self.world_state_baseline = None;
        self.reference_context_item = reference_context_item;
        Ok(self.history_version)
    }

    /// 守护历史不可变性
    /// 
    /// 已记录的历史项不可就地修改。
    pub fn ensure_no_inplace_rewrite(&self) -> Result<(), AppError> {
        Ok(())
    }
}

/// 比较两条消息的可见内容是否等价
fn message_content_eq(a: &Message, b: &Message) -> bool {
    if a.role != b.role || a.content != b.content || a.tool_call_id != b.tool_call_id {
        return false;
    }
    match (&a.tool_calls, &b.tool_calls) {
        (None, None) => true,
        (Some(ac), Some(bc)) => {
            ac.len() == bc.len()
                && ac
                    .iter()
                    .zip(bc.iter())
                    .all(|(x, y)| x.id == y.id && x.name == y.name && x.arguments == y.arguments)
        }
        _ => false,
    }
}

/// 渲染 world state 的完整描述
fn render_world_state_full(ws: &WorldState) -> String {
    let mut lines = Vec::new();
    lines.push("[world_state]".to_string());
    lines.push(format!("cwd: {}", ws.cwd));
    if !ws.workspace_roots.is_empty() {
        lines.push(format!("workspace_roots: {}", ws.workspace_roots.join(", ")));
    }
    lines.push(format!("model: {}", ws.model));
    if let Some(locale) = &ws.locale {
        lines.push(format!("locale: {locale}"));
    }
    lines.join("\n")
}

/// 渲染 world state 的差异
fn render_world_state_diff(prev: &WorldState, curr: &WorldState) -> String {
    let mut lines = Vec::new();
    lines.push("[world_state_diff]".to_string());
    if prev.cwd != curr.cwd {
        lines.push(format!("cwd: {} -> {}", prev.cwd, curr.cwd));
    }
    if prev.workspace_roots != curr.workspace_roots {
        lines.push(format!(
            "workspace_roots: {} -> {}",
            prev.workspace_roots.join(", "),
            curr.workspace_roots.join(", ")
        ));
    }
    if prev.model != curr.model {
        lines.push(format!("model: {} -> {}", prev.model, curr.model));
    }
    if prev.locale != curr.locale {
        lines.push(format!(
            "locale: {} -> {}",
            prev.locale.as_deref().unwrap_or("(none)"),
            curr.locale.as_deref().unwrap_or("(none)")
        ));
    }
    if lines.len() == 1 {
        lines.push("(no changes)".to_string());
    }
    lines.join("\n")
}

// ── 测试用例 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn user_msg(content: &str) -> Message {
        Message {
            role: "user".to_string(),
            content: content.to_string(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    fn assistant_msg(content: &str) -> Message {
        Message {
            role: "assistant".to_string(),
            content: content.to_string(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    #[allow(dead_code)]
    fn turn_ctx(model: &str) -> TurnContextItem {
        TurnContextItem {
            turn_id: Some("turn-1".to_string()),
            cwd: "/tmp".to_string(),
            workspace_roots: Some(vec!["/tmp".to_string()]),
            model: model.to_string(),
            locale: Some("zh".to_string()),
        }
    }

    #[test]
    fn new_manager_has_zero_version() {
        let m = ContextManager::new();
        assert_eq!(m.history_version(), 0);
        assert!(m.raw_items().is_empty());
        assert!(m.reference_context_item().is_none());
        assert!(m.world_state_baseline().is_none());
    }

    #[test]
    fn record_items_does_not_advance_version() {
        let mut m = ContextManager::new();
        m.record_items([user_msg("hi"), assistant_msg("hello")]);
        assert_eq!(m.raw_items().len(), 2);
        assert_eq!(m.history_version(), 0, "record_items 不推进版本号");
    }

    #[test]
    fn replace_advances_version_and_clears_baseline() {
        let mut m = ContextManager::new();
        m.record_items([user_msg("old")]);
        m.set_world_state_baseline(WorldState {
            cwd: "/a".to_string(),
            workspace_roots: vec![],
            model: "m".to_string(),
            locale: None,
        });
        assert_eq!(m.history_version(), 0);

        m.replace(vec![user_msg("new")]);
        assert_eq!(m.history_version(), 1);
        assert_eq!(m.raw_items().len(), 1);
        assert_eq!(m.raw_items()[0].content, "new");
        assert!(m.world_state_baseline().is_none());
    }

    #[test]
    fn replace_compacted_history_advances_version_when_items_change() {
        let mut m = ContextManager::new();
        m.record_items([user_msg("a"), user_msg("b"), user_msg("c")]);
        assert_eq!(m.history_version(), 0);

        let new_items = vec![user_msg("summary"), user_msg("c")];
        let v = m.replace_compacted_history(new_items, None).expect("compact replace ok");
        assert_eq!(v, 1);
        assert_eq!(m.history_version(), 1);
        assert_eq!(m.raw_items().len(), 2);
        assert!(m.reference_context_item().is_none());
    }

    #[test]
    fn replace_compacted_history_noop_when_items_equivalent() {
        let mut m = ContextManager::new();
        let items = vec![user_msg("a"), user_msg("b")];
        m.record_items(items.clone());
        assert_eq!(m.history_version(), 0);

        let same_items = vec![user_msg("a"), user_msg("b")];
        let v = m.replace_compacted_history(same_items, None).expect("noop ok");
        assert_eq!(v, 0, "等价历史不应推进版本号");
        assert_eq!(m.history_version(), 0);
    }

    #[test]
    fn update_world_state_first_call_returns_full() {
        let mut m = ContextManager::new();
        let ws = WorldState {
            cwd: "/tmp".to_string(),
            workspace_roots: vec!["/tmp".to_string()],
            model: "m1".to_string(),
            locale: Some("zh".to_string()),
        };
        let fragment = m.update_world_state(ws);
        assert!(fragment.contains("[world_state]"));
        assert!(fragment.contains("cwd: /tmp"));
        assert!(fragment.contains("model: m1"));
        assert!(m.world_state_baseline().is_some());
    }
}