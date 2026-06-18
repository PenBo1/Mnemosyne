use crate::errors::AppError;
use crate::domain::story::AuditResult;
use crate::domain::agents::*;
use crate::domain::agents::base::AgentContext;
use crate::domain::agents::reviser::ReviseMode;
use crate::infra::data_dir::DataDir;
use crate::infra::gc::utils;

pub struct ReviewCycleResult {
    pub final_content: String,
    pub final_word_count: u32,
    pub revised: bool,
    pub audit_result: AuditResult,
}

/// Run the audit → revise loop for a chapter
pub async fn run_chapter_review_cycle(
    auditor_ctx: &AgentContext,
    reviser_ctx: &AgentContext,
    book_dir: &std::path::Path,
    chapter_number: u32,
    initial_content: &str,
    initial_title: &str,
    max_iterations: u32,
    data_dir: &DataDir,
) -> Result<ReviewCycleResult, AppError> {
    // Initial audit
    let auditor = ContinuityAuditor::new();
    let mut audit = auditor.audit_chapter(auditor_ctx, book_dir, chapter_number, data_dir).await?;

    let mut current_content = initial_content.to_string();
    let mut revised = false;

    // Revise loop
    if !audit.passed {
        for _round in 0..max_iterations {
            if audit.issues.iter().any(|i| i.severity == crate::domain::story::AuditSeverity::Critical) {
                let reviser = ReviserAgent::new();
                let output = reviser.revise_chapter(
                    reviser_ctx, book_dir, chapter_number,
                    &current_content, &audit, ReviseMode::Auto,
                    data_dir,
                ).await?;

                current_content = output.content;
                revised = true;

                // Save and re-audit
                super::chapter_persistence::save_chapter_file(
                    book_dir, chapter_number, initial_title, &current_content,
                )?;

                audit = auditor.audit_chapter(auditor_ctx, book_dir, chapter_number, data_dir).await?;
            } else {
                break;
            }
        }
    }

    let word_count = utils::count_words(&current_content, "zh");

    Ok(ReviewCycleResult {
        final_content: current_content,
        final_word_count: word_count,
        revised,
        audit_result: audit,
    })
}
