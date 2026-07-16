// Story Markdown 解析器。
//
// 提供 3 个解析器，从 markdown 真相文件构建结构化 JSON 索引：
// 1. parse_current_state_facts: current_state.md → Vec<CurrentStateFact>
// 2. parse_pending_hooks_markdown: pending_hooks.md → Vec<HookRecord>
// 3. parse_chapter_summaries_markdown: chapter_summaries.md → Vec<ChapterSummaryRow>
//
// 设计原则：纯函数 + 无 I/O。调用方负责读取 .md 文件内容传入。
// 兼容契约：表格优先 + bullet 列表 fallback。

use crate::domain::pipeline::state::types::{
    ChapterSummaryRow, CurrentStateFact, HookPayoffTiming, HookRecord, HookStatus,
};
use crate::domain::pipeline::types::Language;

// ── 1. parse_current_state_facts ──────────────────────────────

/// 解析 current_state.md → Vec<CurrentStateFact>。
///
/// 兼容两种 markdown 契约：
/// 1. 字段/值表格：`| 字段 | 值 |` 或 `| field | value |`
///    首列匹配预设 alias（当前位置/主角状态/当前目标/当前限制/当前敌我/当前冲突）→ subject="protagonist"
///    其余 → subject="current_state"
///    可选 `| 当前章节 | <n> |` 行提供 valid_from_chapter / source_chapter
/// 2. bullet 列表 fallback：`- <text>`，predicate=`note_{index+1}`
pub fn parse_current_state_facts(
    markdown: &str,
    fallback_chapter: u32,
    language: Language,
) -> Vec<CurrentStateFact> {
    let aliases = current_state_aliases(language);

    // 先尝试字段/值表格
    let table_facts = parse_current_state_table(markdown, fallback_chapter, &aliases);
    if !table_facts.is_empty() {
        return table_facts;
    }

    // fallback: bullet 列表
    parse_current_state_bullets(markdown, fallback_chapter)
}

/// 字段/值表格的 alias 表（中英文）
fn current_state_aliases(language: Language) -> [(&'static str, [&'static str; 2]); 6] {
    match language {
        Language::En => [
            ("current_location", ["Current Location", "当前位置"]),
            ("protagonist_state", ["Protagonist State", "主角状态"]),
            ("current_goal", ["Current Goal", "当前目标"]),
            ("current_constraint", ["Current Constraint", "当前限制"]),
            ("current_alliances", ["Current Alliances", "当前敌我"]),
            ("current_conflict", ["Current Conflict", "当前冲突"]),
        ],
        Language::Zh => [
            ("current_location", ["当前位置", "Current Location"]),
            ("protagonist_state", ["主角状态", "Protagonist State"]),
            ("current_goal", ["当前目标", "Current Goal"]),
            ("current_constraint", ["当前限制", "Current Constraint"]),
            ("current_alliances", ["当前敌我", "Current Alliances"]),
            ("current_conflict", ["当前冲突", "Current Conflict"]),
        ],
    }
}

fn parse_current_state_table(
    markdown: &str,
    fallback_chapter: u32,
    aliases: &[(&'static str, [&'static str; 2]); 6],
) -> Vec<CurrentStateFact> {
    // 提取「当前章节」字段
    let current_chapter = extract_current_chapter_from_table(markdown).unwrap_or(fallback_chapter);

    let mut facts = Vec::new();
    for line in markdown.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        if is_header_or_separator(trimmed) {
            continue;
        }
        let cols = split_table_row(trimmed);
        if cols.len() < 2 {
            continue;
        }
        let label = cols[0].trim();
        let value = cols[1].trim();
        if value.is_empty() {
            continue;
        }
        // 跳过「当前章节」行（已单独提取）
        if label.contains("当前章节") || label.eq_ignore_ascii_case("current chapter") {
            continue;
        }

        let (subject, predicate) = match resolve_alias(label, aliases) {
            Some((subj, pred)) => (subj, pred.to_string()),
            None => ("current_state", label.to_string()),
        };

        facts.push(CurrentStateFact {
            subject: subject.to_string(),
            predicate,
            object: value.to_string(),
            valid_from_chapter: current_chapter,
            valid_until_chapter: None,
            source_chapter: current_chapter,
        });
    }
    facts
}

