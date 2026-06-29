//! Hook lifecycle management.
//!
//! Determines hook staleness, recycling thresholds, and chapter windows.

/// Check if a hook is within the chapter window (active recently enough).
pub fn is_hook_within_chapter_window(
    last_advanced_chapter: u32,
    start_chapter: u32,
    chapter_number: u32,
    keep_recent: u32,
) -> bool {
    let last_touch = last_advanced_chapter.max(start_chapter);
    if last_touch == 0 {
        return true;
    }
    chapter_number.saturating_sub(last_touch) <= keep_recent
}

/// Check if a hook is in a terminal status (resolved/closed).
pub fn is_terminal_status(status: &str) -> bool {
    let lower = status.trim().to_lowercase();
    matches!(lower.as_str(),
        "resolved" | "closed" | "done" | "已回收" | "已解决"
        | "deferred" | "paused" | "hold" | "延后" | "延期" | "搁置" | "暂缓"
    )
}

/// Check if a hook is future-planned (startChapter > current chapter).
pub fn is_future_planned_hook(start_chapter: u32, chapter_number: u32) -> bool {
    start_chapter > chapter_number
}

/// Compute the recycling threshold for a hook based on its status.
/// Hooks that have been silent longer than this threshold MUST be addressed.
pub fn recycle_threshold(status: &str, core_hook: bool) -> u32 {
    let lower = status.trim().to_lowercase();
    if lower.contains("pressured") || lower.contains("near_payoff")
        || lower.contains("progressing") || lower.contains("重大推进")
        || lower.contains("持续推进")
    {
        5
    } else if core_hook {
        8
    } else {
        10
    }
}

/// Compute hook silence (chapters since last touch).
pub fn hook_silence(start_chapter: u32, last_advanced_chapter: u32, chapter_number: u32) -> u32 {
    let last_touch = last_advanced_chapter.max(start_chapter);
    if last_touch == 0 {
        return chapter_number;
    }
    chapter_number.saturating_sub(last_touch)
}

/// Filter active hooks from a list.
pub fn filter_active_hooks<F: Fn(&str) -> bool>(
    hooks: &[(String, String, u32, u32, bool, F)],
) -> Vec<usize> {
    hooks.iter().enumerate()
        .filter(|(_, (_, status, _, _, _, _))| !is_terminal_status(status))
        .map(|(i, _)| i)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_status() {
        assert!(is_terminal_status("resolved"));
        assert!(is_terminal_status("已回收"));
        assert!(!is_terminal_status("open"));
        assert!(!is_terminal_status("pressured"));
    }

    #[test]
    fn test_recycle_threshold() {
        assert_eq!(recycle_threshold("pressured", false), 5);
        assert_eq!(recycle_threshold("open", true), 8);
        assert_eq!(recycle_threshold("open", false), 10);
    }

    #[test]
    fn test_hook_silence() {
        assert_eq!(hook_silence(5, 8, 12), 4);
        assert_eq!(hook_silence(10, 0, 15), 5);
    }
}
