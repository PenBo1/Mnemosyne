use super::agent_configs::AgentConfig;
use super::types::MergedHarnessContext;

pub struct ContextBuilder;

impl ContextBuilder {
    pub fn build_system_prompt(
        agent_config: &AgentConfig,
        ctx: &MergedHarnessContext,
        base_prompt: &str,
    ) -> String {
        let prompt_template = if agent_config.prompt_template.is_empty() {
            base_prompt.to_string()
        } else {
            agent_config.prompt_template.clone()
        };

        let mut parts = vec![
            format!("You are {}.", agent_config.name),
            agent_config.description.clone(),
            prompt_template,
        ];

        if !agent_config.constraints.must_do.is_empty() {
            parts.push("## Must Do".into());
            for c in &agent_config.constraints.must_do {
                parts.push(format!("- {}", c));
            }
        }

        if !agent_config.constraints.must_not_do.is_empty() {
            parts.push("## Must Not Do".into());
            for c in &agent_config.constraints.must_not_do {
                parts.push(format!("- {}", c));
            }
        }

        if !agent_config.constraints.style_rules.is_empty() {
            parts.push("## Style Rules".into());
            for r in &agent_config.constraints.style_rules {
                parts.push(format!("- {}", r));
            }
        }

        if !agent_config.output.validation.forbidden_patterns.is_empty() {
            parts.push("## Forbidden Phrases".into());
            for p in &agent_config.output.validation.forbidden_patterns {
                parts.push(format!("- \"{}\"", p));
            }
        }

        if !agent_config.quality_standards.acceptance_criteria.is_empty() {
            parts.push("## Acceptance Criteria".into());
            for c in &agent_config.quality_standards.acceptance_criteria {
                parts.push(format!("- {}", c));
            }
        }

        if let Some(novel) = &ctx.novel {
            let mut novel_parts = Vec::new();

            if !novel.writing_constraints.structure_rules.is_empty() {
                novel_parts.push("Structure rules:".into());
                for r in &novel.writing_constraints.structure_rules {
                    novel_parts.push(format!("- {}", r));
                }
            }

            if !novel.writing_constraints.prohibited_patterns.is_empty() {
                novel_parts.push("Prohibited patterns:".into());
                for p in &novel.writing_constraints.prohibited_patterns {
                    novel_parts.push(format!("- {}", p));
                }
            }

            if !novel.writing_constraints.required_elements.is_empty() {
                novel_parts.push("Required elements:".into());
                for e in &novel.writing_constraints.required_elements {
                    novel_parts.push(format!("- {}", e));
                }
            }

            if !novel.style_profile.forbidden_phrases.is_empty() {
                novel_parts.push("Forbidden phrases:".into());
                for p in &novel.style_profile.forbidden_phrases {
                    novel_parts.push(format!("- {}", p));
                }
            }

            if !novel_parts.is_empty() {
                parts.push("## Novel Constraints".into());
                parts.extend(novel_parts);
            }
        }

        if !ctx.active_constraints.is_empty() {
            parts.push("## Active Constraints (from feedback loop)".into());
            for c in ctx.active_constraints.iter().take(10) {
                let source_label = match c.source {
                    super::types::ConstraintSource::Project => "project",
                    super::types::ConstraintSource::Novel => "novel",
                    super::types::ConstraintSource::Lesson => "lesson",
                };
                parts.push(format!(
                    "- [{}] (priority {}) {}",
                    source_label, c.priority, c.content
                ));
            }
        }

        parts.join("\n\n")
    }
}