fn extract_current_chapter_from_table(markdown: &str) -> Option<u32> {
    for line in markdown.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        let cols = split_table_row(trimmed);
        if cols.len() < 2 {
            continue;
        }
        let label = cols[0].trim();
        if label.contains("当前章节") || label.eq_ignore_ascii_case("current chapter") {
            let num_str: String = cols[1].chars().filter(|c| c.is_ascii_digit()).collect();
            if !num_str.is_empty() {
                return num_str.parse().ok();
            }
        }
    }
    None
}

fn resolve_alias<'a>(
    label: &str,
    aliases: &'a [(&'static str, [&'static str; 2]); 6],
) -> Option<(&'a str, &'a str)> {
    for (field_key, alias_pair) in aliases {
        for alias in alias_pair {
            if alias.eq_ignore_ascii_case(label) {
                return Some((field_key, alias_pair[0]));
            }
        }
    }
    None
}

fn parse_current_state_bullets(markdown: &str, fallback_chapter: u32) -> Vec<CurrentStateFact> {
    let mut facts = Vec::new();
    let mut idx = 0u32;
    for line in markdown.lines() {
        let trimmed = line.trim();
        let bullet = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| trimmed.strip_prefix("• "));
        if let Some(text) = bullet {
            let text = text.trim();
            if text.is_empty() {
                continue;
            }
            idx += 1;
            facts.push(CurrentStateFact {
                subject: "current_state".to_string(),
                predicate: format!("note_{}", idx),
                object: text.to_string(),
                valid_from_chapter: fallback_chapter,
                valid_until_chapter: None,
                source_chapter: fallback_chapter,
            });
        }
    }
    facts
}

// ── 2. parse_pending_hooks_markdown ───────────────────────────

/// 解析 pending_hooks.md → Vec<HookRecord>。
///
/// 兼容两种 markdown 契约：
/// 1. 表格：`| hook_id | 起始章节 | 类型 | 状态 | 最近推进 | 预期回收 | 回收节奏 | 上游依赖 | 回收卷 | 核心 | 半衰期 | 升级 | 备注 |`
///    首列需为合法 hookId（非表头）→ parse_pending_hook_row 映射 13 列
/// 2. legacy bullet 列表：`- <notes>` → 生成 hook-{index+1}, startChapter=0, type="unspecified", status="open"
pub fn parse_pending_hooks_markdown(markdown: &str, language: Language) -> Vec<HookRecord> {
    // 先尝试表格
    let table_hooks = parse_hooks_table(markdown, language);
    if !table_hooks.is_empty() {
        return table_hooks;
    }
    // fallback: bullet 列表
    parse_hooks_bullets(markdown)
}

fn parse_hooks_table(markdown: &str, language: Language) -> Vec<HookRecord> {
    let mut hooks = Vec::new();
    for line in markdown.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        if is_header_or_separator(trimmed) {
            continue;
        }
        let cols = split_table_row(trimmed);
        if cols.len() < 5 {
            continue;
        }
        let hook_id = cols[0].trim();
        if hook_id.is_empty() || !is_valid_hook_id(hook_id) {
            continue;
        }
        if let Some(hook) = parse_pending_hook_row(&cols, language) {
            hooks.push(hook);
        }
    }
    hooks
}

fn is_valid_hook_id(s: &str) -> bool {
    // hookId 应含至少一个非数字字符（避免把"1"等章节号误识别为 hookId）
    s.chars().any(|c| !c.is_whitespace() && !c.is_ascii_digit())
}

