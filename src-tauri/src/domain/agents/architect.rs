use async_trait::async_trait;
use crate::errors::AppError;
use crate::domain::story::BookConfig;
use crate::infra::data_dir::DataDir;
use super::base::{AgentContext, BaseAgent};
use super::types::AgentRole;
use super::prompts::architect_prompts;
use super::agent_identity::AgentIdentity;

pub struct ArchitectAgent;

impl Default for ArchitectAgent {
    fn default() -> Self { Self }
}
impl ArchitectAgent {
    pub fn new() -> Self { Self }

    /// Generate foundation for a new book
    pub async fn generate_foundation(
        &self,
        ctx: &AgentContext,
        book: &BookConfig,
        external_context: Option<&str>,
        data_dir: &DataDir,
    ) -> Result<ArchitectOutput, AppError> {
        let identity = AgentIdentity::load(data_dir, "architect");
        let task_query = format!("generate foundation for book: {}", book.title);
        let identity_prefix = identity.build_system_prompt_with_memory(
            &ctx.memory, &task_query, ctx.skill_manager.as_deref(), ctx.user_profile.as_deref(),
        ).await;
        let system = architect_prompts::build_system_prompt(&book.language, Some(&identity_prefix));
        let user = architect_prompts::build_user_prompt(
            book,
            external_context,
        );

        let response = self.chat(ctx, &system, &user).await?;
        let output = parse_architect_output(&response.content, book)?;
        Ok(output)
    }

    /// Write foundation files to disk
    pub async fn write_foundation_files(
        &self,
        book_dir: &std::path::Path,
        output: &ArchitectOutput,
        _language: &str,
    ) -> Result<(), AppError> {
        let story_dir = book_dir.join("story");
        let outline_dir = story_dir.join("outline");
        let roles_dir = story_dir.join("roles");

        std::fs::create_dir_all(&outline_dir)?;
        std::fs::create_dir_all(roles_dir.join("major"))?;
        std::fs::create_dir_all(roles_dir.join("minor"))?;

        // Write story_frame.md
        std::fs::write(
            outline_dir.join("story_frame.md"),
            &output.story_frame,
        )?;

        // Write volume_map.md
        std::fs::write(
            outline_dir.join("volume_map.md"),
            &output.volume_map,
        )?;

        // Write book_rules.md
        std::fs::write(
            story_dir.join("book_rules.md"),
            &output.book_rules,
        )?;

        // Write character files
        for role in &output.roles {
            let subdir = match role.tier.as_str() {
                "major" => "major",
                _ => "minor",
            };
            let path = roles_dir.join(subdir).join(format!("{}.md", role.name));
            std::fs::write(path, &role.content)?;
        }

        // Write pending_hooks.md
        std::fs::write(
            story_dir.join("pending_hooks.md"),
            &output.pending_hooks,
        )?;

        // Write story_bible.md (compat shim)
        std::fs::write(
            story_dir.join("story_bible.md"),
            &output.story_bible,
        )?;

        // Write volume_outline.md (compat shim)
        std::fs::write(
            story_dir.join("volume_outline.md"),
            &output.volume_outline,
        )?;

        // Write character_matrix.md (compat shim)
        std::fs::write(
            story_dir.join("character_matrix.md"),
            &output.character_matrix,
        )?;

        Ok(())
    }
}

#[async_trait]
impl BaseAgent for ArchitectAgent {
    fn role(&self) -> AgentRole {
        AgentRole::Architect
    }

    fn name(&self) -> &str {
        "architect"
    }
}

pub struct ArchitectRole {
    pub tier: String,
    pub name: String,
    pub content: String,
}

pub struct ArchitectOutput {
    pub story_frame: String,
    pub volume_map: String,
    pub book_rules: String,
    pub story_bible: String,
    pub volume_outline: String,
    pub character_matrix: String,
    pub pending_hooks: String,
    pub roles: Vec<ArchitectRole>,
}

