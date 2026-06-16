//! Hook governance -- admission, disposition, stale debt collection.

use crate::domain::utils::story_markdown::ParsedHook;

pub enum HookDisposition {
    None,
    Mention,
    Advance,
    Resolve,
    Defer,
}

/// Collect stale hook debt: hooks that need attention.
pub fn collect_stale_hook_debt(
    hooks: &[ParsedHook],
    chapter_number: u32,
    stale_after_chapters: u32,
) -> Vec<&ParsedHook> {
    hooks.iter()
        .filter(|h| !is_terminal_status(&h.status))
        .filter(|h| h.start_chapter <= chapter_number)
        .filter(|h| {
            let silence = chapter_number.saturating_sub(h.last_advanced_chapter.max(h.start_chapter));
            silence > stale_after_chapters
        })
        .collect()
}

/// Evaluate whether a new hook candidate should be admitted.
pub fn evaluate_hook_admission(
    candidate_type: &str,
    candidate_payoff: &str,
    active_hooks: &[ParsedHook],
) -> HookAdmissionDecision {
    if candidate_type.trim().is_empty() {
        return HookAdmissionDecision {
            admit: false,
            reason: "missing_type".to_string(),
            matched_hook_id: None,
        };
    }
    if candidate_payoff.trim().is_empty() {
        return HookAdmissionDecision {
            admit: false,
            reason: "missing_payoff_signal".to_string(),
            matched_hook_id: None,
        };
    }

    let normalized_type = normalize_text(candidate_type);
    let normalized_payoff = normalize_text(candidate_payoff);

    for hook in active_hooks {
        let hook_type = normalize_text(&hook.hook_type);
        if normalized_type != hook_type {
            continue;
        }

        let hook_payoff = normalize_text(&hook.expected_payoff);
        if normalized_payoff == hook_payoff {
            return HookAdmissionDecision {
                admit: false,
                reason: "duplicate_family".to_string(),
                matched_hook_id: Some(hook.hook_id.clone()),
            };
        }
    }

    HookAdmissionDecision {
        admit: true,
        reason: "admit".to_string(),
        matched_hook_id: None,
    }
}

pub struct HookAdmissionDecision {
    pub admit: bool,
    pub reason: String,
    pub matched_hook_id: Option<String>,
}

fn is_terminal_status(status: &str) -> bool {
    let lower = status.trim().to_lowercase();
    matches!(lower.as_str(),
        "resolved" | "closed" | "done" | "已回收" | "已解决"
        | "deferred" | "paused" | "延后" | "延期" | "搁置" | "暂缓"
    )
}

fn normalize_text(value: &str) -> String {
    let re = regex::Regex::new(r"[^a-z0-9\u{4e00}-\u{9fff}]+").unwrap();
    let binding = value.trim().to_lowercase();
    let normalized = re.replace_all(&binding, " ");
    normalized.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hook(id: &str, hook_type: &str, payoff: &str) -> crate::domain::utils::story_markdown::ParsedHook {
        crate::domain::utils::story_markdown::ParsedHook {
            hook_id: id.to_string(), name: id.to_string(), hook_type: hook_type.to_string(),
            status: "open".into(), start_chapter: 1, last_advanced_chapter: 1,
            expected_payoff: payoff.to_string(), core_hook: false, promoted: false,
        }
    }

    #[test]
    fn test_evaluate_hook_admission_missing_type() {
        let decision = evaluate_hook_admission("", "test payoff", &[]);
        assert!(!decision.admit);
        assert_eq!(decision.reason, "missing_type");
    }

    #[test]
    fn test_evaluate_hook_admission_missing_payoff() {
        let decision = evaluate_hook_admission("foreshadowing", "", &[]);
        assert!(!decision.admit);
        assert_eq!(decision.reason, "missing_payoff_signal");
    }

    #[test]
    fn test_evaluate_hook_admission_admit() {
        let hooks = vec![make_hook("H001", "mystery", "solve the case")];
        let decision = evaluate_hook_admission("foreshadowing", "reveal the secret", &hooks);
        assert!(decision.admit);
    }

    #[test]
    fn test_evaluate_hook_admission_duplicate() {
        let hooks = vec![make_hook("H001", "foreshadowing", "reveal the secret")];
        let decision = evaluate_hook_admission("foreshadowing", "reveal the secret", &hooks);
        assert!(!decision.admit);
        assert_eq!(decision.reason, "duplicate_family");
    }

    #[test]
    fn test_collect_stale_hook_debt() {
        let hooks = vec![make_hook("H001", "open", "test")];
        let stale = collect_stale_hook_debt(&hooks, 20, 10);
        assert_eq!(stale.len(), 1);
    }
}
