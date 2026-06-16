use async_trait::async_trait;
use crate::errors::AppError;
use super::base::{AgentContext, BaseAgent};
use super::types::AgentRole;
use super::governance::*;
use super::prompts::planner_prompts;

pub struct PlannerAgent;

impl Default for PlannerAgent {
    fn default() -> Self { Self }
}
impl PlannerAgent {
    pub fn new() -> Self { Self }

    /// Plan the next chapter's intent and memo
    pub async fn plan_chapter(
        &self,
        ctx: &AgentContext,
        book_dir: &std::path::Path,
        chapter_number: u32,
        external_context: Option<&str>,
    ) -> Result<PlanOutput, AppError> {
        let language = read_book_language(book_dir).unwrap_or_else(|| "zh".to_string());
        let system = planner_prompts::build_system_prompt(&language);
        let user = planner_prompts::build_user_message(
            book_dir,
            chapter_number,
            external_context,
            &language,
        )?;

        let response = self.chat(ctx, &system, &user).await?;
        let output = parse_planner_output(&response.content, chapter_number)?;

        // Save intent to disk
        let runtime_dir = book_dir.join("story").join("runtime");
        std::fs::create_dir_all(&runtime_dir)?;
        let intent_path = runtime_dir.join(format!("chapter_{:04}_intent.md", chapter_number));
        let intent_content = render_intent_markdown(&output, chapter_number);
        std::fs::write(&intent_path, &intent_content)?;

        Ok(output)
    }
}

#[async_trait]
impl BaseAgent for PlannerAgent {
    fn role(&self) -> AgentRole {
        AgentRole::Planner
    }

    fn name(&self) -> &str {
        "planner"
    }
}

pub struct PlanOutput {
    pub intent: ChapterIntent,
    pub memo: ChapterMemo,
}

fn parse_planner_output(content: &str, chapter_number: u32) -> Result<PlanOutput, AppError> {
    let goal = extract_section(content, &["本章目标", "Chapter goal"]).unwrap_or_default();
    let is_golden_opening = chapter_number <= 3;

    let memo = ChapterMemo {
        chapter: chapter_number,
        goal: goal.clone(),
        is_golden_opening,
        body: content.to_string(),
        thread_refs: extract_list_items(content, &["关联线索", "Thread refs"]),
    };

    let intent = ChapterIntent {
        chapter: chapter_number,
        goal,
        outline_node: None,
        arc_context: None,
        must_keep: extract_list_items(content, &["must keep", "必须保持"]),
        must_avoid: extract_list_items(content, &["must avoid", "不要做", "Do not"]),
        style_emphasis: extract_list_items(content, &["style emphasis", "风格强调"]),
    };

    Ok(PlanOutput { intent, memo })
}

fn render_intent_markdown(output: &PlanOutput, _chapter_number: u32) -> String {
    let must_keep = if output.intent.must_keep.is_empty() {
        "- none".to_string()
    } else {
        output.intent.must_keep.iter().map(|i| format!("- {}", i)).collect::<Vec<_>>().join("\n")
    };
    let must_avoid = if output.intent.must_avoid.is_empty() {
        "- none".to_string()
    } else {
        output.intent.must_avoid.iter().map(|i| format!("- {}", i)).collect::<Vec<_>>().join("\n")
    };

    format!(
        "# Chapter Intent\n\n## Goal\n{}\n\n## Must Keep\n{}\n\n## Must Avoid\n{}\n\n## Memo\n\n{}",
        output.intent.goal, must_keep, must_avoid, output.memo.body
    )
}

fn extract_section(content: &str, headings: &[&str]) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim().to_lowercase();
        for heading in headings {
            if trimmed.contains(&heading.to_lowercase()) {
                let mut result = Vec::new();
                for next in lines.iter().skip(i + 1) {
                    let next = next.trim();
                    if next.starts_with('#') {
                        break;
                    }
                    if !next.is_empty() {
                        result.push(next.to_string());
                    }
                }
                if !result.is_empty() {
                    return Some(result.join("\n"));
                }
            }
        }
    }
    None
}

fn extract_list_items(content: &str, headings: &[&str]) -> Vec<String> {
    let section = extract_section(content, headings).unwrap_or_default();
    section.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            trimmed.strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
                .map(|s| s.to_string())
        })
        .collect()
}

fn read_book_language(book_dir: &std::path::Path) -> Option<String> {
    let config_path = book_dir.join("book.json");
    if let Ok(content) = std::fs::read_to_string(config_path) {
        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
            return config.get("language").and_then(|v| v.as_str()).map(|s| s.to_string());
        }
    }
    Some("zh".to_string())
}
