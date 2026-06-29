//! Book evaluation utilities.

use crate::features::story::AuditResult;

/// Evaluate overall book quality
pub fn evaluate_book_quality(
    audit_results: &[AuditResult],
    chapter_count: u32,
) -> BookEvaluation {
    let total_score: f64 = audit_results.iter().map(|a| a.score).sum();
    let avg_score = if audit_results.is_empty() { 0.0 } else { total_score / audit_results.len() as f64 };

    let total_critical: usize = audit_results.iter()
        .map(|a| a.issues.iter().filter(|i| i.severity == crate::features::story::AuditSeverity::Critical).count())
        .sum();
    let total_warning: usize = audit_results.iter()
        .map(|a| a.issues.iter().filter(|i| i.severity == crate::features::story::AuditSeverity::Warning).count())
        .sum();

    let quality_grade = if avg_score >= 90.0 { "A" }
        else if avg_score >= 80.0 { "B" }
        else if avg_score >= 70.0 { "C" }
        else if avg_score >= 60.0 { "D" }
        else { "F" };

    BookEvaluation {
        avg_score,
        total_chapters: chapter_count,
        total_critical_issues: total_critical as u32,
        total_warning_issues: total_warning as u32,
        quality_grade: quality_grade.to_string(),
    }
}

pub struct BookEvaluation {
    pub avg_score: f64,
    pub total_chapters: u32,
    pub total_critical_issues: u32,
    pub total_warning_issues: u32,
    pub quality_grade: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::story::AuditResult;

    #[test]
    fn test_evaluate_book_quality() {
        let audits = vec![
            AuditResult { passed: true, score: 85.0, issues: vec![], summary: String::new() },
            AuditResult { passed: true, score: 90.0, issues: vec![], summary: String::new() },
        ];
        let eval = evaluate_book_quality(&audits, 2);
        assert_eq!(eval.avg_score, 87.5);
        assert_eq!(eval.quality_grade, "B");
    }
}
