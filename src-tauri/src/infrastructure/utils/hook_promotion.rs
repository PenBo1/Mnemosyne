//! Hook promotion rules.
//!
//! Not every hook seed belongs in the live ledger. A seed is promoted when:
//! 1. core_hook === true -- architect marked it as main-line
//! 2. advanced_count >= 2 -- readers already track it
//! 3. depends_on non-empty -- has upstream causal dependencies
//! 4. cross_volume -- spans volume boundaries

use crate::infrastructure::utils::story_markdown::ParsedHook;

pub struct VolumeBoundary {
    pub name: String,
    pub start_ch: u32,
    pub end_ch: u32,
}

pub struct PromotionContext {
    pub volume_boundaries: Vec<VolumeBoundary>,
    pub current_chapter: u32,
    pub advanced_counts: std::collections::HashMap<String, u32>,
    pub all_seed_start_chapters: std::collections::HashMap<String, u32>,
}

pub struct PromotionDecision {
    pub promote: bool,
    pub reasons: Vec<PromotionReason>,
}

pub enum PromotionReason {
    CrossVolume,
    AdvancedCount,
    DependsOn,
    CoreHook,
}

/// Determine whether a hook should be promoted to the live ledger.
pub fn should_promote_hook(hook: &ParsedHook, context: &PromotionContext) -> PromotionDecision {
    let mut reasons = Vec::new();

    if hook.core_hook {
        reasons.push(PromotionReason::CoreHook);
    }

    let advanced_count = context.advanced_counts.get(&hook.hook_id).copied().unwrap_or(0);
    if advanced_count >= 2 {
        reasons.push(PromotionReason::AdvancedCount);
    }

    if is_cross_volume(hook, context) {
        reasons.push(PromotionReason::CrossVolume);
    }

    PromotionDecision {
        promote: !reasons.is_empty(),
        reasons,
    }
}

fn is_cross_volume(hook: &ParsedHook, context: &PromotionContext) -> bool {
    if context.volume_boundaries.len() < 2 {
        return false;
    }

    let seed_volume = find_volume_index(&context.volume_boundaries, hook.start_chapter);
    if seed_volume < 0 {
        return false;
    }

    if hook.start_chapter > 0 && seed_volume < context.volume_boundaries.len() as i32 - 1 {
        let first_vol_end = context.volume_boundaries[0].end_ch;
        if hook.start_chapter > first_vol_end {
            return true;
        }
    }

    false
}

fn find_volume_index(boundaries: &[VolumeBoundary], chapter: u32) -> i32 {
    for (i, vol) in boundaries.iter().enumerate() {
        if chapter >= vol.start_ch && chapter <= vol.end_ch {
            return i as i32;
        }
    }
    if chapter == 0 && !boundaries.is_empty() {
        return 0;
    }
    -1
}

/// Run a lightweight promotion pass: check advanced_count >= 2 and flip promoted flag.
pub fn rerun_promotion_pass(
    hooks: &[ParsedHook],
    summaries_raw: &str,
) -> PromotionPassResult {
    if hooks.is_empty() {
        return PromotionPassResult { updated: false, hooks: hooks.to_vec(), flipped_count: 0 };
    }

    let derived_counts = derive_advanced_counts_from_summaries(summaries_raw, hooks);
    let mut flipped = 0u32;

    let updated: Vec<ParsedHook> = hooks.iter().map(|hook| {
        if hook.promoted {
            return hook.clone();
        }
        let advanced = derived_counts.get(&hook.hook_id).copied().unwrap_or(0);
        if advanced >= 2 {
            flipped += 1;
            let mut new_hook = hook.clone();
            new_hook.promoted = true;
            new_hook
        } else {
            hook.clone()
        }
    }).collect();

    PromotionPassResult {
        updated: flipped > 0,
        flipped_count: flipped,
        hooks: updated,
    }
}

pub struct PromotionPassResult {
    pub updated: bool,
    pub flipped_count: u32,
    pub hooks: Vec<ParsedHook>,
}

