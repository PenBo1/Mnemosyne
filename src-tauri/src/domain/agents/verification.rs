use serde::{Deserialize, Serialize};
use crate::errors::AppError;
use crate::infra::gc::utils;

/// Verification gate types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GateType {
    Structural,
    Semantic,
    Consistency,
    Stylistic,
    WordCount,
    ForbiddenPattern,
}

/// A verification gate that validates agent output
pub struct VerificationGate {
    pub gate_type: GateType,
    validator: Box<dyn Fn(&str, &GateContext) -> Result<GateResult, AppError> + Send + Sync>,
}

#[derive(Debug, Clone)]
pub struct GateContext {
    pub chapter_number: u32,
    pub plan: Option<String>,
    pub previous_content: Option<String>,
    pub style_guide: Option<String>,
    pub min_words: Option<u32>,
    pub max_words: Option<u32>,
    pub forbidden_patterns: Vec<String>,
    pub language: String,
}

impl Default for GateContext {
    fn default() -> Self {
        Self {
            chapter_number: 0,
            plan: None,
            previous_content: None,
            style_guide: None,
            min_words: Some(2000),
            max_words: Some(5000),
            forbidden_patterns: Vec::new(),
            language: "zh".to_string(),
        }
    }
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

/// HMAC-signed override for bypassing gates in emergencies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateOverride {
    pub gate_type: GateType,
    pub reason: String,
    pub signature: String,
    pub timestamp: String,
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
    overrides: Vec<GateOverride>,
}

impl VerificationPipeline {
    pub fn new() -> Self {
        let mut gates = Vec::new();

        // Structural gate: check output format
        gates.push(VerificationGate::new(
            GateType::Structural,
            Box::new(|content, _ctx| {
                let mut issues = Vec::new();

                if !content.contains("CHAPTER_TITLE") && !content.contains("标题") {
                    issues.push(GateIssue {
                        severity: IssueSeverity::Critical,
                        dimension: "structure".to_string(),
                        description: "Missing chapter title section".to_string(),
                        suggestion: Some("Add === CHAPTER_TITLE === section".to_string()),
                    });
                }

                if !content.contains("CHAPTER_CONTENT") && !content.contains("正文") {
                    issues.push(GateIssue {
                        severity: IssueSeverity::Critical,
                        dimension: "structure".to_string(),
                        description: "Missing chapter content section".to_string(),
                        suggestion: Some("Add === CHAPTER_CONTENT === section".to_string()),
                    });
                }

                let passed = issues.iter().all(|i| i.severity != IssueSeverity::Critical);
                Ok(GateResult { passed, issues, score: if passed { 1.0 } else { 0.0 } })
            }),
        ));

        // Semantic gate: check content matches plan
        gates.push(VerificationGate::new(
            GateType::Semantic,
            Box::new(|content, ctx| {
                let mut issues = Vec::new();
                let mut score = 1.0;

                if let Some(ref plan) = ctx.plan {
                    let plan_keywords: Vec<&str> = plan.split_whitespace()
                        .filter(|w| w.len() > 3).take(10).collect();
                    let matches = plan_keywords.iter().filter(|kw| content.contains(*kw)).count();
                    let match_ratio = if plan_keywords.is_empty() { 1.0 } else { matches as f64 / plan_keywords.len() as f64 };

                    if match_ratio < 0.3 {
                        score = match_ratio;
                        issues.push(GateIssue {
                            severity: IssueSeverity::Warning,
                            dimension: "semantic".to_string(),
                            description: format!("Low plan alignment: {:.0}%", match_ratio * 100.0),
                            suggestion: Some("Review if content follows the chapter plan".to_string()),
                        });
                    }
                }

                Ok(GateResult { passed: issues.iter().all(|i| i.severity != IssueSeverity::Critical), issues, score })
            }),
        ));

        // Consistency gate: check with previous chapters
        gates.push(VerificationGate::new(
            GateType::Consistency,
            Box::new(|content, ctx| {
                let mut issues = Vec::new();

                if let Some(ref prev) = ctx.previous_content {
                    let prev_names = extract_character_names(prev);
                    let curr_names = extract_character_names(content);

                    // Warn if a character from previous chapter disappears
                    for name in &prev_names {
                        if !curr_names.iter().any(|n| n == name) {
                            issues.push(GateIssue {
                                severity: IssueSeverity::Warning,
                                dimension: "consistency".to_string(),
                                description: format!("Character '{}' not present in current chapter", name),
                                suggestion: Some(format!("Ensure '{}' is mentioned", name)),
                            });
                        }
                    }
                }

                let score = if issues.iter().any(|i| i.severity == IssueSeverity::Critical) {
                    0.5
                } else if issues.iter().any(|i| i.severity == IssueSeverity::Warning) {
                    0.8
                } else {
                    1.0
                };

                Ok(GateResult {
                    passed: issues.iter().all(|i| i.severity != IssueSeverity::Critical),
                    issues,
                    score,
                })
            }),
        ));

        // Word count gate
        gates.push(VerificationGate::new(
            GateType::WordCount,
            Box::new(|content, ctx| {
                let mut issues = Vec::new();
                let words = utils::count_words(content, &ctx.language);

                if let Some(min) = ctx.min_words {
                    if words < min {
                        issues.push(GateIssue {
                            severity: IssueSeverity::Warning,
                            dimension: "word_count".to_string(),
                            description: format!("Too short: {} words (min: {})", words, min),
                            suggestion: Some("Expand scenes and descriptions".to_string()),
                        });
                    }
                }
                if let Some(max) = ctx.max_words {
                    if words > max {
                        issues.push(GateIssue {
                            severity: IssueSeverity::Warning,
                            dimension: "word_count".to_string(),
                            description: format!("Too long: {} words (max: {})", words, max),
                            suggestion: Some("Trim redundant passages".to_string()),
                        });
                    }
                }

                let score = if issues.is_empty() { 1.0 } else { 0.7 };
                Ok(GateResult { passed: issues.iter().all(|i| i.severity != IssueSeverity::Critical), issues, score })
            }),
        ));

        // Forbidden pattern gate
        gates.push(VerificationGate::new(
            GateType::ForbiddenPattern,
            Box::new(|content, ctx| {
                let mut issues = Vec::new();

                for pattern in &ctx.forbidden_patterns {
                    if content.contains(pattern.as_str()) {
                        issues.push(GateIssue {
                            severity: IssueSeverity::Critical,
                            dimension: "forbidden_pattern".to_string(),
                            description: format!("Contains forbidden pattern: '{}'", pattern),
                            suggestion: Some("Remove the forbidden content".to_string()),
                        });
                    }
                }

                let passed = issues.iter().all(|i| i.severity != IssueSeverity::Critical);
                Ok(GateResult { passed, issues, score: if passed { 1.0 } else { 0.0 } })
            }),
        ));

        Self { gates, overrides: Vec::new() }
    }

