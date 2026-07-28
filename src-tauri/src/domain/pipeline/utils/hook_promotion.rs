//! ═══════════════════════════════════════════════════════════════════════════
//! Pipeline Utils Hook Promotion - Hook 晋升工具
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 核心逻辑：
//! 1. 从 chapter_summaries.md 的「伏笔动态」列（第 5 列）按 `\bhookId\b` 正则统计
//!    每个 hook 在历史章节中被推进的次数（advancedCount）。
//! 2. 对每个 promoted !== true 的 hook，若 advancedCount >= 2，则晋升为 promoted=true。
//! 3. 返回 PromotionPassResult { updated, hooks, flipped_count }。
//!
//! 设计原则：纯函数 + 无 LLM 调用。所有 I/O 由调用方完成。

use crate::domain::pipeline::state::types::HookRecord;

/// 晋升通过结果
#[derive(Debug, Clone)]
pub struct PromotionPassResult {
    /// 是否有 hook 被翻转
    pub updated: bool,
    /// 更新后的 hooks 列表
    pub hooks: Vec<HookRecord>,
    /// 翻转数量
    pub flipped_count: u32,
}

/// 从 chapter_summaries.md 内容中重新推导 advancedCount 并执行晋升。
///
/// 输入：
/// - hooks: 当前 pending_hooks 的 HookRecord 列表
/// - summaries_raw: chapter_summaries.md 完整内容（用于推导 advancedCount）
///
/// 输出：PromotionPassResult
///
/// 算法：
/// 1. 解析 summaries_raw 为表格行，提取第 5 列（hookActivity）。
/// 2. 对每个 hook_id，统计其在所有 hookActivity 单元格中作为 word 出现的次数。
/// 3. derived_count = max(原 advancedCount, 统计值)
/// 4. 若 derived_count >= 2 且 promoted != true → 翻转为 promoted=true
pub fn rerun_promotion_pass(
    hooks: &[HookRecord],
    summaries_raw: &str,
) -> PromotionPassResult {
    let activity_cells = extract_hook_activity_cells(summaries_raw);

    let mut flipped_count = 0u32;
    let mut next_hooks: Vec<HookRecord> = Vec::with_capacity(hooks.len());

    for hook in hooks {
        let derived = derive_advanced_count(&hook.hook_id, &activity_cells);
        let merged_count = hook.advanced_count.unwrap_or(0).max(derived);

        let should_promote = hook.promoted != Some(true) && merged_count >= 2;

        let mut updated_hook = hook.clone();
        updated_hook.advanced_count = Some(merged_count);
        if should_promote {
            updated_hook.promoted = Some(true);
            flipped_count += 1;
        }

        next_hooks.push(updated_hook);
    }

    PromotionPassResult {
        updated: flipped_count > 0,
        hooks: next_hooks,
        flipped_count,
    }
}

// ── 内部辅助 ─────────────────────────────────────────────────

/// 从 chapter_summaries.md 中提取所有「伏笔动态」列单元格内容。
///
/// 表格格式（zh / en）：
/// ```text
/// | 章节 | 标题 | 出场人物 | 关键事件 | 状态变化 | 伏笔动态 | 情绪基调 | 章节类型 |
/// | 1   | ... | ...     | ...     | ...     | H007 推进 | ...     | ...      |
/// ```
///
/// 第 6 列（索引 5）即「伏笔动态」/「hookActivity」。
fn extract_hook_activity_cells(summaries_raw: &str) -> Vec<String> {
    let mut cells = Vec::new();
    for line in summaries_raw.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        // 跳过表头与分隔行
        if is_header_or_separator_line(trimmed) {
            continue;
        }
        let cols = split_table_row(trimmed);
        if cols.len() >= 6 {
            cells.push(cols[5].clone());
        }
    }
    cells
}

fn is_header_or_separator_line(line: &str) -> bool {
    line.contains("---")
        || line.contains("章节")
        || line.contains("chapter")
        || line.contains("Chapter")
}

/// 按管道符切分表格行，返回各列 trim 后的内容。
fn split_table_row(line: &str) -> Vec<String> {
    line.trim_start_matches('|')
        .trim_end_matches('|')
        .split('|')
        .map(|c| c.trim().to_string())
        .collect()
}

/// 统计 hook_id 在 activity_cells 中作为 word 出现的总次数。
///
/// - ASCII hook_id：手动 word-boundary 检查（前后字符不是 `[a-zA-Z0-9_-]`）。
///   不用 regex `\b`：`-` 是非单词字符，`\b` 会让 "mentor-oath" 误匹配
///   "mentor-oath-longer"。也不用 lookbehind：Rust `regex` crate 不支持
///   look-around。
/// - 非 ASCII hook_id（如含中文）：退化为纯子串匹配（regex 不支持中文 `\b`）。
fn derive_advanced_count(hook_id: &str, activity_cells: &[String]) -> u32 {
    if hook_id.is_empty() {
        return 0;
    }
    let mut count = 0u32;
    for cell in activity_cells {
        if hook_id.is_ascii() {
            count += count_ascii_word_occurrences(cell, hook_id);
        } else {
            count += cell.matches(hook_id).count() as u32;
        }
    }
    count
}

