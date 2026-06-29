use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessConfig {
    pub pipeline: PipelineSettings,
    pub agents: HashMap<String, AgentSlotConfig>,
    pub quality: QualitySettings,
    pub gc_policy: GcPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSettings {
    pub max_revision_loops: u32,
    pub max_concurrent_books: u32,
    pub default_model: String,
    #[serde(default)]
    pub model_overrides: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSlotConfig {
    pub role: String,
    #[serde(default)]
    pub model_override: Option<String>,
    #[serde(default)]
    pub tools_allowed: Vec<String>,
    #[serde(default)]
    pub tools_denied: Vec<String>,
    pub token_budget: u32,
    #[serde(default)]
    pub sandbox_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitySettings {
    pub min_audit_score: f64,
    pub max_critical_issues: u32,
    pub word_count_variance_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcPolicy {
    pub stale_snapshot_days: u32,
    pub gc_interval_chapters: u32,
}

impl HarnessConfig {
    pub fn load_from_file(path: &Path) -> Result<Self, crate::shared::errors::AppError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            crate::shared::errors::AppError::internal(format!("Failed to read harness config: {}", e))
        })?;
        Self::load_from_str(&content)
    }

    pub fn load_from_str(content: &str) -> Result<Self, crate::shared::errors::AppError> {
        serde_json::from_str(content).map_err(|e| {
            crate::shared::errors::AppError::internal(format!("Failed to parse harness config: {}", e))
        })
    }
}