fn parse_pending_hook_row(cols: &[String], _language: Language) -> Option<HookRecord> {
    // 13 列：hook_id | start_chapter | type | status | last_advanced | expected_payoff |
    //        payoff_timing | depends_on | pays_off_in_arc | core_hook | half_life | promoted | notes
    let hook_id = cols.first()?.trim().to_string();
    let start_chapter = parse_u32_cell(cols.get(1)).unwrap_or(0);
    let type_ = cols.get(2).map(|s| s.trim().to_string()).unwrap_or_default();
    let status = parse_hook_status_cell(cols.get(3));
    let last_advanced_chapter = parse_u32_cell(cols.get(4)).unwrap_or(start_chapter);
    let expected_payoff = cols.get(5).map(|s| s.trim().to_string()).unwrap_or_default();
    let payoff_timing = parse_hook_payoff_timing_cell(cols.get(6));
    let depends_on = cols
        .get(7)
        .map(|s| parse_string_list_cell(s))
        .filter(|v| !v.is_empty());
    let pays_off_in_arc = cols
        .get(8)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let core_hook = cols
        .get(9)
        .and_then(|s| parse_bool_cell(s));
    let half_life_chapters = parse_u32_cell(cols.get(10));
    let promoted = cols.get(11).and_then(|s| parse_bool_cell(s));
    let notes = cols
        .get(12)
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    Some(HookRecord {
        hook_id,
        start_chapter,
        r#type: type_,
        status,
        last_advanced_chapter,
        expected_payoff,
        payoff_timing,
        notes,
        depends_on,
        pays_off_in_arc,
        core_hook,
        half_life_chapters,
        advanced_count: None,
        promoted,
    })
}

fn parse_hooks_bullets(markdown: &str) -> Vec<HookRecord> {
    let mut hooks = Vec::new();
    let mut idx = 0u32;
    for line in markdown.lines() {
        let trimmed = line.trim();
        let bullet = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| trimmed.strip_prefix("• "));
        if let Some(text) = bullet {
            let text = text.trim();
            if text.is_empty() {
                continue;
            }
            idx += 1;
            hooks.push(HookRecord {
                hook_id: format!("hook-{}", idx),
                start_chapter: 0,
                r#type: "unspecified".to_string(),
                status: HookStatus::Open,
                last_advanced_chapter: 0,
                expected_payoff: String::new(),
                payoff_timing: None,
                notes: text.to_string(),
                depends_on: None,
                pays_off_in_arc: None,
                core_hook: None,
                half_life_chapters: None,
                advanced_count: None,
                promoted: None,
            });
        }
    }
    hooks
}

// ── 3. parse_chapter_summaries_markdown ───────────────────────

/// 解析 chapter_summaries.md → Vec<ChapterSummaryRow>。
///
/// 表头（zh）：`| 章节 | 标题 | 出场人物 | 关键事件 | 状态变化 | 伏笔动态 | 情绪基调 | 章节类型 |`
/// 表头（en）：`| chapter | title | characters | events | stateChanges | hookActivity | mood | chapterType |`
///
/// 数据行首列必须为纯数字（章节号）。
pub fn parse_chapter_summaries_markdown(markdown: &str) -> Vec<ChapterSummaryRow> {
    let mut rows = Vec::new();
    for line in markdown.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        if is_header_or_separator(trimmed) {
            continue;
        }
        let cols = split_table_row(trimmed);
        if cols.is_empty() {
            continue;
        }
        let chapter_str = cols[0].trim();
        let chapter: u32 = match chapter_str.parse() {
            Ok(n) => n,
            Err(_) => continue, // 非章节号行跳过
        };

        rows.push(ChapterSummaryRow {
            chapter,
            title: cols.get(1).map(|s| s.trim().to_string()).unwrap_or_default(),
            characters: cols.get(2).map(|s| s.trim().to_string()).unwrap_or_default(),
            events: cols.get(3).map(|s| s.trim().to_string()).unwrap_or_default(),
            state_changes: cols.get(4).map(|s| s.trim().to_string()).unwrap_or_default(),
            hook_activity: cols.get(5).map(|s| s.trim().to_string()).unwrap_or_default(),
            mood: cols.get(6).map(|s| s.trim().to_string()).unwrap_or_default(),
            chapter_type: cols.get(7).map(|s| s.trim().to_string()).unwrap_or_default(),
        });
    }
    rows
}

