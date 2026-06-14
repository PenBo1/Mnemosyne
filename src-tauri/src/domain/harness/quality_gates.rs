use super::types::*;
use crate::domain::story::{AuditResult, AuditSeverity};

pub struct QualityGateEvaluator;

impl QualityGateEvaluator {
    pub fn evaluate_stage(
        content: &str,
        word_count: Option<u32>,
        audit_result: Option<&AuditResult>,
        gates: &[QualityGate],
    ) -> GateEvaluation {
        tracing::debug!(gate_count = gates.len(), "Evaluating quality gates");
        let start = std::time::Instant::now();

        let mut results = Vec::new();
        let mut has_block = false;
        let mut has_revise = false;

        for gate in gates {
            let result = match &gate.gate_type {
                GateType::ScoreThreshold => Self::eval_score_threshold(gate, audit_result),
                GateType::IssueCount => Self::eval_issue_count(gate, audit_result),
                GateType::WordCountRange => Self::eval_word_count(gate, word_count),
                GateType::ForbiddenPattern => Self::eval_forbidden_pattern(gate, content),
                GateType::CompletenessCheck => Self::eval_completeness(gate, content),
                GateType::DimensionScore => Self::eval_dimension_score(gate, audit_result),
                GateType::ConsistencyCheck | GateType::CustomRule => SingleGateResult {
                    gate_id: gate.id.clone(),
                    gate_name: gate.name.clone(),
                    passed: true,
                    actual_value: 0.0,
                    threshold: gate.threshold,
                    message: "Gate type deferred".to_string(),
                },
            };

            if !result.passed {
                match gate.action_on_fail {
                    GateAction::Block => has_block = true,
                    GateAction::Revise => has_revise = true,
                    _ => {}
                }
            }
            results.push(result);
        }

        let all_passed = results.iter().all(|r| r.passed);
        let action = if all_passed {
            GateAction::Warn
        } else if has_block {
            GateAction::Block
        } else if has_revise {
            GateAction::Revise
        } else {
            GateAction::Warn
        };

        let elapsed = start.elapsed().as_millis();
        let passed_count = results.iter().filter(|r| r.passed).count();
        let failed_count = results.len() - passed_count;

        tracing::info!(
            total = results.len(),
            passed = passed_count,
            failed = failed_count,
            action = ?action,
            elapsed_ms = elapsed,
            "Quality gate evaluation completed"
        );

        GateEvaluation {
            passed: all_passed,
            gate_results: results,
            action,
        }
    }

    fn eval_score_threshold(
        gate: &QualityGate,
        audit_result: Option<&AuditResult>,
    ) -> SingleGateResult {
        let actual = audit_result.map(|a| a.score).unwrap_or(0.0);
        let passed = actual >= gate.threshold;
        SingleGateResult {
            gate_id: gate.id.clone(),
            gate_name: gate.name.clone(),
            passed,
            actual_value: actual,
            threshold: gate.threshold,
            message: if passed {
                format!("Score {} passed (threshold: {})", actual, gate.threshold)
            } else {
                format!("Score {} failed (threshold: {})", actual, gate.threshold)
            },
        }
    }

    fn eval_issue_count(
        gate: &QualityGate,
        audit_result: Option<&AuditResult>,
    ) -> SingleGateResult {
        let critical_count = audit_result
            .map(|a| {
                a.issues
                    .iter()
                    .filter(|i| i.severity == AuditSeverity::Critical)
                    .count() as f64
            })
            .unwrap_or(0.0);
        let passed = critical_count <= gate.threshold;
        SingleGateResult {
            gate_id: gate.id.clone(),
            gate_name: gate.name.clone(),
            passed,
            actual_value: critical_count,
            threshold: gate.threshold,
            message: if passed {
                format!(
                    "Critical issues: {} (max: {})",
                    critical_count as u32,
                    gate.threshold as u32
                )
            } else {
                format!(
                    "Critical issues: {} exceeds max: {}",
                    critical_count as u32,
                    gate.threshold as u32
                )
            },
        }
    }

    fn eval_word_count(gate: &QualityGate, word_count: Option<u32>) -> SingleGateResult {
        let actual = word_count.unwrap_or(0) as f64;
        let passed = if gate.threshold > 0.0 {
            (actual - gate.threshold).abs() < gate.threshold * 0.2
        } else {
            true
        };
        SingleGateResult {
            gate_id: gate.id.clone(),
            gate_name: gate.name.clone(),
            passed,
            actual_value: actual,
            threshold: gate.threshold,
            message: if passed {
                format!("Word count {} within range of target {}", actual, gate.threshold)
            } else {
                format!("Word count {} deviates from target {}", actual, gate.threshold)
            },
        }
    }

    fn eval_forbidden_pattern(gate: &QualityGate, content: &str) -> SingleGateResult {
        let patterns: Vec<&str> = gate
            .dimension
            .as_deref()
            .map(|d| d.split('|').collect())
            .unwrap_or_default();
        let mut found = Vec::new();
        for pattern in &patterns {
            if content.contains(pattern) {
                found.push(pattern.to_string());
            }
        }
        SingleGateResult {
            gate_id: gate.id.clone(),
            gate_name: gate.name.clone(),
            passed: found.is_empty(),
            actual_value: found.len() as f64,
            threshold: 0.0,
            message: if found.is_empty() {
                "No forbidden patterns".to_string()
            } else {
                format!("Found forbidden: {}", found.join(", "))
            },
        }
    }

    fn eval_completeness(gate: &QualityGate, content: &str) -> SingleGateResult {
        let required: Vec<&str> = gate
            .dimension
            .as_deref()
            .map(|d| d.split('|').collect())
            .unwrap_or_default();
        let mut missing = Vec::new();
        for field in &required {
            if !content.contains(field) {
                missing.push(field.to_string());
            }
        }
        SingleGateResult {
            gate_id: gate.id.clone(),
            gate_name: gate.name.clone(),
            passed: missing.is_empty(),
            actual_value: missing.len() as f64,
            threshold: 0.0,
            message: if missing.is_empty() {
                "All required fields present".to_string()
            } else {
                format!("Missing fields: {}", missing.join(", "))
            },
        }
    }

    fn eval_dimension_score(
        gate: &QualityGate,
        audit_result: Option<&AuditResult>,
    ) -> SingleGateResult {
        let dimension = gate.dimension.as_deref().unwrap_or("unknown");
        let actual = audit_result
            .map(|a| {
                a.issues
                    .iter()
                    .filter(|i| i.category == dimension && i.severity == AuditSeverity::Critical)
                    .count() as f64
            })
            .unwrap_or(0.0);
        let passed = actual <= gate.threshold;
        SingleGateResult {
            gate_id: gate.id.clone(),
            gate_name: gate.name.clone(),
            passed,
            actual_value: actual,
            threshold: gate.threshold,
            message: if passed {
                format!("Dimension '{}' OK", dimension)
            } else {
                format!(
                    "Dimension '{}' issues: {} (max: {})",
                    dimension,
                    actual,
                    gate.threshold
                )
            },
        }
    }
}
