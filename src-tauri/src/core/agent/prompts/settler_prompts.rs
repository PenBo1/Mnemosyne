// Reflector（Settler）的 prompt 构建。
//
// 命名说明：inkos 中此角色叫 "Settler"，Mnemosyne 的 identity.rs / AGENT_ROLES
// 统一用 "reflector"。本文件保留 settler_prompts 命名以减少破坏性变更，
// 但内容已重写为适配 Mnemosyne 的 JSON StoryState 模型（而非 inkos 的
// markdown truth files）。
//
// 设计参考：inkos 的 Observer → Reflector 两阶段模式。
// - Observer (temperature 0.5)：宁多勿少地提取事实
// - Reflector (temperature 0.3)：保守地把观察结果合并为状态增量

use crate::features::story::{StoryState, ChapterSummary};

/// 构建 Reflector 的 system prompt。
///
/// `identity_prefix` 来自 AgentIdentity::build_system_prompt_with_memory，
/// 包含 SOUL.md / CONTEXT.md / MEMORY.md 内容。
pub fn build_system_prompt(language: &str, identity_prefix: Option<&str>) -> String {
    let task_prompt = match language {
        "en" => {
            r#"You are the state settlement specialist. Given the observer's extraction and the current story state, produce a state delta.

## Output format (strict JSON, no markdown fences)

{
  "updated_hooks": [
    {
      "hook_id": "<stable id, reuse existing if advancing, else new like 'hook-<chapter>-<slug>'>",
      "name": "<short hook name>",
      "hook_type": "foreshadowing|promise|mystery|relationship|thread",
      "start_chapter": <number>,
      "status": "open|progressing|deferred|resolved",
      "expected_payoff": "<semantic timing: immediate|near-term|mid-arc|slow-burn|endgame>",
      "last_advanced_chapter": <number or 0>,
      "core_hook": <true|false>,
      "notes": "<what changed this chapter>"
    }
  ],
  "chapter_summary": {
    "chapter": <number>,
    "title": "<title>",
    "characters": ["<names>"],
    "events": ["<key events>"],
    "state_changes": ["<state transitions>"],
    "hook_activity": ["<hook progressions>"],
    "mood": "<tone/atmosphere>",
    "chapter_type": "setup|rising_action|climax|falling_action|resolution|interlude"
  },
  "new_facts": [
    {
      "fact_id": "<stable id like 'fact-<chapter>-<slug>'>",
      "subject": "<entity>",
      "predicate": "<relation/attribute>",
      "object": "<value>",
      "source_chapter": <number>
    }
  ],
  "notes": ["<settlement rationale>"]
}

## Rules
- Only include CHANGES (delta), not the full state.
- Do not delete existing facts. Update or recontextualize instead.
- Reuse existing hook_id when advancing a known hook. Create new hook_id only for genuinely new hooks.
- "Mentioned again" or "restated" does NOT count as advancing a hook — omit it.
- Only record what actually happened in the chapter text. No inference, no prediction.
- Validate JSON schema before output."#
        }
        _ => {
            r#"你是状态结算专家。根据 Observer 的提取结果和当前故事状态，产出状态增量。

## 输出格式（严格 JSON，不要 markdown 代码围栏）

{
  "updated_hooks": [
    {
      "hook_id": "<稳定 ID，推进已有伏笔则复用，新建则用 'hook-<章号>-<slug>'>",
      "name": "<伏笔简短名称>",
      "hook_type": "foreshadowing|promise|mystery|relationship|thread",
      "start_chapter": <章号>,
      "status": "open|progressing|deferred|resolved",
      "expected_payoff": "<语义节奏: immediate|near-term|mid-arc|slow-burn|endgame>",
      "last_advanced_chapter": <章号或 0>,
      "core_hook": <true|false>,
      "notes": "<本章变化说明>"
    }
  ],
  "chapter_summary": {
    "chapter": <章号>,
    "title": "<标题>",
    "characters": ["<人物名>"],
    "events": ["<关键事件>"],
    "state_changes": ["<状态转变>"],
    "hook_activity": ["<伏笔推进>"],
    "mood": "<基调/氛围>",
    "chapter_type": "setup|rising_action|climax|falling_action|resolution|interlude"
  },
  "new_facts": [
    {
      "fact_id": "<稳定 ID 如 'fact-<章号>-<slug>'>",
      "subject": "<实体>",
      "predicate": "<关系/属性>",
      "object": "<值>",
      "source_chapter": <章号>
    }
  ],
  "notes": ["<结算理由>"]
}

## 规则
- 只包含变更（增量），不是完整状态
- 不要删除已有事实，只能更新或重新上下文化
- 推进已知伏笔时复用已有 hook_id；仅对真正的新伏笔创建新 hook_id
- "再次提到"或"换种说法重述"不算推进伏笔，应省略
- 只记录正文中实际发生的事，不推断、不预测、不补充大纲内容
- 输出前验证 JSON 格式"#
        }
    };

    match identity_prefix {
        Some(prefix) if !prefix.is_empty() => format!("{}\n\n{}", prefix, task_prompt),
        _ => task_prompt.to_string(),
    }
}

