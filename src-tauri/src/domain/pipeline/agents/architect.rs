// Architect Agent。
//
// 职责：建书时生成 5-SECTION 基础设定（story_frame / volume_map / roles / book_rules / pending_hooks）。
// 输出用 === SECTION: xxx === 分隔，解析后落盘到 outline/ + roles/ + story/ 目录。
//
// prompt 策略：保留 5-SECTION 结构 + 去重铁律 + 预算 + 完结检查。
// 精简冗长解释，保留关键要求。

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;
use super::super::types::BookConfig;

/// Architect 输出（5 个 SECTION）
#[derive(Debug, Clone, Default)]
pub struct ArchitectOutput {
    pub story_frame: String,
    pub volume_map: String,
    pub roles: String,
    pub book_rules: String,
    pub pending_hooks: String,
}

/// 生成基础设定。
pub async fn generate_foundation(
    engine: &AgentEngine,
    book: &BookConfig,
    genre_name: &str,
    genre_body: &str,
    external_context: Option<&str>,
) -> Result<ArchitectOutput, AppError> {
    let system_prompt = build_system_prompt(book, genre_name, genre_body, external_context);
    let user_message = format!(
        "Generate the complete foundation specification for the {} novel titled \"{}\".",
        genre_name, book.title
    );

    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    parse_sections(&response)
}

