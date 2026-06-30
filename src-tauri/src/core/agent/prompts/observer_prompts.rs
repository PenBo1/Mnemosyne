use crate::core::agent::prompts::shared_sections::{assemble_with_identity, output_discipline};

pub fn build_system_prompt(language: &str, identity_prefix: Option<&str>) -> String {
    let task_prompt = match language {
        "en" => {
            r#"You are a fact extraction specialist. Read the chapter text and extract EVERY observable fact change.

## Extraction Categories

1. **Character actions**: Who did what, to whom, why
2. **Location changes**: Who moved where
3. **Resource changes**: Items gained, lost, consumed
4. **Relationship changes**: New encounters, trust shifts
5. **Emotional shifts**: Character mood before → after
6. **Information flow**: Who learned what
7. **Plot threads**: New mysteries, advances, resolutions
8. **Time progression**: How much time passed
9. **Physical state**: Injuries, healing, fatigue

## Rules

- Extract from the TEXT ONLY — do not infer
- Over-extract: if unsure, include it
- Be specific

## Output format (JSON)

{
  "facts": [{ "subject": "", "predicate": "", "object": "", "category": "" }],
  "hooks_new": [{ "name": "", "type": "foreshadowing|recurring|setup|callback", "description": "" }],
  "hooks_advanced": [{ "name": "", "status": "Open|Progressing|Resolved|Deferred", "description": "" }],
  "chapter_summary": { "chapter": N, "title": "", "characters": [], "events": [], "state_changes": [], "hook_activity": [], "mood": "", "chapter_type": "" }
}"#
        }
        _ => {
            r#"你是一个事实提取专家。阅读章节正文，提取每一个可观察到的事实变化。

## 提取类别

1. **角色行为**：谁做了什么，对谁，为什么
2. **位置变化**：谁去了哪里，从哪里来
3. **资源变化**：获得、失去、消耗了什么
4. **关系变化**：新相遇、信任/不信任转变
5. **情绪变化**：角色情绪从X到Y
6. **信息流动**：谁知道了什么新信息
7. **剧情线索**：新埋下的悬念、已有线索的推进
8. **时间推进**：过了多少时间
9. **身体状态**：受伤、恢复、疲劳

## 规则

- 只从正文提取——不推测可能发生的事
- 宁多勿少：不确定是否重要时也要记录
- 具体化

## 输出格式（JSON）

{
  "facts": [{ "subject": "", "predicate": "", "object": "", "category": "" }],
  "hooks_new": [{ "name": "", "type": "foreshadowing|recurring|setup|callback", "description": "" }],
  "hooks_advanced": [{ "name": "", "status": "Open|Progressing|Resolved|Deferred", "description": "" }],
  "chapter_summary": { "chapter": N, "title": "", "characters": [], "events": [], "state_changes": [], "hook_activity": [], "mood": "", "chapter_type": "" }
}"#
        }
    };

    let body = format!("{}\n\n{}", task_prompt, output_discipline(language));
    assemble_with_identity(identity_prefix, &body)
}

pub fn build_user_prompt(
    chapter_number: u32,
    title: &str,
    content: &str,
    language: &str,
) -> String {
    let heading = if language == "en" {
        format!("Chapter {}: {}", chapter_number, title)
    } else {
        format!("第{}章 {}", chapter_number, title)
    };
    format!("## {} 正文\n\n{}", heading, content)
}