// ── 共享工具函数 ─────────────────────────────────────────────

fn is_header_or_separator(line: &str) -> bool {
    let cols = split_table_row(line);
    if cols.is_empty() {
        return false;
    }

    // 分隔符行：所有单元格匹配 ^:?-+:?$ 模式（markdown table separator）
    let is_separator = cols.iter().all(|c| {
        let t = c.trim();
        t.contains('-') && t.chars().all(|ch| ch == '-' || ch == ':')
    });
    if is_separator {
        return true;
    }

    // 表头行：任一单元格精确匹配已知表头关键词（不再用 substring 匹配，避免误伤数据行）
    cols.iter().any(|c| {
        let t = c.trim().to_lowercase();
        matches!(t.as_str(),
            "章节" | "chapter" | "hook_id" | "field" | "字段"
            | "title" | "标题"
        )
    })
}

fn split_table_row(line: &str) -> Vec<String> {
    line.trim_start_matches('|')
        .trim_end_matches('|')
        .split('|')
        .map(|c| c.trim().to_string())
        .collect()
}

fn parse_u32_cell(cell: Option<&String>) -> Option<u32> {
    let s = cell?.trim();
    let num_str: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if num_str.is_empty() {
        None
    } else {
        num_str.parse().ok()
    }
}

fn parse_hook_status_cell(cell: Option<&String>) -> HookStatus {
    let s = cell.map(|s| s.trim().to_lowercase()).unwrap_or_default();
    match s.as_str() {
        "open" | "未启动" | "开启" => HookStatus::Open,
        "progressing" | "进行中" | "推进中" => HookStatus::Progressing,
        "deferred" | "搁置" | "延后" => HookStatus::Deferred,
        "resolved" | "已回收" | "已解决" => HookStatus::Resolved,
        _ => HookStatus::Open,
    }
}

fn parse_hook_payoff_timing_cell(cell: Option<&String>) -> Option<HookPayoffTiming> {
    let s = cell.map(|s| s.trim().to_lowercase()).unwrap_or_default();
    match s.as_str() {
        "immediate" | "即时" => Some(HookPayoffTiming::Immediate),
        "near-term" | "近期" | "短期" => Some(HookPayoffTiming::NearTerm),
        // "mid-term" 为 architect SYSTEM_PROMPT 中使用的别名，不可修改 prompt 文本，
        // 故在此处作为 MidArc 的解析别名兼容。
        "mid-arc" | "mid-term" | "中段" | "中期" => Some(HookPayoffTiming::MidArc),
        "slow-burn" | "慢热" | "长线" => Some(HookPayoffTiming::SlowBurn),
        "endgame" | "终局" => Some(HookPayoffTiming::Endgame),
        _ => None,
    }
}

fn parse_bool_cell(cell: &str) -> Option<bool> {
    let s = cell.trim().to_lowercase();
    match s.as_str() {
        "true" | "yes" | "1" | "是" | "✓" => Some(true),
        "false" | "no" | "0" | "否" | "✗" | "" => Some(false),
        _ => None,
    }
}