/// Derive hook advancement counts from chapter_summaries.md content.
pub fn derive_advanced_counts_from_summaries(
    summaries_raw: &str,
    hooks: &[ParsedHook],
) -> std::collections::HashMap<String, u32> {
    let mut counts = std::collections::HashMap::new();
    if summaries_raw.trim().is_empty() || hooks.is_empty() {
        return counts;
    }

    let hook_activity_index = detect_hook_activity_column_index(summaries_raw);

    for hook in hooks {
        let pattern = regex::Regex::new(&format!(r"(?i)\b{}\b", regex::escape(&hook.hook_id))).unwrap();
        let mut count = 0u32;
        for line in summaries_raw.lines() {
            if !line.starts_with('|') || line.contains("---") {
                continue;
            }
            if let Some(cell) = extract_column(line, hook_activity_index) {
                if pattern.is_match(cell) {
                    count += 1;
                }
            }
        }
        if count > 0 {
            counts.insert(hook.hook_id.clone(), count);
        }
    }
    counts
}

fn detect_hook_activity_column_index(summaries_raw: &str) -> usize {
    const DEFAULT_INDEX: usize = 5;
    let header_re = regex::Regex::new(r"(?i)\|\s*(章节|Chapter)\s*\|").unwrap();
    let hook_activity_re = regex::Regex::new(r"(?i)^(伏笔动态|hookActivity)$").unwrap();
    for line in summaries_raw.lines() {
        if !line.starts_with('|') {
            continue;
        }
        if header_re.is_match(line) {
            let cols: Vec<&str> = line.split('|').map(|c| c.trim()).collect();
            let idx = cols.iter().position(|c| hook_activity_re.is_match(c));
            return idx.unwrap_or(DEFAULT_INDEX);
        }
    }
    DEFAULT_INDEX
}

fn extract_column(row: &str, index: usize) -> Option<&str> {
    let cols: Vec<&str> = row.split('|').collect();
    cols.get(index).map(|c| c.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hook(id: &str, start: u32, last_advanced: u32, core: bool, promoted: bool) -> ParsedHook {
        ParsedHook {
            hook_id: id.to_string(), name: id.to_string(), hook_type: "foreshadowing".into(),
            status: "open".into(), start_chapter: start, last_advanced_chapter: last_advanced,
            expected_payoff: "test".into(), core_hook: core, promoted,
        }
    }

    #[test]
    fn test_should_promote_core_hook() {
        let hook = make_hook("H001", 1, 1, true, false);
        let ctx = PromotionContext { volume_boundaries: vec![], current_chapter: 5, advanced_counts: std::collections::HashMap::new(), all_seed_start_chapters: std::collections::HashMap::new() };
        let decision = should_promote_hook(&hook, &ctx);
        assert!(decision.promote);
        assert!(decision.reasons.iter().any(|r| matches!(r, PromotionReason::CoreHook)));
    }

    #[test]
    fn test_should_promote_advanced_count() {
        let hook = make_hook("H002", 1, 3, false, false);
        let mut counts = std::collections::HashMap::new();
        counts.insert("H002".to_string(), 2);
        let ctx = PromotionContext { volume_boundaries: vec![], current_chapter: 5, advanced_counts: counts, all_seed_start_chapters: std::collections::HashMap::new() };
        let decision = should_promote_hook(&hook, &ctx);
        assert!(decision.promote);
        assert!(decision.reasons.iter().any(|r| matches!(r, PromotionReason::AdvancedCount)));
    }

    #[test]
    fn test_should_not_promote() {
        let hook = make_hook("H003", 1, 1, false, false);
        let ctx = PromotionContext { volume_boundaries: vec![], current_chapter: 5, advanced_counts: std::collections::HashMap::new(), all_seed_start_chapters: std::collections::HashMap::new() };
        let decision = should_promote_hook(&hook, &ctx);
        assert!(!decision.promote);
    }

    #[test]
    fn test_rerun_promotion_pass() {
        let hooks = vec![
            make_hook("H001", 1, 3, false, false),
            make_hook("H002", 1, 1, false, false),
        ];
        let summaries = "| 章节 | 标题 | 伏笔动态 |\n| --- | --- | --- |\n| 2 | Ch2 | H001 advance |\n| 3 | Ch3 | H001 advance |\n";
        let result = rerun_promotion_pass(&hooks, summaries);
        assert!(result.updated);
        assert_eq!(result.flipped_count, 1);
        assert!(result.hooks.iter().find(|h| h.hook_id == "H001").unwrap().promoted);
    }

    #[test]
    fn test_extract_column() {
        assert_eq!(extract_column("| a | b | c |", 1), Some("a"));
        assert_eq!(extract_column("| a | b | c |", 5), None);
    }
}
