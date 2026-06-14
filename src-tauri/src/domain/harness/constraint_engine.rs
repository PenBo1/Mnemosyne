use super::types::*;

pub struct ConstraintEngine;

impl ConstraintEngine {
    pub fn merge(
        project: &ProjectHarness,
        novel: Option<&NovelHarness>,
        lessons: &[ConstraintLesson],
    ) -> MergedHarnessContext {
        let mut active_constraints = Vec::new();

        for pattern in &project.agent_constraints.forbidden_patterns {
            active_constraints.push(ActiveConstraint {
                source: ConstraintSource::Project,
                target_agent: None,
                priority: 100,
                content: pattern.clone(),
            });
        }

        if let Some(novel) = novel {
            for rule in &novel.writing_constraints.structure_rules {
                active_constraints.push(ActiveConstraint {
                    source: ConstraintSource::Novel,
                    target_agent: None,
                    priority: 80,
                    content: rule.clone(),
                });
            }
        }

        for lesson in lessons {
            if lesson.active {
                active_constraints.push(ActiveConstraint {
                    source: ConstraintSource::Lesson,
                    target_agent: None,
                    priority: 60,
                    content: lesson.constraint_added.clone(),
                });
            }
        }

        active_constraints.sort_by(|a, b| b.priority.cmp(&a.priority));

        let quality_gates = project
            .quality_gates
            .iter()
            .cloned()
            .chain(novel.into_iter().flat_map(|n| n.quality_gates.iter().cloned()))
            .collect();

        let audit_config = novel
            .map(|n| n.audit_dimensions.clone())
            .unwrap_or_else(|| AuditDimensionConfig {
                enabled_dimensions: Vec::new(),
                dimension_weights: std::collections::HashMap::new(),
                severity_overrides: std::collections::HashMap::new(),
                pass_threshold: 0.7,
                critical_dimensions: Vec::new(),
                custom_rules: Vec::new(),
            });

        MergedHarnessContext {
            project: project.clone(),
            novel: novel.cloned(),
            active_constraints,
            audit_config,
            quality_gates,
        }
    }
}
