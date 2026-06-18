pub fn build_system_prompt(language: &str, identity_prefix: Option<&str>) -> String {
    let task_prompt = match language {
        "en" => {
            r#"You are a state management specialist. Given the observer's extraction results and the current story state, produce a state delta that updates the truth files.

## Output format (JSON)

{
  "updated_state": "<updated current_state.md content>",
  "updated_hooks": "<updated pending_hooks.md content>",
  "chapter_summary": "<one-row table entry for chapter_summaries.md>",
  "updated_subplots": "<updated subplot_board.md content>",
  "updated_emotional_arcs": "<updated emotional_arcs.md content>",
  "updated_character_matrix": "<updated character_matrix.md content>"
}

## Rules
- Only include CHANGES (delta), not the full state
- Do not delete existing facts
- Validate JSON schema before output"#
        }
        _ => {
            r#"你是一位状态管理专家。根据观察结果和当前故事状态，产出状态增量来更新真相文件。

## 输出格式（JSON）

{
  "updated_state": "<更新后的 current_state.md 内容>",
  "updated_hooks": "<更新后的 pending_hooks.md 内容>",
  "chapter_summary": "<chapter_summaries.md 的一行表格条目>",
  "updated_subplots": "<更新后的 subplot_board.md 内容>",
  "updated_emotional_arcs": "<更新后的 emotional_arcs.md 内容>",
  "updated_character_matrix": "<更新后的 character_matrix.md 内容>"
}

## 规则
- 只包含变更（增量），不是完整状态
- 不要删除已有事实
- 输出前验证 JSON 格式"#
        }
    };

    match identity_prefix {
        Some(prefix) if !prefix.is_empty() => format!("{}\n\n{}", prefix, task_prompt),
        _ => task_prompt.to_string(),
    }
}

pub fn build_user_message(
    chapter_number: u32,
    title: &str,
    content: &str,
    book_dir: &std::path::Path,
    observations: &str,
    language: &str,
) -> Result<String, crate::errors::AppError> {
    let story_dir = book_dir.join("story");

    let read_safe = |path: &std::path::Path| -> String {
        std::fs::read_to_string(path).unwrap_or_default()
    };

    let current_state = read_safe(&story_dir.join("current_state.md"));
    let pending_hooks = read_safe(&story_dir.join("pending_hooks.md"));
    let chapter_summaries = read_safe(&story_dir.join("chapter_summaries.md"));
    let subplot_board = read_safe(&story_dir.join("subplot_board.md"));
    let emotional_arcs = read_safe(&story_dir.join("emotional_arcs.md"));
    let character_matrix = read_safe(&story_dir.join("character_matrix.md"));

    let heading = if language == "en" {
        format!("Chapter {}: {}", chapter_number, title)
    } else {
        format!("第{}章 {}", chapter_number, title)
    };

    Ok(format!(
        "## {} 内容\n\n{}\n\n## 观察结果\n\n{}\n\n## 当前状态\n\n{}\n\n## 伏笔池\n\n{}\n\n## 章节摘要\n\n{}\n\n## 支线板\n\n{}\n\n## 情感弧线\n\n{}\n\n## 角色矩阵\n\n{}",
        heading, content, observations, current_state, pending_hooks,
        chapter_summaries, subplot_board, emotional_arcs, character_matrix
    ))
}
