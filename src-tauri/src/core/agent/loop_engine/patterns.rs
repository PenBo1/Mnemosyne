use crate::infrastructure::db::models::LoopPattern;

pub struct PatternRegistry;

impl PatternRegistry {
    pub fn built_in_patterns() -> Vec<LoopPattern> {
        vec![
            Self::make_pattern(
                "daily-triage",
                "Daily Triage",
                "Scan all chapters and tasks, generate a prioritized report of what needs attention.",
                "1d",
                "low",
                vec!["discover", "triage", "report"],
            ),
            Self::make_pattern(
                "chapter-quality-check",
                "Chapter Quality Check",
                "Run quality audit on the most recently completed chapter.",
                "per-chapter",
                "low",
                vec!["discover", "audit", "report"],
            ),
            Self::make_pattern(
                "dependency-audit",
                "Dependency Audit",
                "Check character and setting reference consistency across chapters.",
                "6h",
                "medium",
                vec!["discover", "scan", "verify", "report"],
            ),
            Self::make_pattern(
                "pipeline-health-monitor",
                "Pipeline Health Monitor",
                "Monitor pipeline run success rates, durations, and error patterns.",
                "1d",
                "low",
                vec!["discover", "analyze", "report"],
            ),
            Self::make_pattern(
                "token-budget-watcher",
                "Token Budget Watcher",
                "Track token consumption across all loops and pipelines, alert on overspend.",
                "1d",
                "low",
                vec!["discover", "measure", "alert"],
            ),
            Self::make_pattern(
                "character-consistency-checker",
                "Character Consistency Checker",
                "Verify character behavior, appearance, and relationship consistency.",
                "per-chapter",
                "medium",
                vec!["discover", "scan", "verify", "report"],
            ),
            Self::make_pattern(
                "plot-hole-detector",
                "Plot Hole Detector",
                "Detect unresolved plot threads, logical inconsistencies, and dropped threads.",
                "5-chapters",
                "high",
                vec!["discover", "analyze", "verify", "report", "escalate"],
            ),
        ]
    }

    fn make_pattern(
        id: &str,
        name: &str,
        goal: &str,
        cadence: &str,
        risk: &str,
        phases: Vec<&str>,
    ) -> LoopPattern {
        let now = chrono::Utc::now().to_rfc3339();
        LoopPattern {
            id: id.to_string(),
            name: name.to_string(),
            description: goal.to_string(),
            goal: goal.to_string(),
            cadence: cadence.to_string(),
            risk_level: risk.to_string(),
            phases: phases
                .into_iter()
                .map(|p| serde_json::json!({"name": p, "description": "", "type": "discover"}))
                .collect(),
            human_gates: vec![],
            cost_config: serde_json::json!({
                "tokens_noop": 500,
                "tokens_report": 2000,
                "tokens_action": 5000,
                "daily_cap": 50000,
                "early_exit_required": true,
            }),
            skills_required: vec![],
            state_schema: serde_json::json!({}),
            is_active: true,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}
