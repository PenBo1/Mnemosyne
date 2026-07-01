use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterIntent {
    pub chapter: u32,
    pub goal: String,
    pub outline_node: Option<String>,
    pub arc_context: Option<String>,
    pub must_keep: Vec<String>,
    pub must_avoid: Vec<String>,
    pub style_emphasis: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterMemo {
    pub chapter: u32,
    pub goal: String,
    pub is_golden_opening: bool,
    pub body: String,
    pub thread_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSource {
    pub source: String,
    pub reason: String,
    pub excerpt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackage {
    pub chapter: u32,
    pub selected_context: Vec<ContextSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleLayer {
    pub id: String,
    pub name: String,
    pub precedence: u32,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverrideEdge {
    pub from: String,
    pub to: String,
    pub allowed: bool,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveOverride {
    pub from: String,
    pub to: String,
    pub target: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuleStackSections {
    pub hard: Vec<String>,
    pub soft: Vec<String>,
    pub diagnostic: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleStack {
    pub layers: Vec<RuleLayer>,
    pub sections: RuleStackSections,
    pub override_edges: Vec<OverrideEdge>,
    pub active_overrides: Vec<ActiveOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenBudget {
    pub protected_tokens: u32,
    pub compressible_tokens: u32,
    pub total_selected_tokens: u32,
}

/// S5.4: 章节上下文预算。
///
/// 移植自 inkos `ContextBudget`。当总 token 超过 `context_window_tokens - reserved_output_tokens`
/// 时，触发 compressible 段 LLM 编译压缩。protected 段永不压缩。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ContextBudget {
    /// LLM 上下文窗口大小（tokens）
    pub context_window_tokens: u32,
    /// 为输出预留的 tokens（max_tokens）
    pub reserved_output_tokens: u32,
}

impl ContextBudget {
    /// 可用于输入的 token 数 = 上下文窗口 - 输出预留
    pub fn available_input_tokens(&self) -> u32 {
        self.context_window_tokens.saturating_sub(self.reserved_output_tokens)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextTiers {
    pub protected_sources: Vec<String>,
    pub compressible_sources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChapterTrace {
    pub chapter: u32,
    pub planner_inputs: Vec<String>,
    pub composer_inputs: Vec<String>,
    pub selected_sources: Vec<String>,
    pub context_tiers: ContextTiers,
    pub token_budget: TokenBudget,
    pub notes: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chapter_intent_creation() {
        let intent = ChapterIntent {
            chapter: 1,
            goal: "推进主线".to_string(),
            outline_node: Some("第一章节点".to_string()),
            arc_context: None,
            must_keep: vec!["保持人设".to_string()],
            must_avoid: vec!["不要水字数".to_string()],
            style_emphasis: vec![],
        };
        assert_eq!(intent.chapter, 1);
        assert_eq!(intent.goal, "推进主线");
        assert_eq!(intent.must_keep.len(), 1);
        assert_eq!(intent.must_avoid.len(), 1);
    }

    #[test]
    fn test_chapter_memo_creation() {
        let memo = ChapterMemo {
            chapter: 5,
            goal: "揭露秘密".to_string(),
            is_golden_opening: false,
            body: "## 当前任务\n揭露反派秘密".to_string(),
            thread_refs: vec!["H001".to_string(), "H003".to_string()],
        };
        assert_eq!(memo.chapter, 5);
        assert!(!memo.is_golden_opening);
        assert_eq!(memo.thread_refs.len(), 2);
    }

    #[test]
    fn test_context_package() {
        let pkg = ContextPackage {
            chapter: 3,
            selected_context: vec![
                ContextSource { source: "outline/story_frame.md".into(), reason: "World foundation".into(), excerpt: None },
                ContextSource { source: "story/current_state.md".into(), reason: "Current state".into(), excerpt: Some("test".into()) },
            ],
        };
        assert_eq!(pkg.selected_context.len(), 2);
        assert!(pkg.selected_context[0].excerpt.is_none());
        assert!(pkg.selected_context[1].excerpt.is_some());
    }

    #[test]
    fn test_rule_stack_creation() {
        let stack = RuleStack {
            layers: vec![RuleLayer { id: "L1".into(), name: "hard_facts".into(), precedence: 100, scope: "global".into() }],
            sections: RuleStackSections { hard: vec!["test".into()], soft: vec![], diagnostic: vec![] },
            override_edges: vec![],
            active_overrides: vec![],
        };
        assert_eq!(stack.layers.len(), 1);
        assert_eq!(stack.sections.hard.len(), 1);
    }

    #[test]
    fn test_chapter_trace_default() {
        let trace = ChapterTrace::default();
        assert_eq!(trace.chapter, 0);
        assert!(trace.notes.is_empty());
    }

    #[test]
    fn test_json_serialization_roundtrip() {
        let intent = ChapterIntent {
            chapter: 1, goal: "test".into(), outline_node: None, arc_context: None,
            must_keep: vec![], must_avoid: vec![], style_emphasis: vec![],
        };
        let json = serde_json::to_string(&intent).unwrap();
        let parsed: ChapterIntent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.chapter, intent.chapter);
        assert_eq!(parsed.goal, intent.goal);
    }

    #[test]
    fn test_context_budget_available_input_tokens() {
        let budget = ContextBudget {
            context_window_tokens: 128_000,
            reserved_output_tokens: 8_000,
        };
        assert_eq!(budget.available_input_tokens(), 120_000);
    }

    #[test]
    fn test_context_budget_saturating_sub() {
        // reserved > window 时不应下溢
        let budget = ContextBudget {
            context_window_tokens: 1_000,
            reserved_output_tokens: 2_000,
        };
        assert_eq!(budget.available_input_tokens(), 0);
    }

    #[test]
    fn test_context_budget_zero_window() {
        let budget = ContextBudget {
            context_window_tokens: 0,
            reserved_output_tokens: 0,
        };
        assert_eq!(budget.available_input_tokens(), 0);
    }
}
