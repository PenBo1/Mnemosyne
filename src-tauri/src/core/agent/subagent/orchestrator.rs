//! ═══════════════════════════════════════════════════════════════════════════
//! AgentOrchestrator - 代理编排器
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::PathBuf;
use std::collections::HashMap;

use crate::shared::error::AppError;
use crate::infrastructure::llm::registry::ProviderRegistry;

use super::types::{SubAgentRole, SubAgentResult};
use super::agent::SubAgent;

pub struct AgentOrchestrator {
    registry: ProviderRegistry,
    workspace_root: PathBuf,
    subagents: HashMap<SubAgentRole, SubAgent>,
}

impl AgentOrchestrator {
    pub fn new(registry: ProviderRegistry, workspace_root: PathBuf) -> Self {
        let mut subagents = HashMap::new();

        for role in [SubAgentRole::Researcher, SubAgentRole::Outliner, SubAgentRole::Critic] {
            subagents.insert(role, SubAgent::new(role));
        }

        Self {
            registry,
            workspace_root,
            subagents,
        }
    }

    pub async fn delegate_to_subagent(
        &self,
        role: SubAgentRole,
        task: &str,
        context: &str,
    ) -> Result<SubAgentResult, AppError> {
        let start = std::time::Instant::now();
        tracing::info!(role = ?role, task_len = task.len(), "delegate_to_subagent: enter");

        let result = async {
            let subagent = self.subagents.get(&role)
                .ok_or_else(|| AppError::internal(format!("Sub-agent {:?} not initialized", role)))?;

            let config = self.registry.active_model_config()
                .ok_or_else(AppError::no_active_model)?;

            let model = config.model.clone();
            let provider = self.registry.active_provider()?;

            subagent.execute(&provider, &model, task, context, None).await
        }.await;

        match &result {
            Ok(r) => tracing::info!(
                role = ?role,
                tokens_used = r.tokens_used,
                duration_ms = start.elapsed().as_millis(),
                "delegate_to_subagent: exit"
            ),
            Err(e) => tracing::error!(
                role = ?role,
                error = %e,
                duration_ms = start.elapsed().as_millis(),
                "delegate_to_subagent: error"
            ),
        }
        result
    }

    pub fn workspace_root(&self) -> &PathBuf {
        &self.workspace_root
    }

    pub fn registry(&self) -> &ProviderRegistry {
        &self.registry
    }
}
