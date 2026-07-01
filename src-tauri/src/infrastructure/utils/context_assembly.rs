use crate::core::agent::governance::*;
use crate::core::agent::planner::PlanOutput;

/// S5.4: 估算一组 ContextSource 的总 token 数。
///
/// 粗略估算：1 token ≈ 3 字节（中文偏保守，英文偏宽松）。
/// 与 inkos `estimateTextTokens` 行为对齐。
pub fn estimate_selected_context_tokens(entries: &[ContextSource]) -> u32 {
    entries.iter().map(|e| estimate_tokens(e)).sum()
}

/// S5.4: 把 ContextSource 列表渲染为 Markdown，供 LLM 编译时查看。
///
/// 与 inkos `renderContextEntries` 对齐。
pub fn render_context_entries(entries: &[ContextSource]) -> String {
    entries
        .iter()
        .map(|e| {
            let excerpt = e.excerpt.as_deref().unwrap_or("(no excerpt)");
            format!("### {}\nReason: {}\n{}", e.source, e.reason, excerpt)
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// S5.4: 上下文预算处理结果。
///
/// - `WithinBudget`：总 token 未超预算，原样返回
/// - `Compiled`：超预算且编译成功，返回新的 context_package（protected + compiled 压缩段）
/// - `OverBudgetNoCompressible`：超预算但无可压缩段（应记录警告）
/// - `ProtectedExceedsBudget`：protected 段已超预算（错误，无法挽救）
#[derive(Debug)]
pub enum BudgetApplication {
    /// 总 token 未超预算
    WithinBudget,
    /// 超预算且编译成功
    Compiled {
        compiled_excerpt: String,
        protected_tokens: u32,
        compressible_tokens: u32,
        compile_budget: u32,
    },
    /// 超预算但无可压缩段
    OverBudgetNoCompressible,
    /// protected 段已超预算
    ProtectedExceedsBudget {
        protected_tokens: u32,
        budget_tokens: u32,
    },
}

/// S5.4: 判断是否需要应用上下文预算压缩。
///
/// 纯函数，不做副作用。返回 `BudgetApplication` 让调用方决定如何处理。
/// 与 inkos `applyContextBudgetIfNeeded` 的核心逻辑对齐，但把 LLM 调用
/// 留给调用方（保持本模块无 LLM 依赖）。
pub fn apply_context_budget_if_needed(
    context_package: &ContextPackage,
    budget: ContextBudget,
) -> BudgetApplication {
    let available_input = budget.available_input_tokens();
    if available_input == 0 {
        return BudgetApplication::WithinBudget;
    }

    let selected = &context_package.selected_context;
    let total_tokens = estimate_selected_context_tokens(selected);
    if total_tokens <= available_input {
        return BudgetApplication::WithinBudget;
    }

    let protected_entries: Vec<&ContextSource> = selected
        .iter()
        .filter(|e| is_protected_context_source(&e.source))
        .collect();
    let compressible_entries: Vec<&ContextSource> = selected
        .iter()
        .filter(|e| !is_protected_context_source(&e.source))
        .collect();

    let protected_tokens = estimate_selected_context_tokens(
        &protected_entries.into_iter().cloned().collect::<Vec<_>>(),
    );
    if protected_tokens > available_input {
        return BudgetApplication::ProtectedExceedsBudget {
            protected_tokens,
            budget_tokens: available_input,
        };
    }
    if compressible_entries.is_empty() {
        return BudgetApplication::OverBudgetNoCompressible;
    }

    let compile_budget = available_input.saturating_sub(protected_tokens).max(1);
    let compressible_tokens = estimate_selected_context_tokens(
        &compressible_entries.into_iter().cloned().collect::<Vec<_>>(),
    );

    // 调用方需要基于 BudgetApplication::Compiled 自行调用 LLM 编译，
    // 然后用编译结果替换 compressible 段。
    // 这里只返回决策信息，不直接执行编译（保持纯函数）。
    BudgetApplication::Compiled {
        // compiled_excerpt 由调用方填充（这里是占位空串）
        compiled_excerpt: String::new(),
        protected_tokens,
        compressible_tokens,
        compile_budget,
    }
}

/// S5.4: 构造压缩后的 ContextPackage。
///
/// 用 protected 段 + 一条 `runtime/compiled-compressible-context` 条目
/// 替换原始 context_package。
pub fn build_compiled_context_package(
    original: &ContextPackage,
    compiled_excerpt: String,
) -> ContextPackage {
    let protected: Vec<ContextSource> = original
        .selected_context
        .iter()
        .filter(|e| is_protected_context_source(&e.source))
        .cloned()
        .collect();

    let mut new_context = protected;
    new_context.push(ContextSource {
        source: "runtime/compiled-compressible-context".to_string(),
        reason: "Semantic compilation of lower-priority context after protected context exceeded the input budget.".to_string(),
        excerpt: Some(compiled_excerpt),
    });

    ContextPackage {
        chapter: original.chapter,
        selected_context: new_context,
    }
}

/// Build the per-chapter rule stack from planner output.
pub fn build_governed_rule_stack(plan: &PlanOutput, chapter_number: u32) -> RuleStack {
    let mut active_overrides = Vec::new();

    // Per-chapter prohibitions narrow the planning layer
    for item in &plan.intent.must_avoid {
        active_overrides.push(ActiveOverride {
            from: "L4".to_string(),
            to: "L3".to_string(),
            target: format!("chapter:{}/mustAvoid", chapter_number),
            reason: truncate_for_override_reason(item),
        });
    }

    // Per-chapter style emphasis
    for item in &plan.intent.style_emphasis {
        active_overrides.push(ActiveOverride {
            from: "L4".to_string(),
            to: "L3".to_string(),
            target: format!("chapter:{}/styleEmphasis", chapter_number),
            reason: truncate_for_override_reason(item),
        });
    }

    RuleStack {
        layers: vec![
            RuleLayer { id: "L1".into(), name: "hard_facts".into(), precedence: 100, scope: "global".into() },
            RuleLayer { id: "L2".into(), name: "author_intent".into(), precedence: 80, scope: "book".into() },
            RuleLayer { id: "L3".into(), name: "planning".into(), precedence: 60, scope: "arc".into() },
            RuleLayer { id: "L4".into(), name: "current_task".into(), precedence: 70, scope: "local".into() },
        ],
        sections: RuleStackSections {
            hard: vec!["story_frame".into(), "current_state".into(), "book_rules".into(), "roles".into()],
            soft: vec!["author_intent".into(), "current_focus".into(), "volume_map".into()],
            diagnostic: vec!["anti_ai_checks".into(), "continuity_audit".into(), "style_regression_checks".into()],
        },
        override_edges: vec![
            OverrideEdge { from: "L4".into(), to: "L3".into(), allowed: true, scope: "current_chapter".into() },
            OverrideEdge { from: "L4".into(), to: "L2".into(), allowed: false, scope: "current_chapter".into() },
            OverrideEdge { from: "L4".into(), to: "L1".into(), allowed: false, scope: "current_chapter".into() },
        ],
        active_overrides,
    }
}

/// Build a chapter trace for debugging/audit.
pub fn build_governed_trace(
    chapter_number: u32,
    plan: &PlanOutput,
    context_package: &ContextPackage,
    composer_inputs: Vec<String>,
    notes: Vec<String>,
) -> ChapterTrace {
    let protected_entries: Vec<&ContextSource> = context_package.selected_context.iter()
        .filter(|e| is_protected_context_source(&e.source))
        .collect();
    let compressible_entries: Vec<&ContextSource> = context_package.selected_context.iter()
        .filter(|e| !is_protected_context_source(&e.source))
        .collect();

    let protected_tokens: u32 = protected_entries.iter().map(|e| estimate_tokens(e)).sum();
    let compressible_tokens: u32 = compressible_entries.iter().map(|e| estimate_tokens(e)).sum();

    ChapterTrace {
        chapter: chapter_number,
        planner_inputs: vec![plan.intent.goal.clone()],
        composer_inputs,
        selected_sources: context_package.selected_context.iter().map(|e| e.source.clone()).collect(),
        context_tiers: ContextTiers {
            protected_sources: protected_entries.into_iter().map(|e| e.source.clone()).collect(),
            compressible_sources: compressible_entries.into_iter().map(|e| e.source.clone()).collect(),
        },
        token_budget: TokenBudget {
            protected_tokens,
            compressible_tokens,
            total_selected_tokens: protected_tokens + compressible_tokens,
        },
        notes,
    }
}

/// Check if a context source is protected (must always be included).
pub fn is_protected_context_source(source: &str) -> bool {
    source == "runtime/chapter_memo"
        || source == "story/current_focus.md"
        || source == "story/author_intent.md"
        || source == "story/outline/story_frame.md"
        || source.starts_with("story/outline/story_frame.md#")
        || source == "story/story_bible.md"
        || source == "story/outline/volume_map.md"
        || source.starts_with("story/outline/volume_map.md#")
        || source == "story/volume_outline.md"
        || source == "story/parent_canon.md"
        || source == "story/fanfic_canon.md"
        || source.starts_with("story/current_state.md")
        || source.starts_with("story/pending_hooks.md#")
}

fn truncate_for_override_reason(value: &str) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.len() > 80 {
        format!("{}...", &collapsed[..79])
    } else {
        collapsed
    }
}

fn estimate_tokens(entry: &ContextSource) -> u32 {
    let text = match &entry.excerpt {
        Some(excerpt) => format!("{}\n{}\n{}", entry.source, entry.reason, excerpt),
        None => format!("{}\n{}", entry.source, entry.reason),
    };
    // Rough estimate: 1 token ~= 2 bytes for Chinese, 4 chars for English
    (text.len() / 3) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_source(source: &str, excerpt: &str) -> ContextSource {
        ContextSource {
            source: source.to_string(),
            reason: "test".to_string(),
            excerpt: Some(excerpt.to_string()),
        }
    }

    #[test]
    fn test_estimate_tokens_non_empty() {
        let entry = make_source("story/current_focus.md", "some content here");
        let tokens = estimate_tokens(&entry);
        assert!(tokens > 0);
    }

    #[test]
    fn test_estimate_selected_context_tokens_sums() {
        let entries = vec![
            make_source("story/current_focus.md", "abc"),
            make_source("story/author_intent.md", "def"),
        ];
        let total = estimate_selected_context_tokens(&entries);
        assert!(total > 0);
    }

    #[test]
    fn test_render_context_entries_includes_source_and_excerpt() {
        let entries = vec![make_source("story/foo.md", "hello world")];
        let rendered = render_context_entries(&entries);
        assert!(rendered.contains("### story/foo.md"));
        assert!(rendered.contains("hello world"));
        assert!(rendered.contains("Reason: test"));
    }

    #[test]
    fn test_render_context_entries_handles_no_excerpt() {
        let entry = ContextSource {
            source: "story/foo.md".to_string(),
            reason: "r".to_string(),
            excerpt: None,
        };
        let rendered = render_context_entries(&[entry]);
        assert!(rendered.contains("(no excerpt)"));
    }

    #[test]
    fn test_apply_budget_within_budget() {
        let pkg = ContextPackage {
            chapter: 1,
            selected_context: vec![make_source("story/current_focus.md", "short")],
        };
        let budget = ContextBudget {
            context_window_tokens: 1_000_000,
            reserved_output_tokens: 1_000,
        };
        match apply_context_budget_if_needed(&pkg, budget) {
            BudgetApplication::WithinBudget => {}
            other => panic!("expected WithinBudget, got {:?}", other),
        }
    }

    #[test]
    fn test_apply_budget_zero_window_returns_within_budget() {
        // available_input_tokens == 0 时跳过预算检查（与 inkos 行为对齐）
        let pkg = ContextPackage {
            chapter: 1,
            selected_context: vec![make_source("story/current_focus.md", "short")],
        };
        let budget = ContextBudget {
            context_window_tokens: 0,
            reserved_output_tokens: 0,
        };
        assert!(matches!(
            apply_context_budget_if_needed(&pkg, budget),
            BudgetApplication::WithinBudget
        ));
    }

    #[test]
    fn test_apply_budget_protected_exceeds_budget() {
        // protected 段单独超预算 → ProtectedExceedsBudget
        let long_protected = "x".repeat(10_000);
        let pkg = ContextPackage {
            chapter: 1,
            selected_context: vec![make_source("story/outline/story_frame.md", &long_protected)],
        };
        let budget = ContextBudget {
            context_window_tokens: 1_000,
            reserved_output_tokens: 0,
        };
        match apply_context_budget_if_needed(&pkg, budget) {
            BudgetApplication::ProtectedExceedsBudget { protected_tokens, budget_tokens } => {
                assert!(protected_tokens > budget_tokens);
                assert_eq!(budget_tokens, 1_000);
            }
            other => panic!("expected ProtectedExceedsBudget, got {:?}", other),
        }
    }

    #[test]
    fn test_apply_budget_over_budget_no_compressible() {
        // 只有 protected 段且超预算 → ProtectedExceedsBudget（不是 OverBudgetNoCompressible）
        // OverBudgetNoCompressible 要求 protected 未超但 compressible 为空
        // 构造：protected 不超预算，compressible 为空但总 token 超预算 — 这不可能
        // 除非 protected 本身不超但 total > available，且没有 compressible
        // 实际上：若 total > available 且 protected ≤ available 且 compressible 为空
        //        则 total == protected ≤ available，矛盾
        // 所以此分支在纯 protected 场景下不可达
        // 测试改成：有 protected + compressible，但 compressible 为空已经被前面分支处理
        // 这里改为测试 OverBudgetNoCompressible 不可达的边界
        let pkg = ContextPackage {
            chapter: 1,
            selected_context: vec![make_source("story/outline/story_frame.md", "short")],
        };
        let budget = ContextBudget {
            context_window_tokens: 1,
            reserved_output_tokens: 0,
        };
        // protected (short) > 1 → ProtectedExceedsBudget
        assert!(matches!(
            apply_context_budget_if_needed(&pkg, budget),
            BudgetApplication::ProtectedExceedsBudget { .. }
        ));
    }

    #[test]
    fn test_apply_budget_compiled_when_over_budget_with_compressible() {
        // protected 短（不超预算），compressible 长（使总 token 超预算）
        let protected = make_source("story/outline/story_frame.md", "protected short");
        let compressible = make_source("story/book_rules.md", &"y".repeat(5_000));
        let pkg = ContextPackage {
            chapter: 1,
            selected_context: vec![protected, compressible],
        };
        let budget = ContextBudget {
            context_window_tokens: 1_000,
            reserved_output_tokens: 0,
        };
        match apply_context_budget_if_needed(&pkg, budget) {
            BudgetApplication::Compiled {
                protected_tokens,
                compressible_tokens,
                compile_budget,
                ..
            } => {
                assert!(protected_tokens < compressible_tokens);
                assert!(compile_budget > 0);
                assert!(compile_budget < 1_000);
            }
            other => panic!("expected Compiled, got {:?}", other),
        }
    }

    #[test]
    fn test_build_compiled_context_package_replaces_compressible() {
        let protected = make_source("story/outline/story_frame.md", "protected");
        let compressible = make_source("story/book_rules.md", "compressible original");
        let original = ContextPackage {
            chapter: 5,
            selected_context: vec![protected, compressible],
        };
        let compiled = build_compiled_context_package(&original, "compiled excerpt".to_string());
        assert_eq!(compiled.chapter, 5);
        // 应该有 2 条：1 条 protected + 1 条 compiled
        assert_eq!(compiled.selected_context.len(), 2);
        assert_eq!(compiled.selected_context[0].source, "story/outline/story_frame.md");
        assert_eq!(compiled.selected_context[1].source, "runtime/compiled-compressible-context");
        assert_eq!(
            compiled.selected_context[1].excerpt.as_deref(),
            Some("compiled excerpt")
        );
    }

    #[test]
    fn test_build_compiled_context_package_drops_all_compressible() {
        let protected = make_source("story/current_focus.md", "p");
        let c1 = make_source("story/book_rules.md", "c1");
        let c2 = make_source("story/chapter_summaries.md", "c2");
        let c3 = make_source("story/character_matrix.md", "c3");
        let original = ContextPackage {
            chapter: 1,
            selected_context: vec![protected, c1, c2, c3],
        };
        let compiled = build_compiled_context_package(&original, "merged".to_string());
        // 1 protected + 1 compiled = 2
        assert_eq!(compiled.selected_context.len(), 2);
        assert_eq!(compiled.selected_context[0].source, "story/current_focus.md");
        assert_eq!(compiled.selected_context[1].source, "runtime/compiled-compressible-context");
    }

    #[test]
    fn test_is_protected_context_source_all_branches() {
        assert!(is_protected_context_source("runtime/chapter_memo"));
        assert!(is_protected_context_source("story/current_focus.md"));
        assert!(is_protected_context_source("story/author_intent.md"));
        assert!(is_protected_context_source("story/outline/story_frame.md"));
        assert!(is_protected_context_source("story/outline/story_frame.md#section"));
        assert!(is_protected_context_source("story/story_bible.md"));
        assert!(is_protected_context_source("story/outline/volume_map.md"));
        assert!(is_protected_context_source("story/outline/volume_map.md#ch5"));
        assert!(is_protected_context_source("story/volume_outline.md"));
        assert!(is_protected_context_source("story/parent_canon.md"));
        assert!(is_protected_context_source("story/fanfic_canon.md"));
        assert!(is_protected_context_source("story/current_state.md"));
        assert!(is_protected_context_source("story/pending_hooks.md#H001"));

        // compressible 段
        assert!(!is_protected_context_source("story/book_rules.md"));
        assert!(!is_protected_context_source("story/chapter_summaries.md"));
        assert!(!is_protected_context_source("story/character_matrix.md"));
        assert!(!is_protected_context_source("story/pending_hooks.md"));
    }
}
