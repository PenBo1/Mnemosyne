use super::types::ProjectHarness;

/// Embedded global harness configuration.
///
/// All config is compiled into the binary.
/// No file-based loading — config lives in code only.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GlobalHarnessConfig {
    pub project: ProjectHarness,
}

impl GlobalHarnessConfig {
    pub fn new() -> Self {
        Self {
            project: ProjectHarness {
                version: "0.1.0".into(),
                agent_constraints: super::types::AgentConstraints {
                    max_turns_per_session: 50,
                    max_tool_calls_per_turn: 10,
                    required_output_format: super::types::OutputFormat::Structured,
                    forbidden_patterns: vec![],
                    role_isolation: std::collections::HashMap::new(),
                },
                tool_constraints: super::types::ToolConstraints {
                    tool_permissions: std::collections::HashMap::new(),
                    rate_limits: std::collections::HashMap::new(),
                    approval_required: vec![],
                },
                pipeline_config: super::types::PipelineHarnessConfig {
                    stage_order: vec![
                        "plan".into(),
                        "compose".into(),
                        "write".into(),
                        "settle".into(),
                        "audit".into(),
                        "revise".into(),
                    ],
                    required_stages: vec![
                        "plan".into(),
                        "compose".into(),
                        "write".into(),
                        "settle".into(),
                        "audit".into(),
                    ],
                    conditional_stages: vec![super::types::ConditionalStage {
                        stage: "revise".into(),
                        condition: "audit_failed".into(),
                    }],
                    max_revision_rounds: 3,
                    auto_revise_threshold: 60.0,
                    gate_config: std::collections::HashMap::new(),
                },
                quality_gates: vec![
                    super::types::QualityGate {
                        id: "g_write_wordcount".into(),
                        name: "Word Count".into(),
                        stage: "audit".into(),
                        gate_type: super::types::GateType::WordCountRange,
                        threshold: 3000.0,
                        dimension: None,
                        action_on_fail: super::types::GateAction::Revise,
                    },
                    super::types::QualityGate {
                        id: "g_write_forbidden".into(),
                        name: "Forbidden Phrases".into(),
                        stage: "audit".into(),
                        gate_type: super::types::GateType::ForbiddenPattern,
                        threshold: 0.0,
                        dimension: Some("值得一提的是|不禁|缓缓|仿佛|宛如".into()),
                        action_on_fail: super::types::GateAction::Revise,
                    },
                    super::types::QualityGate {
                        id: "g_audit_score".into(),
                        name: "Audit Score".into(),
                        stage: "audit".into(),
                        gate_type: super::types::GateType::ScoreThreshold,
                        threshold: 70.0,
                        dimension: None,
                        action_on_fail: super::types::GateAction::Revise,
                    },
                    super::types::QualityGate {
                        id: "g_audit_critical".into(),
                        name: "Critical Issues".into(),
                        stage: "audit".into(),
                        gate_type: super::types::GateType::IssueCount,
                        threshold: 0.0,
                        dimension: None,
                        action_on_fail: super::types::GateAction::Revise,
                    },
                    super::types::QualityGate {
                        id: "g_plan_completeness".into(),
                        name: "Plan Completeness".into(),
                        stage: "plan".into(),
                        gate_type: super::types::GateType::CompletenessCheck,
                        threshold: 0.0,
                        dimension: Some("must_keep|must_avoid|focus_points".into()),
                        action_on_fail: super::types::GateAction::Revise,
                    },
                    super::types::QualityGate {
                        id: "g_compose_completeness".into(),
                        name: "Context Completeness".into(),
                        stage: "compose".into(),
                        gate_type: super::types::GateType::CompletenessCheck,
                        threshold: 0.0,
                        dimension: Some("chapter_intent|relevant_facts|active_hooks".into()),
                        action_on_fail: super::types::GateAction::Revise,
                    },
                ],
                context_engine: super::types::ContextEngineConfig {
                    max_system_prompt_tokens: 8000,
                    max_context_window_tokens: 32000,
                    protected_sections: vec![
                        "chapter_intent".into(),
                        "author_intent".into(),
                        "current_focus".into(),
                    ],
                    context_priorities: vec![
                        "chapter_intent".into(),
                        "chapter_context".into(),
                        "story_state".into(),
                        "active_hooks".into(),
                        "relevant_facts".into(),
                        "constraint_lessons".into(),
                    ],
                    compaction_threshold: 0.8,
                },
                feedback_rules: vec![
                    super::types::FeedbackRule {
                        id: "fr_ooc".into(),
                        trigger: super::types::FeedbackTrigger {
                            error_type: "ooc_violation".into(),
                            min_occurrences: 3,
                            scope: "novel".into(),
                        },
                        constraint: "角色行为出现 OOC，必须严格遵守已建立的性格特征".into(),
                        target: super::types::FeedbackTarget::AllAgents,
                        cooldown_chapters: 5,
                    },
                    super::types::FeedbackRule {
                        id: "fr_timeline".into(),
                        trigger: super::types::FeedbackTrigger {
                            error_type: "timeline_error".into(),
                            min_occurrences: 2,
                            scope: "novel".into(),
                        },
                        constraint: "时间线出现矛盾，必须维护事件的时间顺序一致性".into(),
                        target: super::types::FeedbackTarget::AuditDimension,
                        cooldown_chapters: 5,
                    },
                    super::types::FeedbackRule {
                        id: "fr_lore".into(),
                        trigger: super::types::FeedbackTrigger {
                            error_type: "lore_conflict".into(),
                            min_occurrences: 2,
                            scope: "novel".into(),
                        },
                        constraint: "设定出现自相矛盾，必须维护世界观的一致性".into(),
                        target: super::types::FeedbackTarget::AllAgents,
                        cooldown_chapters: 10,
                    },
                    super::types::FeedbackRule {
                        id: "fr_ai_flavor".into(),
                        trigger: super::types::FeedbackTrigger {
                            error_type: "ai_flavor".into(),
                            min_occurrences: 3,
                            scope: "novel".into(),
                        },
                        constraint: "文本出现 AI 痕迹，必须使用自然的中文表达".into(),
                        target: super::types::FeedbackTarget::WritingRules,
                        cooldown_chapters: 3,
                    },
                ],
                gc_policy: super::types::GcPolicy {
                    stale_snapshot_days: 90,
                    max_snapshots_per_novel: 200,
                    compact_state_every_n_chapters: 10,
                    archive_completed_novels: true,
                },
            },
        }
    }

    pub fn project(&self) -> &ProjectHarness {
        &self.project
    }
}

impl Default for GlobalHarnessConfig {
    fn default() -> Self {
        Self::new()
    }
}
