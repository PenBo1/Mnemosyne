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
use crate::core::agent::prompts::shared_sections::{assemble_with_identity, output_discipline};

/// 构建 Reflector 的 system prompt。
///
/// `identity_prefix` 来自 AgentIdentity::build_system_prompt_with_memory，
/// 包含 SOUL.md / CONTEXT.md / MEMORY.md 内容。
///
/// S3 升级：输出格式与 inkos `RuntimeStateDeltaSchema` 对齐，使用 `hook_ops`
/// 显式区分 upsert / mention / resolve / defer 四种操作。
pub fn build_system_prompt(language: &str, identity_prefix: Option<&str>) -> String {
    let task_prompt = match language {
        "en" => {
            r#"You are the state settlement specialist. Given the observer's extraction and the current story state, produce a state delta.

## Output format (strict JSON, no markdown fences)

{
  "hook_ops": {
    "upsert": [
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
    "mention": ["<hook_id of hooks merely referenced, not advanced>"],
    "resolve": ["<hook_id of hooks paid off this chapter>"],
    "defer": ["<hook_id of hooks intentionally deferred>"]
  },
  "new_hook_candidates": [
    {
      "type": "foreshadowing|promise|mystery|relationship|thread",
      "expected_payoff": "<semantic timing>",
      "notes": "<what kind of hook this would be>"
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
- Use `hook_ops.upsert` to advance a known hook (reuse existing hook_id) or to create a new hook with explicit id.
- Use `new_hook_candidates` to suggest a new hook without committing to a specific hook_id (system will assign canonical id).
- Use `hook_ops.mention` when an existing hook is referenced but NOT advanced (status unchanged).
- Use `hook_ops.resolve` when an existing hook is paid off this chapter (will be marked resolved).
- Use `hook_ops.defer` when intentionally delaying a hook's progression.
- "Mentioned again" or "restated" does NOT count as advancing a hook — put it in `mention`, not `upsert`.
- Only record what actually happened in the chapter text. No inference, no prediction.
- All four `hook_ops` arrays are optional; omit empty arrays if desired.
- Validate JSON schema before output."#
        }
        _ => {
            r#"你是状态结算专家。根据 Observer 的提取结果和当前故事状态，产出状态增量。

## 输出格式（严格 JSON，不要 markdown 代码围栏）

{
  "hook_ops": {
    "upsert": [
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
    "mention": ["<仅被提及、未推进的 hook_id>"],
    "resolve": ["<本章兑现的 hook_id>"],
    "defer": ["<有意推迟的 hook_id>"]
  },
  "new_hook_candidates": [
    {
      "type": "foreshadowing|promise|mystery|relationship|thread",
      "expected_payoff": "<语义节奏>",
      "notes": "<这是何种伏笔>"
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
- 用 `hook_ops.upsert` 推进已知伏笔（复用已有 hook_id）或创建有明确 id 的新伏笔
- 用 `new_hook_candidates` 提议新伏笔但不指定 hook_id（系统会分配规范 id）
- 用 `hook_ops.mention` 表示已有伏笔被提及但未推进（status 不变）
- 用 `hook_ops.resolve` 表示已有伏笔本章兑现（将标记为 resolved）
- 用 `hook_ops.defer` 表示有意推迟推进某伏笔
- "再次提到"或"换种说法重述"不算推进伏笔，应放入 `mention`，不放入 `upsert`
- 只记录正文中实际发生的事，不推断、不预测、不补充大纲内容
- 四个 `hook_ops` 子数组都是可选的，空数组可省略
- 输出前验证 JSON 格式"#
        }
    };

    let body = format!("{}\n\n{}", task_prompt, output_discipline(language));
    assemble_with_identity(identity_prefix, &body)
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
