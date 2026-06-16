use serde::{Deserialize, Serialize};
use crate::errors::AppError;

/// Verification gate types (P14.38)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GateType {
    /// Check output format and structure
    Structural,
    /// Check content matches the plan
    Semantic,
    /// Check consistency with previous chapters
    Consistency,
    /// Check style matches the book's style
    Stylistic,
}

/// A verification gate that validates agent output
pub struct VerificationGate {
    gate_type: GateType,
    validator: Box<dyn Fn(&str, &GateContext) -> Result<GateResult, AppError> + Send + Sync>,
}

#[derive(Debug, Clone)]
pub struct GateContext {
    pub chapter_number: u32,
    pub plan: Option<String>,
    pub previous_content: Option<String>,
    pub style_guide: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub passed: bool,
    pub issues: Vec<GateIssue>,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateIssue {
    pub severity: IssueSeverity,
    pub dimension: String,
    pub description: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IssueSeverity {
    Critical,
    Warning,
    Info,
}

impl VerificationGate {
    pub fn new(gate_type: GateType, validator: Box<dyn Fn(&str, &GateContext) -> Result<GateResult, AppError> + Send + Sync>) -> Self {
        Self { gate_type, validator }
    }

    pub async fn validate(&self, content: &str, context: &GateContext) -> Result<GateResult, AppError> {
        (self.validator)(content, context)
    }
}

/// Pipeline of verification gates
pub struct VerificationPipeline {
    gates: Vec<VerificationGate>,
}

impl VerificationPipeline {
    pub fn new() -> Self {
        let mut gates = Vec::new();

        // Structural gate: check output format
        gates.push(VerificationGate::new(
            GateType::Structural,
            Box::new(|content, _ctx| {
                let mut issues = Vec::new();
                let mut passed = true;

                // Check for required sections
                if !content.contains("CHAPTER_TITLE") && !content.contains("标题") {
                    passed = false;
                    issues.push(GateIssue {
                        severity: IssueSeverity::Critical,
                        dimension: "structure".to_string(),
                        description: "Missing chapter title section".to_string(),
                        suggestion: Some("Add === CHAPTER_TITLE === section".to_string()),
                    });
                }

                if !content.contains("CHAPTER_CONTENT") && !content.contains("正文") {
                    passed = false;
                    issues.push(GateIssue {
                        severity: IssueSeverity::Critical,
                        dimension: "structure".to_string(),
                        description: "Missing chapter content section".to_string(),
                        suggestion: Some("Add === CHAPTER_CONTENT === section".to_string()),
                    });
                }

                Ok(GateResult {
                    passed,
                    issues,
                    score: if passed { 1.0 } else { 0.0 },
                })
            }),
        ));

        // Semantic gate: check content matches plan
        gates.push(VerificationGate::new(
            GateType::Semantic,
            Box::new(|content, ctx| {
                let mut issues = Vec::new();
                let mut score = 1.0;

                if let Some(ref plan) = ctx.plan {
                    // Simple keyword matching as a basic semantic check
                    let plan_keywords: Vec<&str> = plan
                        .split_whitespace()
                        .filter(|w| w.len() > 3)
                        .take(10)
                        .collect();

                    let matches: usize = plan_keywords
                        .iter()
                        .filter(|kw| content.contains(*kw))
                        .count();

                    let match_ratio = matches as f64 / plan_keywords.len() as f64;
                    if match_ratio < 0.3 {
                        score = match_ratio;
                        issues.push(GateIssue {
                            severity: IssueSeverity::Warning,
                            dimension: "semantic".to_string(),
                            description: format!(
                                "Low plan alignment: {:.0}% of plan keywords found in content",
                                match_ratio * 100.0
                            ),
                            suggestion: Some("Review if content follows the chapter plan".to_string()),
                        });
                    }
                }

                Ok(GateResult {
                    passed: issues.iter().all(|i| i.severity != IssueSeverity::Critical),
                    issues,
                    score,
                })
            }),
        ));

        // Consistency gate: check with previous chapters
        gates.push(VerificationGate::new(
            GateType::Consistency,
            Box::new(|_content, ctx| {
                let issues = Vec::new();

                // Basic consistency check - could be enhanced with LLM
                if let Some(ref prev) = ctx.previous_content {
                    // Check for contradictory statements (simplified)
                    let _ = prev; // Placeholder for more sophisticated checks
                }

                Ok(GateResult {
                    passed: true, // Default to pass, LLM auditor does detailed check
                    issues,
                    score: 1.0,
                })
            }),
        ));

        Self { gates }
    }

    pub async fn validate_all(
        &self,
        content: &str,
        context: &GateContext,
    ) -> Result<Vec<GateResult>, AppError> {
        let mut results = Vec::new();
        for gate in &self.gates {
            let result = gate.validate(content, context).await?;
            results.push(result);
        }
        Ok(results)
    }

    pub fn overall_passed(&self, results: &[GateResult]) -> bool {
        results.iter().all(|r| r.passed)
    }
}
