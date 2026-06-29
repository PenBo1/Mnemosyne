use async_trait::async_trait;
use crate::shared::errors::AppError;
use crate::features::story::{AuditResult, AuditIssue, AuditSeverity};
use crate::infrastructure::file_storage::data_dir::DataDir;
use super::base::{AgentContext, BaseAgent};
use super::types::AgentRole;
use super::prompts::auditor_prompts;
use super::agent_identity::AgentIdentity;

pub struct ContinuityAuditor;

impl Default for ContinuityAuditor {
    fn default() -> Self { Self }
}
impl ContinuityAuditor {
    pub fn new() -> Self { Self }

    /// Audit a chapter across 37 dimensions
    pub async fn audit_chapter(
        &self,
        ctx: &AgentContext,
        book_dir: &std::path::Path,
        chapter_number: u32,
        data_dir: &DataDir,
    ) -> Result<AuditResult, AppError> {
        let language = read_book_language(book_dir).unwrap_or_else(|| "zh".to_string());
        let chapter_content = read_chapter_content(book_dir, chapter_number)?;

        let identity = AgentIdentity::load(data_dir, "auditor");
        let task_query = format!("audit chapter {} for continuity issues", chapter_number);
        let identity_prefix = identity.build_system_prompt_with_memory(
            &ctx.memory, &task_query, ctx.skill_manager.as_deref(), ctx.user_profile.as_deref(),
        ).await;
        let system = auditor_prompts::build_system_prompt(&language, Some(&identity_prefix));
        let user = auditor_prompts::build_user_message(
            chapter_number,
            &chapter_content,
            book_dir,
            &language,
        );

        let response = self.chat(ctx, &system, &user).await?;
        let result = parse_audit_result(&response.content)?;
        Ok(result)
    }
}

#[async_trait]
impl BaseAgent for ContinuityAuditor {
    fn role(&self) -> AgentRole {
        AgentRole::Auditor
    }

    fn name(&self) -> &str {
        "continuity-auditor"
    }
}

/// The 37 audit dimensions.
pub const AUDIT_DIMENSIONS: &[(u32, &str, &str)] = &[
    (1, "OOC检查", "OOC Check"),
    (2, "时间线检查", "Timeline Check"),
    (3, "设定冲突", "Lore Conflict Check"),
    (4, "战力崩坏", "Power Scaling Check"),
    (5, "数值检查", "Numerical Consistency Check"),
    (6, "伏笔检查", "Hook Check"),
    (7, "节奏检查", "Pacing Check"),
    (8, "文风检查", "Style Check"),
    (9, "信息越界", "Information Boundary Check"),
    (10, "词汇疲劳", "Lexical Fatigue Check"),
    (11, "利益链断裂", "Incentive Chain Check"),
    (12, "年代考据", "Era Accuracy Check"),
    (13, "配角降智", "Side Character Competence Check"),
    (14, "配角工具人化", "Side Character Instrumentalization Check"),
    (15, "爽点虚化", "Payoff Dilution Check"),
    (16, "台词失真", "Dialogue Authenticity Check"),
    (17, "流水账", "Chronicle Drift Check"),
    (18, "知识库污染", "Knowledge Base Pollution Check"),
    (19, "视角一致性", "POV Consistency Check"),
    (20, "段落等长", "Paragraph Uniformity Check"),
    (21, "套话密度", "Cliche Density Check"),
    (22, "公式化转折", "Formulaic Twist Check"),
    (23, "列表式结构", "List-like Structure Check"),
    (24, "支线停滞", "Subplot Stagnation Check"),
    (25, "弧线平坦", "Arc Flatline Check"),
    (26, "节奏单调", "Pacing Monotony Check"),
    (27, "敏感词检查", "Sensitive Content Check"),
    (28, "读者期待管理", "Reader Expectation Check"),
    (29, "章节备忘偏离", "Chapter Memo Drift Check"),
];

fn parse_audit_result(content: &str) -> Result<AuditResult, AppError> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
        let passed = json.get("passed").and_then(|v| v.as_bool()).unwrap_or(false);
        let score = json.get("overall_score")
            .or_else(|| json.get("score"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let summary = json.get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let issues = json.get("issues")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|item| {
                    let severity = match item.get("severity")?.as_str()? {
                        "critical" => AuditSeverity::Critical,
                        "warning" => AuditSeverity::Warning,
                        _ => AuditSeverity::Info,
                    };
                    Some(AuditIssue {
                        severity,
                        category: item.get("category")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        description: item.get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        suggestion: item.get("suggestion")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    })
                }).collect()
            })
            .unwrap_or_default();

        return Ok(AuditResult { passed, score, issues, summary });
    }

    Ok(AuditResult {
        passed: false,
        score: 0.0,
        issues: Vec::new(),
        summary: content.to_string(),
    })
}

fn read_book_language(book_dir: &std::path::Path) -> Option<String> {
    crate::infrastructure::state_store::gc::utils::read_book_language_from_dir(book_dir)
}

fn read_chapter_content(book_dir: &std::path::Path, chapter_number: u32) -> Result<String, AppError> {
    let chapters_dir = book_dir.join("chapters");
    let prefix = format!("{:04}_", chapter_number);

    if let Ok(entries) = std::fs::read_dir(&chapters_dir) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(&prefix) {
                let content = std::fs::read_to_string(entry.path())?;
                return Ok(content);
            }
        }
    }

    Err(AppError::not_found(format!("Chapter {} not found", chapter_number)))
}
