/// Runtime agent identity loader.
///
/// Loads SOUL.md, CONTEXT.md, MEMORY.md from the app data directory
/// and assembles them into the system prompt prefix layer.
///
/// Design: Hermes Agent's three-tier prompt assembly:
/// - SOUL.md → identity (slot #1 in system prompt)
/// - CONTEXT.md → pipeline rules (how this agent works)
/// - MEMORY.md → accumulated learning (what this agent knows)
use std::path::Path;
use crate::infra::data_dir::DataDir;

/// Assembled identity for a pipeline agent, loaded from data directory.
pub struct AgentIdentity {
    /// The agent's soul/personality (SOUL.md content)
    pub soul: String,
    /// Pipeline rules and context (CONTEXT.md content)
    pub context: String,
    /// Accumulated memory (MEMORY.md content)
    pub memory: String,
}

impl AgentIdentity {
    /// Load agent identity from the data directory.
    /// Falls back to empty strings if files don't exist.
    pub fn load(data_dir: &DataDir, role: &str) -> Self {
        let soul = read_file_or_empty(&data_dir.agent_soul_path(role));
        let context = read_file_or_empty(&data_dir.agent_context_path(role));
        let memory = read_file_or_empty(&data_dir.agent_memory_path(role));

        Self { soul, context, memory }
    }

    /// Build the identity prefix for the system prompt.
    /// This is injected before the task-specific prompt.
    pub fn build_system_prefix(&self) -> String {
        let mut parts = Vec::new();

        if !self.soul.is_empty() {
            parts.push(self.soul.trim().to_string());
        }
        if !self.context.is_empty() {
            parts.push(self.context.trim().to_string());
        }
        if !self.memory.is_empty() && self.memory.trim() != "# Agent Memory" {
            parts.push(format!(
                "## Agent Memory\n\n{}",
                self.memory.trim().lines()
                    .filter(|l| !l.starts_with('#') && !l.starts_with("<!--"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        parts.join("\n\n")
    }

    /// Build system prompt with memory and skill injection.
    pub async fn build_system_prompt_with_memory(
        &self,
        memory: &tokio::sync::RwLock<crate::domain::agents::base::MemorySystem>,
        context_query: &str,
        skill_manager: Option<&crate::infra::skill::discovery::SkillManager>,
    ) -> String {
        let mut prompt = self.build_system_prefix();

        let mem = memory.read().await;
        let relevant = mem.search_memory(context_query, 10);
        if !relevant.is_empty() {
            let memory_section: Vec<String> = relevant.iter().map(|e| {
                format!("- [{}] {}", e.entry_type as u8, e.content)
            }).collect();
            prompt.push_str(&format!(
                "\n\n## Relevant Memories\n{}\n",
                memory_section.join("\n")
            ));
        }

        let main_ctx = mem.format_main_context();
        if !main_ctx.is_empty() {
            prompt.push_str(&format!(
                "\n\n## Active Context\n{}\n",
                main_ctx
            ));
        }

        if let Some(sm) = skill_manager {
            let skills = sm.find_relevant(context_query, 3);
            if !skills.is_empty() {
                let skill_section: Vec<String> = skills.iter().map(|(s, score)| {
                    format!("- **{}** (relevance: {:.0}%): {}", s.meta.name, score * 100.0, s.meta.description)
                }).collect();
                prompt.push_str(&format!(
                    "\n\n## Available Skills\n{}\n",
                    skill_section.join("\n")
                ));
            }
        }

        prompt
    }

    /// Save updated MEMORY.md back to disk.
    pub fn save_memory(&self, data_dir: &DataDir, role: &str, content: &str) -> Result<(), std::io::Error> {
        std::fs::write(data_dir.agent_memory_path(role), content)
    }
}

fn read_file_or_empty(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_data_dir() -> (TempDir, DataDir) {
        let tmp = TempDir::new().unwrap();
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        data_dir.initialize().unwrap();
        (tmp, data_dir)
    }

    #[test]
    fn load_creates_default_files_on_first_run() {
        let (_tmp, data_dir) = make_data_dir();
        let identity = AgentIdentity::load(&data_dir, "writer");

        assert!(!identity.soul.is_empty());
        assert!(identity.soul.contains("Writer Agent"));
        assert!(!identity.context.is_empty());
        assert!(identity.context.contains("Pipeline Context"));
        assert!(!identity.memory.is_empty());
    }

    #[test]
    fn build_system_prefix_combines_layers() {
        let (_tmp, data_dir) = make_data_dir();
        let identity = AgentIdentity::load(&data_dir, "writer");
        let prefix = identity.build_system_prefix();

        assert!(prefix.contains("Writer Agent"));
        assert!(prefix.contains("Pipeline Context"));
    }

    #[test]
    fn user_edited_soul_is_preserved() {
        let (_tmp, data_dir) = make_data_dir();
        let custom_soul = "# My Custom Writer\n\nYou write in a noir style.\n";
        fs::write(data_dir.agent_soul_path("writer"), custom_soul).unwrap();

        let identity = AgentIdentity::load(&data_dir, "writer");
        assert!(identity.soul.contains("noir style"));
    }
}