/// 构建 Reflector 的 user prompt。
///
/// 输入：章节内容、Observer 的提取结果（facts/hooks 文本摘要）、当前 StoryState。
pub fn build_user_prompt(
    chapter_number: u32,
    title: &str,
    content: &str,
    observations: &str,
    current_state: &StoryState,
    language: &str,
) -> String {
    let heading = if language == "en" {
        format!("Chapter {}: {}", chapter_number, title)
    } else {
        format!("第{}章 {}", chapter_number, title)
    };

    let current_hooks = format_current_hooks(current_state);
    let recent_summaries = format_recent_summaries(current_state);
    let current_facts = format_current_facts(current_state);

    let section_label = if language == "en" {
        ("## Chapter Content", "## Observer Extraction", "## Current Hooks", "## Recent Summaries", "## Current Facts")
    } else {
        ("## 本章正文", "## Observer 提取结果", "## 当前伏笔池", "## 最近章节摘要", "## 当前事实")
    };

    format!(
        "{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}",
        heading,
        section_label.0, truncate(content, 8000),
        section_label.1, observations,
        section_label.2, current_hooks,
        section_label.3, recent_summaries,
        section_label.4, current_facts,
    )
}

fn format_current_hooks(state: &StoryState) -> String {
    if state.hooks.is_empty() {
        return "(none)".to_string();
    }
    state.hooks.iter().take(30).map(|h| {
        format!("- [{}] {} (type={}, status={:?}, start={}, last_advanced={})",
            h.hook_id, h.name, h.hook_type, h.status, h.start_chapter, h.last_advanced_chapter)
    }).collect::<Vec<_>>().join("\n")
}

fn format_recent_summaries(state: &StoryState) -> String {
    if state.summaries.is_empty() {
        return "(none)".to_string();
    }
    let recent: Vec<&ChapterSummary> = state.summaries.iter().rev().take(5).collect();
    recent.iter().rev().map(|s| {
        format!("- Ch{} {}: characters={}, events={}, mood={}",
            s.chapter, s.title,
            s.characters.join(","), s.events.join(";"), s.mood)
    }).collect::<Vec<_>>().join("\n")
}

fn format_current_facts(state: &StoryState) -> String {
    if state.facts.is_empty() {
        return "(none)".to_string();
    }
    state.facts.iter().take(40).map(|f| {
        format!("- [{}] {} {} {} (from ch{})",
            f.fact_id, f.subject, f.predicate, f.object, f.source_chapter)
    }).collect::<Vec<_>>().join("\n")
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}\n\n[... truncated, showing first {} chars ...]", truncated, max_chars)
    }
}

/// 为 LLM 输出构造 Observer 提取结果的文本摘要。
///
/// 供 Reflector 的 user prompt 使用，把 Observer 的结构化输出转为可读文本。
pub fn format_observations(
    facts: &[crate::core::agent::observer::ExtractedFact],
    hooks_new: &[crate::core::agent::observer::HookAction],
    hooks_advanced: &[crate::core::agent::observer::HookAction],
) -> String {
    let mut sections = Vec::new();

    if !facts.is_empty() {
        let facts_text = facts.iter().map(|f| {
            format!("- [{}] {} {} {} ({})",
                f.category, f.subject, f.predicate, f.object, f.category)
        }).collect::<Vec<_>>().join("\n");
        sections.push(format!("### Facts\n{}", facts_text));
    }

    if !hooks_new.is_empty() {
        let hooks_text = hooks_new.iter().map(|h| {
            format!("- {} (type={}, status={}, desc={})", h.name, h.hook_type, h.status, h.description)
        }).collect::<Vec<_>>().join("\n");
        sections.push(format!("### New Hooks\n{}", hooks_text));
    }

    if !hooks_advanced.is_empty() {
        let hooks_text = hooks_advanced.iter().map(|h| {
            format!("- {} (type={}, status={}, desc={})", h.name, h.hook_type, h.status, h.description)
        }).collect::<Vec<_>>().join("\n");
        sections.push(format!("### Advanced Hooks\n{}", hooks_text));
    }

    if sections.is_empty() {
        "(no observations extracted)".to_string()
    } else {
        sections.join("\n\n")
    }
}
