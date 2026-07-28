//! ═══════════════════════════════════════════════════════════════════════════
//! Context - 提示词上下文
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::super::user_profile::UserProfileSnapshot;

const PROMPT_CONTEXT_VERSION: u32 = 1;

// ── Audience 枚举 ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Audience {
    Primary,
    Subagent,
}

impl Default for Audience {
    fn default() -> Self {
        Self::Primary
    }
}

// ── PromptContext ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptContext {
    pub version: u32,
    pub role: String,
    pub audience: Audience,
    pub soul_content: Option<String>,
    pub context_content: Option<String>,
    pub memory_content: Option<String>,
    pub user_profile: Option<UserProfileSnapshot>,
    pub custom_instructions: Option<String>,
    pub os_name: Option<String>,
    pub shell_path: Option<String>,
    pub working_directory: Option<String>,
    pub current_date: Option<String>,
}

impl Default for PromptContext {
    fn default() -> Self {
        Self {
            version: PROMPT_CONTEXT_VERSION,
            role: "main".to_string(),
            audience: Audience::default(),
            soul_content: None,
            context_content: None,
            memory_content: None,
            user_profile: None,
            custom_instructions: None,
            os_name: None,
            shell_path: None,
            working_directory: None,
            current_date: None,
        }
    }
}

