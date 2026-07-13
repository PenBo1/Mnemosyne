use serde::{Deserialize, Serialize};
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubAgentRole {
    Researcher,
    Outliner,
    Critic,
}

impl SubAgentRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            SubAgentRole::Researcher => "researcher",
            SubAgentRole::Outliner => "outliner",
            SubAgentRole::Critic => "critic",
        }
    }

    pub fn system_prompt(&self) -> &'static str {
        match self {
            SubAgentRole::Researcher => RESEARCHER_PROMPT,
            SubAgentRole::Outliner => OUTLINER_PROMPT,
            SubAgentRole::Critic => CRITIC_PROMPT,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            SubAgentRole::Researcher => "Research information, analyze code, find relevant files and patterns",
            SubAgentRole::Outliner => "Create structured outlines, organize content, plan implementation steps",
            SubAgentRole::Critic => "Review quality, identify issues, suggest improvements and best practices",
        }
    }
}

const RESEARCHER_PROMPT: &str = r#"You are a research assistant specialized in gathering and analyzing information.

Your responsibilities:
- Search for relevant files and code patterns
- Analyze existing code structure and dependencies
- Find relevant documentation and examples
- Summarize findings clearly and concisely

Output format:
- Use bullet points for findings
- Include file paths and line references
- Keep responses focused and actionable
"#;

const OUTLINER_PROMPT: &str = r#"You are an outline specialist focused on structuring content and planning.

Your responsibilities:
- Create structured outlines for features or refactoring
- Break down complex tasks into manageable steps
- Organize content logically
- Define clear milestones and deliverables

Output format:
- Use numbered lists for sequential steps
- Use bullet points for options
- Include time estimates when relevant
"#;

const CRITIC_PROMPT: &str = r#"You are a quality reviewer focused on identifying issues and improvements.

Your responsibilities:
- Review code quality, architecture, and best practices
- Identify potential bugs, security issues, performance problems
- Suggest concrete improvements
- Validate against project conventions

Output format:
- Group findings by severity (critical/warning/info)
- Provide specific line references
- Include suggested fixes
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentResult {
    pub role: SubAgentRole,
    pub task: String,
    pub output: String,
    pub tokens_used: u32,
}