/// 统计 ASCII needle 在 text 中作为"词"出现的次数。
/// 词边界：前后字符不是 `[a-zA-Z0-9_-]`。
fn count_ascii_word_occurrences(text: &str, needle: &str) -> u32 {
    debug_assert!(needle.is_ascii() && !needle.is_empty());
    let text_bytes = text.as_bytes();
    let n = needle.len();
    let mut count = 0u32;
    let mut search_from = 0;
    while let Some(rel) = text[search_from..].find(needle) {
        let abs = search_from + rel;
        let after = abs + n;
        let before_ok = abs == 0 || !is_word_char(text_bytes[abs - 1]);
        let after_ok = after >= text.len() || !is_word_char(text_bytes[after]);
        if before_ok && after_ok {
            count += 1;
        }
        search_from = after;
    }
    count
}

fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

// ── 测试 ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::pipeline::state::types::HookStatus;

    fn make_hook(id: &str, advanced_count: Option<u32>, promoted: Option<bool>) -> HookRecord {
        HookRecord {
            hook_id: id.to_string(),
            start_chapter: 1,
            r#type: "mystery".to_string(),
            status: HookStatus::Open,
            last_advanced_chapter: 1,
            expected_payoff: String::new(),
            payoff_timing: None,
            notes: String::new(),
            depends_on: None,
            pays_off_in_arc: None,
            core_hook: None,
            half_life_chapters: None,
            advanced_count,
            promoted,
        }
    }

    #[test]
    fn promotes_hook_when_advanced_count_reaches_two() {
        let summaries = "| 章节 | 标题 | 人物 | 事件 | 变化 | 伏笔动态 | 情绪 | 类型 |\n\
                         |---|---|---|---|---|---|---|---|\n\
                         | 1 | t | p | e | s | H007 推进 | m | main |\n\
                         | 2 | t | p | e | s | H007 再次推进 | m | main |";
        let hooks = vec![make_hook("H007", None, None)];
        let result = rerun_promotion_pass(&hooks, summaries);
        assert!(result.updated);
        assert_eq!(result.flipped_count, 1);
        assert_eq!(result.hooks[0].promoted, Some(true));
        assert_eq!(result.hooks[0].advanced_count, Some(2));
    }

    #[test]
    fn does_not_promote_when_already_promoted() {
        let summaries = "| 章节 | 标题 | 人物 | 事件 | 变化 | 伏笔动态 | 情绪 | 类型 |\n\
                         |---|---|---|---|---|---|---|---|\n\
                         | 1 | t | p | e | s | H007 推进 | m | main |\n\
                         | 2 | t | p | e | s | H007 再次推进 | m | main |";
        let hooks = vec![make_hook("H007", None, Some(true))];
        let result = rerun_promotion_pass(&hooks, summaries);
        assert!(!result.updated);
        assert_eq!(result.flipped_count, 0);
        // advancedCount 仍应更新（即使已 promoted）
        assert_eq!(result.hooks[0].advanced_count, Some(2));
    }

    #[test]
    fn does_not_promote_when_only_one_occurrence() {
        let summaries = "| 章节 | 标题 | 人物 | 事件 | 变化 | 伏笔动态 | 情绪 | 类型 |\n\
                         |---|---|---|---|---|---|---|---|\n\
                         | 1 | t | p | e | s | H007 推进 | m | main |";
        let hooks = vec![make_hook("H007", None, None)];
        let result = rerun_promotion_pass(&hooks, summaries);
        assert!(!result.updated);
        assert_eq!(result.hooks[0].promoted, None);
        assert_eq!(result.hooks[0].advanced_count, Some(1));
    }

    #[test]
    fn preserves_existing_advanced_count_if_higher() {
        let summaries = "| 章节 | 标题 | 人物 | 事件 | 变化 | 伏笔动态 | 情绪 | 类型 |\n\
                         |---|---|---|---|---|---|---|---|\n\
                         | 1 | t | p | e | s | H007 推进 | m | main |";
        // 已有 advanced_count=5，但 summaries 中只出现 1 次 → 应保留 5
        let hooks = vec![make_hook("H007", Some(5), None)];
        let result = rerun_promotion_pass(&hooks, summaries);
        // 5 >= 2 应触发晋升
        assert!(result.updated);
        assert_eq!(result.hooks[0].advanced_count, Some(5));
        assert_eq!(result.hooks[0].promoted, Some(true));
    }

    #[test]
    fn handles_empty_summaries() {
        let hooks = vec![make_hook("H007", None, None)];
        let result = rerun_promotion_pass(&hooks, "");
        assert!(!result.updated);
        assert_eq!(result.hooks[0].advanced_count, Some(0));
    }

    #[test]
    fn handles_ascii_hook_id_with_word_boundary() {
        // 确保 hookId-startup 不会匹配 hookId-startup-longer
        let summaries = "| 章节 | 标题 | 人物 | 事件 | 变化 | 伏笔动态 | 情绪 | 类型 |\n\
                         |---|---|---|---|---|---|---|---|\n\
                         | 1 | t | p | e | s | mentor-oath advanced | m | main |\n\
                         | 2 | t | p | e | s | mentor-oath advanced | m | main |\n\
                         | 3 | t | p | e | s | mentor-oath-longer advanced | m | main |";
        let hooks = vec![make_hook("mentor-oath", None, None)];
        let result = rerun_promotion_pass(&hooks, summaries);
        // mentor-oath 出现 2 次（行 1、2），mentor-oath-longer 不应被误匹配
        assert!(result.updated);
        assert_eq!(result.hooks[0].advanced_count, Some(2));
    }
}