fn build_system_prompt(
    book: &BookConfig,
    genre_name: &str,
    genre_body: &str,
    external_context: Option<&str>,
) -> String {
    let context_block = match external_context {
        Some(ctx) if !ctx.trim().is_empty() => format!("\n\n## External Instructions\nThe following creative directives come from an external system. Weave them into the foundation:\n\n{}", ctx),
        _ => String::new(),
    };

    format!(
        r#"<identity>
You are the Architect Agent for this novel. Your sole output is a prose-dense foundation specification — not tables, not schemas, not bulleted item lists. Your prose density determines whether the Planner can extract sparse memos, whether the Writer can produce living characters, and whether the Reviewer can calibrate against hard facts.
</identity>

<responsibilities>
Produce the 5-section foundation that downstream agents (Planner, Writer, Reviewer) depend on. Each section is a deliverable: the Planner reads it to extract chapter memos, the Writer reads it to ground prose in facts, and the Reviewer reads it to detect drift. Your prose density is the load-bearing wall for the entire pipeline.
</responsibilities>{context_block}

## Book Metadata
- Platform: {platform}
- Genre: {genre_name} ({genre_id})
- Target chapters: {target_chapters} chapters
- Words per chapter: {chapter_word_count} words
- Title: {title}

## Genre Foundation
{genre_body}

## Output Structure (5 sections, strictly delimited by === SECTION: === blocks; do not omit any block)

<rules>
<rule name="deduplication">
Never repeat the same fact across multiple paragraphs. Each fact has exactly one authoritative home:
- Protagonist arc → only in `roles`
- World ironclad rules → only in `story_frame` worldview section
- Pacing principles → only in the final paragraph of `volume_map`
- Character current status → only in `roles` current-status section
- Initial hooks → only in `pending_hooks` (startChapter=0 rows)
</rule>

<rule name="budget">
Hard upper bounds. If you exceed a budget, trim ruthlessly before emitting:
- story_frame ≤ 3000 chars
- volume_map ≤ 5000 chars
- roles (total) ≤ 8000 chars
- book_rules ≤ 1000 chars
- pending_hooks ≤ 2000 chars
</rule>
</rules>

=== SECTION: story_frame ===

Prose skeleton, **4 paragraphs**, each roughly 600-900 characters. No tables. No bullet lists. Paragraph headings start with `##`.

### Paragraph 1: Theme and Tone
What this book is actually about — a specific proposition, not the empty phrase "how the protagonist grows from weak to strong." What is the tone, and why?

### Paragraph 2: Core Conflict, Antagonist Profile, Front-stage / Back-stage Story
What is the primary conflict? Who are the main antagonists (at least 2)? You must explicitly write both a "front-stage story" and a "back-stage story" line: the front stage is the surface conflict the reader sees every chapter; the back stage is the undercurrent that runs through the whole book. The two must be causally linked.

### Paragraph 3: Worldview Foundation (Ironclad Rules + Texture)
3-5 inviolable rules, written in prose. What is the texture of this world — wet or dry, fast or slow?

### Paragraph 4: Ending Direction + Whole-book Objective
Roughly what the final shot looks like. The paragraph must end with one explicit whole-book Objective sentence: the protagonist must reach a **verifiable ending state**.

=== SECTION: volume_map ===

Volume-level prose map, **5 main paragraphs + 1 closing paragraph on pacing principles**. Write at volume granularity only — never assign specific chapter tasks.

### Paragraph 1: Each volume's theme and emotional curve
### Paragraph 2: Inter-volume hooks and payoff promises (cover both front-stage and back-stage layers)
### Paragraph 3: Each volume's OKR (Objective + Key Results, 3 quantifiable KRs per volume)
### Paragraph 4: What must change at each volume's end
### Paragraph 5: Pacing principles (at least 3, made specific to this book; if 6, write 2-3 sentences each)

=== SECTION: roles ===

One prose card per character. The protagonist card is the sole authoritative source for the protagonist's arc. Separate each character with ---ROLE---:

---ROLE---
tier: major
name: <character name>
---CONTENT---
## Core Tags (3-5 keywords + one sentence)
## Contrast Details (1-2 concrete details that contrast with the core tags)
## Character Biography (past experiences)
## Protagonist Arc (start → end → cost) — required only for the protagonist
## Current Status (Chapter 0 initial state)
## Relationship Network
## Inner Drive
## Growth Arc

At least 3 major characters (protagonist + primary antagonist + primary collaborator). 3-5 minor characters; their simplified cards need only 4 sub-headings.

=== SECTION: book_rules ===

A plain Markdown rule card. No YAML, no JSON, no code blocks.
## Protagonist (name + personality lock + behavioral constraints)
## Genre Lock (primary genre + forbidden mix-ins)
## Narrative POV
## Prohibited Elements

=== SECTION: pending_hooks ===

Initial hook pool (Markdown table), Phase 7 extended columns:
| hook_id | start_chapter | type | status | last_advanced | expected_payoff | payoff_timing | upstream_dependency | payoff_volume | core | half_life | notes |

- During book creation, fill column 5 (last_advanced) with 0 uniformly.
- Column 7 (payoff_timing) must be one of: immediate / near-term / mid-term / slow-burn / endgame.
- 3-7 core_hook=true mainline load-bearing hooks.

## Hard Completion Check
You must emit all 5 SECTION blocks in order. The deliverable is complete only when the last row of `pending_hooks` has been written."#,
        context_block = context_block,
        platform = format!("{:?}", book.platform).to_lowercase(),
        genre_name = genre_name,
        genre_id = book.genre,
        target_chapters = book.target_chapters,
        chapter_word_count = book.chapter_word_count,
        title = book.title,
        genre_body = genre_body,
    )
}

/// 解析 === SECTION: xxx === 分隔的输出
pub fn parse_sections(content: &str) -> Result<ArchitectOutput, AppError> {
    let mut output = ArchitectOutput::default();
    let sections = ["story_frame", "volume_map", "roles", "book_rules", "pending_hooks"];

    let mut found_sections = Vec::new();
    for section_name in &sections {
        let marker = format!("=== SECTION: {} ===", section_name);
        if let Some(start) = content.find(&marker) {
            let content_start = start + marker.len();
            // 找下一个 === SECTION: === 或文本结尾
            let section_end = sections
                .iter()
                .filter_map(|s| {
                    let m = format!("=== SECTION: {} ===", s);
                    content[content_start..].find(&m).map(|pos| content_start + pos)
                })
                .min()
                .unwrap_or(content.len());
            let section_content = content[content_start..section_end].trim();
            match *section_name {
                "story_frame" => output.story_frame = section_content.to_string(),
                "volume_map" => output.volume_map = section_content.to_string(),
                "roles" => output.roles = section_content.to_string(),
                "book_rules" => output.book_rules = section_content.to_string(),
                "pending_hooks" => output.pending_hooks = section_content.to_string(),
                _ => {}
            }
            found_sections.push(*section_name);
        }
    }

    let missing: Vec<&str> = sections
        .iter()
        .copied()
        .filter(|s| !found_sections.contains(s))
        .collect();
    if !missing.is_empty() {
        return Err(AppError::invalid_format(format!(
            "Architect output missing sections: {}",
            missing.join(", ")
        )));
    }

    Ok(output)
}

