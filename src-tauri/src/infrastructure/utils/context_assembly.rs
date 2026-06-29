use crate::core::agent::governance::*;
use crate::core::agent::planner::PlanOutput;

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