    pub async fn validate_all(&self, content: &str, context: &GateContext) -> Result<Vec<GateResult>, AppError> {
        let mut results = Vec::new();
        for gate in &self.gates {
            // Skip overridden gates
            if self.overrides.iter().any(|o| std::mem::discriminant(&o.gate_type) == std::mem::discriminant(&gate.gate_type)) {
                continue;
            }
            let result = gate.validate(content, context).await?;
            results.push(result);
        }
        Ok(results)
    }

    pub fn overall_passed(&self, results: &[GateResult]) -> bool {
        results.iter().all(|r| r.passed)
    }

    pub fn average_score(&self, results: &[GateResult]) -> f64 {
        if results.is_empty() { return 1.0; }
        results.iter().map(|r| r.score).sum::<f64>() / results.len() as f64
    }

    /// Add a signed override to bypass a gate.
    pub fn add_override(&mut self, override_entry: GateOverride) {
        self.overrides.push(override_entry);
    }

    /// Get word count for content (language-aware).
    pub fn word_count(content: &str, language: &str) -> u32 {
        utils::count_words(content, language)
    }
}

/// Extract character names from text (simple heuristic: 2-4 char Chinese names or capitalized English names).
fn extract_character_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    // Look for patterns like: "张三说" "李四道" "Alice said" "Bob replied"
    for word in text.split(|c: char| c.is_whitespace() || c == '，' || c == '。' || c == ',' || c == '.') {
        let w = word.trim();
        // Chinese name pattern: 2-4 chars, all CJK
        if w.chars().all(|c| c >= '\u{4e00}' && c <= '\u{9fff}') && w.len() >= 2 && w.len() <= 8 {
            // Skip common non-name words
            let skip = ["我们", "他们", "她们", "自己", "什么", "怎么", "这里", "那里", "这个", "那个", "可以", "没有", "已经", "但是", "因为", "所以", "如果", "就是", "不是", "还有", "一个", "这个", "那个", "什么", "怎么", "很", "都", "也", "就", "又", "还", "很", "了", "的", "地", "得", "着", "过"];
            if !skip.contains(&w) {
                names.push(w.to_string());
            }
        }
    }
    // Deduplicate
    names.sort();
    names.dedup();
    // Return at most 10 names
    names.into_iter().take(10).collect()
}
