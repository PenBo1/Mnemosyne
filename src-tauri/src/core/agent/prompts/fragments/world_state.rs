//! ═══════════════════════════════════════════════════════════════════════════
//! WorldStateFragment - 世界状态 baseline + render_diff
//! ═══════════════════════════════════════════════════════════════════════════

use std::any::Any;

use crate::shared::error::AppError;

use super::super::fragment::{ContextualUserFragment, FragmentRole};

/// 最小 WorldState 结构（事实列表）。
///
/// Stage E 会扩展为完整类型（含角色状态、伏笔账本等），届时可替换。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldState {
    /// 已建立的事实（按时间顺序追加）
    pub facts: Vec<String>,
}

impl WorldState {
    pub fn new() -> Self {
        Self { facts: Vec::new() }
    }

    pub fn from_facts(facts: Vec<String>) -> Self {
        Self { facts }
    }

    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }
}

impl Default for WorldState {
    fn default() -> Self {
        Self::new()
    }
}

/// WorldState fragment。
///
/// 持有 current state + 可选 baseline（上一轮的 state）。
/// - render_full：输出 current 的完整快照
/// - render_diff：若 baseline 存在且与 current 不同，输出变化项；否则返回 None
pub struct WorldStateFragment {
    current: WorldState,
    baseline: Option<WorldState>,
}

impl WorldStateFragment {
    /// 用 current state 构造（无 baseline，首次注入用）。
    pub fn new(current: WorldState) -> Self {
        Self {
            current,
            baseline: None,
        }
    }

    /// 用 current + baseline 构造（后续轮次，支持 diff）。
    pub fn with_baseline(current: WorldState, baseline: WorldState) -> Self {
        Self {
            current,
            baseline: Some(baseline),
        }
    }

    /// 当前 state 引用（用于调试）。
    pub fn current(&self) -> &WorldState {
        &self.current
    }

    /// baseline 引用（用于调试）。
    pub fn baseline(&self) -> Option<&WorldState> {
        self.baseline.as_ref()
    }

    /// 渲染完整 WorldState 快照。
    fn render_state(state: &WorldState) -> String {
        if state.is_empty() {
            return String::new();
        }
        let mut out = String::from("<world-state>\n");
        for fact in &state.facts {
            out.push_str("- ");
            out.push_str(fact);
            out.push('\n');
        }
        out.push_str("</world-state>");
        out
    }

    /// 计算与 baseline 的差异（新增事实）。
    ///
    /// 简化实现：仅检测 baseline 之后新增的事实（顺序敏感）。
    /// 复杂的 diff 算法（LCS / Myers）由 Stage E 引入完整 WorldState 时补充。
    fn diff_against(&self, baseline: &WorldState) -> Vec<String> {
        // 找到 baseline.facts 在 current.facts 中的最长公共前缀
        let common_prefix_len = baseline
            .facts
            .iter()
            .zip(self.current.facts.iter())
            .take_while(|(a, b)| a == b)
            .count();
        // 公共前缀之后的事实为新增
        self.current.facts[common_prefix_len..].to_vec()
    }
}

impl ContextualUserFragment for WorldStateFragment {
    fn name(&self) -> &str {
        "world_state"
    }

    fn render_full(&self) -> Result<String, AppError> {
        Ok(Self::render_state(&self.current))
    }

    fn render_diff(&self, other: &dyn ContextualUserFragment) -> Option<String> {
        // 仅当 baseline 是同类型 fragment 时才尝试 diff
        let other_any = other.as_any();
        let other_state = other_any.downcast_ref::<WorldStateFragment>()?;

        // 用 other 的 current 作为 baseline
        let baseline = &other_state.current;

        if baseline == &self.current {
            // 状态未变：不注入（返回 None 让 registry 回退到空）
            return None;
        }

        let added = self.diff_against(baseline);
        if added.is_empty() {
            return None;
        }

        let mut out = String::from("<world-state-diff>\n");
        for fact in &added {
            out.push_str("+ ");
            out.push_str(fact);
            out.push('\n');
        }
        out.push_str("</world-state-diff>");
        Some(out)
    }

