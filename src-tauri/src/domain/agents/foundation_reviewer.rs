use async_trait::async_trait;
use crate::errors::AppError;
use crate::domain::story::BookConfig;
use super::base::{AgentContext, BaseAgent};
use super::types::AgentRole;
use super::architect::ArchitectOutput;

pub struct FoundationReviewerAgent;

impl Default for FoundationReviewerAgent {
    fn default() -> Self { Self }
}
impl FoundationReviewerAgent {
    pub fn new() -> Self { Self }

    /// Review foundation quality. Returns score 0-100 and pass/fail.
    pub async fn review(
        &self,
        ctx: &AgentContext,
        foundation: &ArchitectOutput,
        book: &BookConfig,
        language: &str,
    ) -> Result<FoundationReviewResult, AppError> {
        let system = build_review_system_prompt(language);
        let user = build_review_user_prompt(foundation, book, language);

        let response = self.chat(ctx, &system, &user).await?;
        let result = parse_review_result(&response.content)?;
        Ok(result)
    }
}

#[async_trait]
impl BaseAgent for FoundationReviewerAgent {
    fn role(&self) -> AgentRole {
        AgentRole::FoundationReviewer
    }

    fn name(&self) -> &str {
        "foundation-reviewer"
    }
}

pub struct FoundationReviewResult {
    pub passed: bool,
    pub total_score: u32,
    pub dimensions: Vec<DimensionScore>,
    pub overall_feedback: String,
}

pub struct DimensionScore {
    pub name: String,
    pub score: u32,
    pub feedback: String,
}

fn build_review_system_prompt(language: &str) -> String {
    match language {
        "en" => {
            r#"You are a story foundation quality reviewer. Audit the generated foundation for completeness, internal consistency, and potential issues.

## Review Dimensions (score each 0-20)

1. **World Consistency**: Are world rules internally consistent? Any contradictions?
2. **Character Depth**: Do characters have clear motivations, flaws, and arcs?
3. **Conflict Clarity**: Is the core conflict clear and compelling?
4. **Hook Setup**: Are initial hooks/foreshadowing properly seeded?
5. **Pacing Blueprint**: Does the volume map suggest good pacing?

## Output format (JSON)

{
  "passed": true/false,
  "total_score": 0-100,
  "dimensions": [
    { "name": "dimension_name", "score": 0-20, "feedback": "..." }
  ],
  "overall_feedback": "Overall assessment"
}

passed is true when total_score >= 60 and no dimension is below 8."#
        }
        _ => {
            r#"你是一位小说基础设定质量审核员。审核生成的基础设定的完整性、内部一致性和潜在问题。

## 审核维度（每项 0-20 分）

1. **世界观一致性**：世界规则是否内部自洽？有无矛盾？
2. **角色深度**：角色是否有清晰的动机、缺陷和成长弧线？
3. **冲突清晰度**：核心冲突是否清晰且有吸引力？
4. **伏笔设置**：初始伏笔/悬念是否恰当埋设？
5. **节奏蓝图**：卷纲是否暗示了良好的节奏？

## 输出格式（JSON）

{
  "passed": true/false,
  "total_score": 0-100,
  "dimensions": [
    { "name": "维度名称", "score": 0-20, "feedback": "..." }
  ],
  "overall_feedback": "整体评估"
}

total_score >= 60 且没有维度低于 8 分时 passed 为 true。"#
        }
    }.to_string()
}

fn build_review_user_prompt(foundation: &ArchitectOutput, book: &BookConfig, language: &str) -> String {
    let heading = if language == "en" {
        format!("Review foundation for \"{}\" ({})", book.title, book.genre)
    } else {
        format!("审核《{}》（{}题材）的基础设定", book.title, book.genre)
    };

    format!(
        "## {}\n\n### Story Frame\n\n{}\n\n### Volume Map\n\n{}\n\n### Book Rules\n\n{}\n\n### Roles\n\n{}\n\n### Pending Hooks\n\n{}",
        heading,
        foundation.story_frame,
        foundation.volume_map,
        foundation.book_rules,
        foundation.roles.iter().map(|r| format!("#### {} ({})\n{}", r.name, r.tier, r.content)).collect::<Vec<_>>().join("\n\n"),
        foundation.pending_hooks
    )
}

fn parse_review_result(content: &str) -> Result<FoundationReviewResult, AppError> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
        let passed = json.get("passed").and_then(|v| v.as_bool()).unwrap_or(false);
        let total_score = json.get("total_score")
            .or_else(|| json.get("overall_score"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let overall_feedback = json.get("overall_feedback")
            .or_else(|| json.get("summary"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let dimensions = json.get("dimensions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|item| {
                    Some(DimensionScore {
                        name: item.get("name")?.as_str()?.to_string(),
                        score: item.get("score").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                        feedback: item.get("feedback").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    })
                }).collect()
            })
            .unwrap_or_default();

        return Ok(FoundationReviewResult { passed, total_score, dimensions, overall_feedback });
    }

    Ok(FoundationReviewResult {
        passed: false,
        total_score: 0,
        dimensions: Vec::new(),
        overall_feedback: content.to_string(),
    })
}