/// 将 ArchitectOutput 落盘到书籍目录
pub fn persist_output(book_dir: &std::path::Path, output: &ArchitectOutput) -> Result<(), AppError> {
    let story_dir = book_dir.join("story");
    let outline_dir = story_dir.join("outline");
    std::fs::create_dir_all(&outline_dir)?;

    std::fs::write(outline_dir.join("story_frame.md"), &output.story_frame)?;
    std::fs::write(outline_dir.join("volume_map.md"), &output.volume_map)?;
    std::fs::write(story_dir.join("book_rules.md"), &output.book_rules)?;
    std::fs::write(story_dir.join("pending_hooks.md"), &output.pending_hooks)?;

    // roles 需要按 ---ROLE--- 分隔拆分为一人一文件
    persist_roles(&story_dir, &output.roles)?;

    Ok(())
}

/// 解析 roles SECTION 并按角色拆分文件
fn persist_roles(story_dir: &std::path::Path, roles_content: &str) -> Result<(), AppError> {
    let roles_major = story_dir.join("roles").join("主要角色");
    let roles_minor = story_dir.join("roles").join("次要角色");
    std::fs::create_dir_all(&roles_major)?;
    std::fs::create_dir_all(&roles_minor)?;

    // 按 ---ROLE--- 分割
    let parts: Vec<&str> = roles_content.split("---ROLE---").collect();
    for part in parts.iter().skip(1) {
        // 解析 tier 和 name
        let (tier, name, content) = parse_role_block(part)?;
        if name.is_empty() {
            continue;
        }
        let dir = if tier == "major" { &roles_major } else { &roles_minor };
        let file_path = dir.join(format!("{}.md", sanitize_filename(&name)));
        std::fs::write(file_path, content)?;
    }
    Ok(())
}

fn parse_role_block(block: &str) -> Result<(String, String, String), AppError> {
    let mut tier = String::new();
    let mut name = String::new();
    let mut content_start = 0;

    for (i, line) in block.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("tier:") {
            tier = trimmed[5..].trim().to_string();
        } else if trimmed.starts_with("name:") {
            name = trimmed[5..].trim().to_string();
        } else if trimmed.starts_with("---CONTENT---") {
            content_start = block.find("---CONTENT---").unwrap_or(0) + "---CONTENT---".len();
            break;
        }
        if i > 10 {
            break;
        }
    }

    let content = if content_start > 0 {
        block[content_start..].trim().to_string()
    } else {
        block.trim().to_string()
    };

    Ok((tier, name, content))
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_five_sections() {
        let content = r#"
=== SECTION: story_frame ===
故事框架内容

=== SECTION: volume_map ===
卷地图内容

=== SECTION: roles ===
---ROLE---
tier: major
name: 主角
---CONTENT---
角色卡

=== SECTION: book_rules ===
规则卡

=== SECTION: pending_hooks ===
伏笔池
"#;
        let output = parse_sections(content).unwrap();
        assert_eq!(output.story_frame, "故事框架内容");
        assert_eq!(output.volume_map, "卷地图内容");
        assert!(output.roles.contains("主角"));
        assert_eq!(output.book_rules, "规则卡");
        assert_eq!(output.pending_hooks, "伏笔池");
    }

    #[test]
    fn rejects_missing_sections() {
        let content = "=== SECTION: story_frame ===\n只有一段";
        assert!(parse_sections(content).is_err());
    }
}