impl PromptContext {
    pub fn new(role: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            ..Default::default()
        }
    }

    pub fn with_audience(mut self, audience: Audience) -> Self {
        self.audience = audience;
        self
    }

    pub fn with_soul(mut self, content: impl Into<String>) -> Self {
        self.soul_content = Some(content.into());
        self
    }

    pub fn with_context(mut self, content: impl Into<String>) -> Self {
        self.context_content = Some(content.into());
        self
    }

    pub fn with_memory(mut self, content: impl Into<String>) -> Self {
        self.memory_content = Some(content.into());
        self
    }

    pub fn with_user_profile(mut self, profile: UserProfileSnapshot) -> Self {
        self.user_profile = Some(profile);
        self
    }

    pub fn with_custom_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.custom_instructions = Some(instructions.into());
        self
    }

    pub fn with_os_name(mut self, os: impl Into<String>) -> Self {
        self.os_name = Some(os.into());
        self
    }

    pub fn with_shell_path(mut self, path: impl Into<String>) -> Self {
        self.shell_path = Some(path.into());
        self
    }

    pub fn with_working_directory(mut self, dir: impl Into<String>) -> Self {
        self.working_directory = Some(dir.into());
        self
    }

    pub fn with_current_date(mut self, date: impl Into<String>) -> Self {
        self.current_date = Some(date.into());
        self
    }

    pub fn placeholders(&self) -> HashMap<&'static str, String> {
        let mut map = HashMap::new();

        map.insert("role", self.role.clone());
        map.insert("audience", self.audience_label());

        if let Some(ref soul) = self.soul_content {
            map.insert("soul_content", soul.clone());
        }
        if let Some(ref ctx) = self.context_content {
            map.insert("context_content", ctx.clone());
        }
        if let Some(ref mem) = self.memory_content {
            map.insert("memory_content", mem.clone());
        }
        if let Some(ref inst) = self.custom_instructions {
            map.insert("custom_instructions", inst.clone());
        }
        if let Some(ref os) = self.os_name {
            map.insert("os_name", os.clone());
        }
        if let Some(ref shell) = self.shell_path {
            map.insert("shell_path", shell.clone());
        }
        if let Some(ref wd) = self.working_directory {
            map.insert("working_directory", wd.clone());
        }
        if let Some(ref date) = self.current_date {
            map.insert("current_date", date.clone());
        }

        if let Some(ref profile) = self.user_profile {
            map.insert("user_profile", profile.format_for_prompt());
        }

        map
    }

    pub fn render(&self) -> String {
        let mut parts: Vec<String> = Vec::with_capacity(6);

        if let Some(ref soul) = self.soul_content {
            if !soul.trim().is_empty() {
                parts.push(soul.clone());
            }
        }

        if let Some(ref ctx) = self.context_content {
            if !ctx.trim().is_empty() {
                parts.push(ctx.clone());
            }
        }

        if let Some(ref profile) = self.user_profile {
            let formatted = profile.format_for_prompt();
            if !formatted.trim().is_empty() {
                parts.push(formatted);
            }
        }

        if let Some(ref inst) = self.custom_instructions {
            if !inst.trim().is_empty() {
                parts.push(format!("# Additional Instructions\n\n{}", inst));
            }
        }

        if let Some(ref mem) = self.memory_content {
            if !mem.trim().is_empty() {
                parts.push(mem.clone());
            }
        }

        parts.join("\n\n---\n\n")
    }

    fn audience_label(&self) -> String {
        match self.audience {
            Audience::Primary => "primary".to_string(),
            Audience::Subagent => "subagent".to_string(),
        }
    }
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_context_default() {
        let ctx = PromptContext::default();
        assert_eq!(ctx.version, PROMPT_CONTEXT_VERSION);
        assert_eq!(ctx.role, "main");
        assert_eq!(ctx.audience, Audience::Primary);
        assert!(ctx.soul_content.is_none());
    }

    #[test]
    fn prompt_context_builder() {
        let ctx = PromptContext::new("architect")
            .with_audience(Audience::Subagent)
            .with_soul("Architect soul")
            .with_context("Architect context");

        assert_eq!(ctx.role, "architect");
        assert_eq!(ctx.audience, Audience::Subagent);
        assert_eq!(ctx.soul_content, Some("Architect soul".to_string()));
    }

    #[test]
    fn placeholders_includes_all_fields() {
        let ctx = PromptContext::new("writer")
            .with_os_name("Windows")
            .with_shell_path("/bin/bash");

        let ph = ctx.placeholders();
        assert_eq!(ph.get("role"), Some(&"writer".to_string()));
        assert_eq!(ph.get("os_name"), Some(&"Windows".to_string()));
        assert_eq!(ph.get("shell_path"), Some(&"/bin/bash".to_string()));
    }

    #[test]
    fn render_joins_sections() {
        let ctx = PromptContext::new("main")
            .with_soul("Soul content")
            .with_context("Context content")
            .with_memory("Memory content");

        let rendered = ctx.render();
        assert!(rendered.contains("Soul content"));
        assert!(rendered.contains("Context content"));
        assert!(rendered.contains("Memory content"));
        assert!(rendered.contains("---"));
    }

    #[test]
    fn render_includes_user_profile() {
        let profile = UserProfileSnapshot::default();
        let ctx = PromptContext::new("main")
            .with_user_profile(profile);

        let rendered = ctx.render();
        assert!(rendered.contains("## User Profile"));
    }

    #[test]
    fn render_includes_custom_instructions() {
        let ctx = PromptContext::new("main")
            .with_custom_instructions("Be extra careful with details.");

        let rendered = ctx.render();
        assert!(rendered.contains("# Additional Instructions"));
        assert!(rendered.contains("Be extra careful with details."));
    }

    #[test]
    fn render_skips_empty_sections() {
        let ctx = PromptContext::new("main")
            .with_soul("")
            .with_context("   ")
            .with_memory("Valid content");

        let rendered = ctx.render();
        assert!(!rendered.contains("---\n\n---"));
        assert!(rendered.contains("Valid content"));
    }

    #[test]
    fn serialize_deserialize_roundtrip() {
        let ctx = PromptContext::new("architect")
            .with_audience(Audience::Subagent)
            .with_soul("Soul")
            .with_os_name("Linux");

        let json = serde_json::to_string(&ctx).expect("serialize");
        let decoded: PromptContext = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(decoded.role, "architect");
        assert_eq!(decoded.audience, Audience::Subagent);
        assert_eq!(decoded.soul_content, Some("Soul".to_string()));
        assert_eq!(decoded.os_name, Some("Linux".to_string()));
    }
}