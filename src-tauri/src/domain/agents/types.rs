use serde::{Deserialize, Serialize};

/// Agent roles in the pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Architect,
    FoundationReviewer,
    Planner,
    Composer,
    Writer,
    LengthNormalizer,
    Auditor,
    Reviser,
    Observer,
    Reflector,
    Radar,
    Detector,
}

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Architect => write!(f, "architect"),
            Self::FoundationReviewer => write!(f, "foundation-reviewer"),
            Self::Planner => write!(f, "planner"),
            Self::Composer => write!(f, "composer"),
            Self::Writer => write!(f, "writer"),
            Self::LengthNormalizer => write!(f, "length-normalizer"),
            Self::Auditor => write!(f, "auditor"),
            Self::Reviser => write!(f, "reviser"),
            Self::Observer => write!(f, "observer"),
            Self::Reflector => write!(f, "reflector"),
            Self::Radar => write!(f, "radar"),
            Self::Detector => write!(f, "detector"),
        }
    }
}

/// LLM response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