fn parse_architect_output(content: &str, _book: &BookConfig) -> Result<ArchitectOutput, AppError> {
    let sections = extract_sections(content);

    let story_frame = sections.get("story_frame").cloned()
        .or_else(|| sections.get("STORY_FRAME").cloned())
        .unwrap_or_default();

    let volume_map = sections.get("volume_map").cloned()
        .or_else(|| sections.get("VOLUME_MAP").cloned())
        .unwrap_or_default();

    let book_rules = sections.get("book_rules").cloned()
        .or_else(|| sections.get("BOOK_RULES").cloned())
        .unwrap_or_default();

    let pending_hooks = sections.get("pending_hooks").cloned()
        .or_else(|| sections.get("PENDING_HOOKS").cloned())
        .unwrap_or_default();

    // Parse roles from the roles section
    let roles_text = sections.get("roles").cloned()
        .or_else(|| sections.get("ROLES").cloned())
        .unwrap_or_default();

    let roles = parse_roles_from_text(&roles_text);

    // Build compat shims
    let story_bible = build_story_bible_shim(&story_frame, &book_rules);
    let volume_outline = build_volume_outline_shim(&volume_map);
    let character_matrix = build_character_matrix_shim(&roles);

    Ok(ArchitectOutput {
        story_frame,
        volume_map,
        book_rules,
        story_bible,
        volume_outline,
        character_matrix,
        pending_hooks,
        roles,
    })
}

fn extract_sections(content: &str) -> std::collections::HashMap<String, String> {
    let mut sections = std::collections::HashMap::new();
    let mut current_section: Option<String> = None;
    let mut current_content = String::new();

    for line in content.lines() {
        if line.starts_with("=== ") && line.ends_with(" ===") {
            if let Some(name) = current_section.take() {
                sections.insert(name, current_content.trim().to_string());
                current_content.clear();
            }
            let name = line[4..line.len()-4].trim().to_string();
            current_section = Some(name);
        } else if let Some(ref _name) = current_section {
            if !current_content.is_empty() {
                current_content.push('\n');
            }
            current_content.push_str(line);
        }
    }

    if let Some(name) = current_section {
        sections.insert(name, current_content.trim().to_string());
    }

    sections
}

fn parse_roles_from_text(text: &str) -> Vec<ArchitectRole> {
    let mut roles = Vec::new();
    let mut current_role: Option<String> = None;
    let mut current_tier = "minor".to_string();
    let mut current_content = String::new();

    for line in text.lines() {
        if line.starts_with("## ") || line.starts_with("### ") {
            if let Some(name) = current_role.take() {
                roles.push(ArchitectRole {
                    tier: current_tier.clone(),
                    name,
                    content: current_content.trim().to_string(),
                });
                current_content.clear();
            }
            let heading = line.trim_start_matches('#').trim();
            if heading.contains("主角") || heading.contains("protagonist") {
                current_tier = "major".to_string();
            }
            current_role = Some(heading.to_string());
        } else if let Some(ref _name) = current_role {
            if !current_content.is_empty() {
                current_content.push('\n');
            }
            current_content.push_str(line);
        }
    }

    if let Some(name) = current_role {
        roles.push(ArchitectRole {
            tier: current_tier,
            name,
            content: current_content.trim().to_string(),
        });
    }

    roles
}

fn build_story_bible_shim(story_frame: &str, book_rules: &str) -> String {
    let mut parts = Vec::new();
    parts.push("# Story Bible\n".to_string());
    if !story_frame.is_empty() {
        parts.push(format!("## Story Frame\n\n{}\n", story_frame));
    }
    if !book_rules.is_empty() {
        parts.push(format!("## Book Rules\n\n{}\n", book_rules));
    }
    parts.join("\n")
}

fn build_volume_outline_shim(volume_map: &str) -> String {
    if volume_map.is_empty() {
        return "# Volume Outline\n\n(Not yet generated)".to_string();
    }
    format!("# Volume Outline\n\n{}", volume_map)
}

fn build_character_matrix_shim(roles: &[ArchitectRole]) -> String {
    let mut lines = vec!["# Character Matrix\n".to_string(), "| Name | Role | Status |".to_string(), "| --- | --- | --- |".to_string()];
    for role in roles {
        lines.push(format!("| {} | {} | Active |", role.name, role.tier));
    }
    lines.join("\n")
}