fn parse_string_list_cell(cell: &str) -> Vec<String> {
    cell.trim()
        .split([',', '，', ';', '；'])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

// ── 4. render_hooks_markdown（HookRecord → markdown 表格）─────

/// 将 HookRecord 列表渲染为 markdown 表格（写回 pending_hooks.md）。
///
/// 13 列表格，zh/en 双表头。
/// 空列表返回 "- none"。
pub fn render_hooks_markdown(hooks: &[HookRecord], language: Language) -> String {
    if hooks.is_empty() {
        return "- none".to_string();
    }

    let is_en = matches!(language, Language::En);
    let header = if is_en {
        "| hook_id | start_chapter | type | status | last_advanced | expected_payoff | payoff_timing | depends_on | pays_off_in_arc | core_hook | half_life | promoted | notes |\n\
         | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    } else {
        "| hook_id | 起始章节 | 类型 | 状态 | 最近推进 | 预期回收 | 回收节奏 | 上游依赖 | 回收卷 | 核心 | 半衰期 | 升级 | 备注 |\n\
         | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    };

    let mut lines = vec![header.to_string()];
    for hook in hooks {
        let cells = vec![
            hook.hook_id.clone(),
            hook.start_chapter.to_string(),
            hook.r#type.clone(),
            render_hook_status(hook.status),
            hook.last_advanced_chapter.to_string(),
            hook.expected_payoff.clone(),
            hook.payoff_timing
                .map(|t| render_payoff_timing(t, language))
                .unwrap_or_default(),
            render_depends_on_cell(hook.depends_on.as_deref(), is_en),
            hook.pays_off_in_arc.clone().unwrap_or_default(),
            hook.core_hook
                .map(|v| render_bool_cell(v, is_en))
                .unwrap_or_default(),
            hook.half_life_chapters
                .map(|v| v.to_string())
                .unwrap_or_default(),
            hook.promoted
                .map(|v| render_bool_cell(v, is_en))
                .unwrap_or_default(),
            hook.notes.clone(),
        ];
        let escaped: Vec<String> = cells.iter().map(|c| escape_table_cell(c)).collect();
        lines.push(format!("| {} |", escaped.join(" | ")));
    }

    lines.join("\n")
}

fn render_hook_status(status: HookStatus) -> String {
    match status {
        HookStatus::Open => "open".to_string(),
        HookStatus::Progressing => "progressing".to_string(),
        HookStatus::Deferred => "deferred".to_string(),
        HookStatus::Resolved => "resolved".to_string(),
    }
}

fn render_payoff_timing(timing: HookPayoffTiming, language: Language) -> String {
    match (timing, language) {
        (HookPayoffTiming::Immediate, Language::En) => "immediate".to_string(),
        (HookPayoffTiming::Immediate, _) => "即时".to_string(),
        (HookPayoffTiming::NearTerm, Language::En) => "near-term".to_string(),
        (HookPayoffTiming::NearTerm, _) => "近期".to_string(),
        (HookPayoffTiming::MidArc, Language::En) => "mid-arc".to_string(),
        (HookPayoffTiming::MidArc, _) => "中弧".to_string(),
        (HookPayoffTiming::SlowBurn, Language::En) => "slow-burn".to_string(),
        (HookPayoffTiming::SlowBurn, _) => "慢热".to_string(),
        (HookPayoffTiming::Endgame, Language::En) => "endgame".to_string(),
        (HookPayoffTiming::Endgame, _) => "终局".to_string(),
    }
}

fn render_depends_on_cell(depends_on: Option<&[String]>, is_en: bool) -> String {
    match depends_on {
        Some(ids) if !ids.is_empty() => format!("[{}]", ids.join(", ")),
        _ => if is_en { "none" } else { "无" }.to_string(),
    }
}

fn render_bool_cell(value: bool, is_en: bool) -> String {
    if is_en {
        if value { "true".to_string() } else { "false".to_string() }
    } else if value { "是".to_string() } else { "否".to_string() }
}

fn escape_table_cell(cell: &str) -> String {
    cell.replace('|', "\\|").replace('\n', " ").replace('\r', "")
}

// ── 测试 ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_current_state_table_zh() {
        let md = "| 字段 | 值 |\n|---|---|\n| 当前章节 | 12 |\n| 当前位置 | 码头 |\n| 主角状态 | 受伤 |";
        let facts = parse_current_state_facts(md, 1, Language::Zh);
        assert_eq!(facts.len(), 2);
        assert_eq!(facts[0].predicate, "当前位置");
        assert_eq!(facts[0].object, "码头");
        assert_eq!(facts[0].subject, "current_location");
        assert_eq!(facts[0].valid_from_chapter, 12);
        assert_eq!(facts[1].predicate, "主角状态");
        assert_eq!(facts[1].subject, "protagonist_state");
    }

    #[test]
    fn parses_current_state_bullets_fallback() {
        let md = "- 主角受伤\n- 在码头";
        let facts = parse_current_state_facts(md, 5, Language::Zh);
        assert_eq!(facts.len(), 2);
        assert_eq!(facts[0].predicate, "note_1");
        assert_eq!(facts[0].object, "主角受伤");
        assert_eq!(facts[0].subject, "current_state");
        assert_eq!(facts[0].source_chapter, 5);
    }

    #[test]
    fn parses_pending_hooks_table() {
        let md = "| hook_id | 起始章节 | 类型 | 状态 | 最近推进 | 预期回收 | 回收节奏 | 上游依赖 | 回收卷 | 核心 | 半衰期 | 升级 | 备注 |\n\
                  |---|---|---|---|---|---|---|---|---|---|---|---|---|\n\
                  | mentor-oath | 8 | relationship | progressing | 12 | 揭示真相 | slow-burn |  | 第二卷 | true | 5 | false | 推进 |";
        let hooks = parse_pending_hooks_markdown(md, Language::Zh);
        assert_eq!(hooks.len(), 1);
        let h = &hooks[0];
        assert_eq!(h.hook_id, "mentor-oath");
        assert_eq!(h.start_chapter, 8);
        assert_eq!(h.r#type, "relationship");
        assert_eq!(h.status, HookStatus::Progressing);
        assert_eq!(h.last_advanced_chapter, 12);
        assert_eq!(h.expected_payoff, "揭示真相");
        assert_eq!(h.payoff_timing, Some(HookPayoffTiming::SlowBurn));
        assert_eq!(h.core_hook, Some(true));
        assert_eq!(h.half_life_chapters, Some(5));
        assert_eq!(h.promoted, Some(false));
        assert_eq!(h.notes, "推进");
    }

    #[test]
    fn parses_pending_hooks_bullets_fallback() {
        let md = "- 主角发现藏宝图\n- 神秘老人留下警告";
        let hooks = parse_pending_hooks_markdown(md, Language::Zh);
        assert_eq!(hooks.len(), 2);
        assert_eq!(hooks[0].hook_id, "hook-1");
        assert_eq!(hooks[0].start_chapter, 0);
        assert_eq!(hooks[0].r#type, "unspecified");
        assert_eq!(hooks[0].status, HookStatus::Open);
        assert_eq!(hooks[0].notes, "主角发现藏宝图");
    }

    #[test]
    fn parses_chapter_summaries_table() {
        let md = "| 章节 | 标题 | 出场人物 | 关键事件 | 状态变化 | 伏笔动态 | 情绪基调 | 章节类型 |\n\
                  |---|---|---|---|---|---|---|---|\n\
                  | 1 | 暗流 | 陆承锦 | 遇见老人 | 受伤 | H007 推进 | 紧张 | main |\n\
                  | 2 | 夜行 | 陆承锦 | 进入码头 | 抵达 | H008 埋下 | 神秘 | main |";
        let rows = parse_chapter_summaries_markdown(md);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].chapter, 1);
        assert_eq!(rows[0].title, "暗流");
        assert_eq!(rows[0].characters, "陆承锦");
        assert_eq!(rows[0].hook_activity, "H007 推进");
        assert_eq!(rows[1].chapter, 2);
    }

    #[test]
    fn skips_non_chapter_rows_in_summaries() {
        let md = "| 章节 | 标题 |\n|---|---|\n| 1 | 暗流 |\n| 附录 | 备注 |";
        let rows = parse_chapter_summaries_markdown(md);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].chapter, 1);
    }

    #[test]
    fn handles_empty_markdown() {
        assert!(parse_current_state_facts("", 1, Language::Zh).is_empty());
        assert!(parse_pending_hooks_markdown("", Language::Zh).is_empty());
        assert!(parse_chapter_summaries_markdown("").is_empty());
    }
}