    fn bounded_size(&self) -> Option<usize> {
        Some(2048)
    }

    fn role(&self) -> FragmentRole {
        FragmentRole::System
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_full_empty_state_returns_empty() {
        let frag = WorldStateFragment::new(WorldState::new());
        assert_eq!(frag.render_full().unwrap(), "");
    }

    #[test]
    fn render_full_lists_all_facts() {
        let state = WorldState::from_facts(vec![
            "主角在森林中".to_string(),
            "持有长剑".to_string(),
        ]);
        let frag = WorldStateFragment::new(state);

        let rendered = frag.render_full().unwrap();
        assert!(rendered.contains("<world-state>"));
        assert!(rendered.contains("- 主角在森林中"));
        assert!(rendered.contains("- 持有长剑"));
        assert!(rendered.contains("</world-state>"));
    }

    #[test]
    fn render_diff_returns_none_when_no_baseline_match() {
        // baseline 不是 WorldStateFragment → render_diff 返回 None
        let state = WorldState::from_facts(vec!["x".to_string()]);
        let frag = WorldStateFragment::new(state);

        // 用一个不同类型的 fragment 作为 baseline（这里用 stub）
        struct StubFragment;
        impl ContextualUserFragment for StubFragment {
            fn name(&self) -> &str {
                "stub"
            }
            fn render_full(&self) -> Result<String, AppError> {
                Ok(String::new())
            }
            fn bounded_size(&self) -> Option<usize> {
                None
            }
            fn role(&self) -> FragmentRole {
                FragmentRole::System
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
        }

        assert!(frag.render_diff(&StubFragment).is_none());
    }

    #[test]
    fn render_diff_returns_none_when_state_unchanged() {
        let state = WorldState::from_facts(vec!["x".to_string(), "y".to_string()]);
        let baseline_frag = WorldStateFragment::new(state.clone());
        let current_frag = WorldStateFragment::new(state);

        // current 与 baseline 相同 → diff 返回 None
        assert!(current_frag.render_diff(&baseline_frag).is_none());
    }

    #[test]
    fn render_diff_outputs_added_facts() {
        let baseline = WorldState::from_facts(vec![
            "主角在森林".to_string(),
            "持有长剑".to_string(),
        ]);
        let current = WorldState::from_facts(vec![
            "主角在森林".to_string(),
            "持有长剑".to_string(),
            "遇到导师".to_string(), // 新增
        ]);

        let baseline_frag = WorldStateFragment::new(baseline);
        let current_frag = WorldStateFragment::new(current);

        let diff = current_frag.render_diff(&baseline_frag).expect("应有 diff");

        assert!(diff.contains("<world-state-diff>"));
        assert!(diff.contains("+ 遇到导师"));
        assert!(!diff.contains("主角在森林"), "diff 不应含未变化项");
    }

    #[test]
    fn render_diff_returns_none_when_nothing_added() {
        let baseline = WorldState::from_facts(vec!["x".to_string()]);
        // current 与 baseline 相同前缀，无新增
        let current = WorldState::from_facts(vec!["x".to_string()]);

        let baseline_frag = WorldStateFragment::new(baseline);
        let current_frag = WorldStateFragment::new(current);

        assert!(current_frag.render_diff(&baseline_frag).is_none());
    }

    #[test]
    fn fragment_metadata() {
        let frag = WorldStateFragment::new(WorldState::new());
        assert_eq!(frag.name(), "world_state");
        assert_eq!(frag.bounded_size(), Some(2048));
        assert_eq!(frag.role(), FragmentRole::System);
    }

    #[test]
    fn with_baseline_stores_baseline() {
        let baseline = WorldState::from_facts(vec!["old".to_string()]);
        let current = WorldState::from_facts(vec!["new".to_string()]);

        let frag = WorldStateFragment::with_baseline(current.clone(), baseline.clone());

        assert_eq!(frag.current(), &current);
        assert_eq!(frag.baseline(), Some(&baseline));
    }
}
