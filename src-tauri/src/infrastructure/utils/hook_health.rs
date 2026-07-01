//! Hook health analysis.

use crate::features::story::{AuditIssue, AuditSeverity};
use crate::infrastructure::utils::story_markdown::ParsedHook;

pub const DEFAULT_MAX_ACTIVE_HOOKS: u32 = 12;
pub const DEFAULT_STALE_AFTER_CHAPTERS: u32 = 10;
pub const DEFAULT_NO_ADVANCE_WINDOW: u32 = 5;

pub struct HookHealthParams {
    pub language: String,
    pub chapter_number: u32,
    pub target_chapters: Option<u32>,
    pub hooks: Vec<ParsedHook>,
    pub max_active_hooks: Option<u32>,
    pub stale_after_chapters: Option<u32>,
    pub no_advance_window: Option<u32>,
}

/// Analyze hook health and return audit issues.
pub fn analyze_hook_health(params: HookHealthParams) -> Vec<AuditIssue> {
    let max_active = params.max_active_hooks.unwrap_or(DEFAULT_MAX_ACTIVE_HOOKS);
    let no_advance_window = params.no_advance_window.unwrap_or(DEFAULT_NO_ADVANCE_WINDOW);
    let is_en = params.language == "en";

    let active_hooks: Vec<&ParsedHook> = params.hooks.iter()
        .filter(|h| !is_terminal_status(&h.status))
        .collect();

    let mut issues = Vec::new();

    if active_hooks.len() as u32 > max_active {
        issues.push(AuditIssue {
            severity: AuditSeverity::Warning,
            category: if is_en { "Hook Debt" } else { "伏笔债务" }.to_string(),
            description: if is_en {
                format!("There are {} active hooks, above the recommended cap of {}.", active_hooks.len(), max_active)
            } else {
                format!("当前有 {} 个活跃伏笔，已经高于建议上限 {} 个。", active_hooks.len(), max_active)
            },
            suggestion: if is_en {
                "Prefer advancing, resolving, or deferring existing debt before opening more hooks.".to_string()
            } else {
                "优先推进、回收或延后已有伏笔，再继续开新伏笔。".to_string()
            },
            repair_scope: None,
        });
    }

    let stale_hooks: Vec<&&ParsedHook> = active_hooks.iter()
        .filter(|h| {
            let silence = params.chapter_number.saturating_sub(h.last_advanced_chapter.max(h.start_chapter));
            silence > params.stale_after_chapters.unwrap_or(DEFAULT_STALE_AFTER_CHAPTERS)
        })
        .collect();

    if !stale_hooks.is_empty() && stale_hooks.len() <= 3 {
        let hook_ids: Vec<&str> = stale_hooks.iter().map(|h| h.hook_id.as_str()).collect();
        issues.push(AuditIssue {
            severity: AuditSeverity::Warning,
            category: if is_en { "Hook Debt" } else { "伏笔债务" }.to_string(),
            description: if is_en {
                format!("Hooks under stale pressure: {}", hook_ids.join(", "))
            } else {
                format!("陈旧伏笔：{}", hook_ids.join("、"))
            },
            suggestion: if is_en {
                "Move one pressured hook with a real payoff before opening adjacent debt.".to_string()
            } else {
                "先让一个已进入压力区的伏笔发生真实推进，再继续扩展同类债务。".to_string()
            },
            repair_scope: None,
        });
    }

    let latest_advance = active_hooks.iter()
        .map(|h| h.last_advanced_chapter)
        .max()
        .unwrap_or(0);

    if !active_hooks.is_empty()
        && params.chapter_number.saturating_sub(latest_advance) >= no_advance_window
        && stale_hooks.is_empty()
    {
        issues.push(AuditIssue {
            severity: AuditSeverity::Warning,
            category: if is_en { "Hook Debt" } else { "伏笔债务" }.to_string(),
            description: if is_en {
                format!("No real hook advancement has landed for {} chapters.",
                    params.chapter_number.saturating_sub(latest_advance))
            } else {
                format!("已经连续 {} 章没有真实伏笔推进。",
                    params.chapter_number.saturating_sub(latest_advance))
            },
            suggestion: if is_en {
                "Schedule one old hook for real movement instead of opening parallel restatements.".to_string()
            } else {
                "下一章优先让一个旧伏笔发生真实推进，而不是继续平行重述。".to_string()
            },
            repair_scope: None,
        });
    }

    issues
}

fn is_terminal_status(status: &str) -> bool {
    let lower = status.trim().to_lowercase();
    matches!(lower.as_str(),
        "resolved" | "closed" | "done" | "已回收" | "已解决"
        | "deferred" | "paused" | "延后" | "延期" | "搁置" | "暂缓"
    )
}
