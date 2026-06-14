use std::collections::HashMap;

use tokio::sync::{mpsc, Mutex};

use super::types::*;
use super::message_handler;
use crate::infra::db::Database;
use crate::infra::llm::Provider;
use crate::infra::sandbox::enforce::SandboxEnforcer;
use crate::domain::tools::ToolRegistry;
use std::sync::Arc;

pub struct AgentLoop {
    pub(crate) rx_sub: mpsc::Receiver<Submission>,
    pub(crate) pending_approvals: HashMap<String, tokio::sync::oneshot::Sender<bool>>,
}

pub struct AgentResources {
    pub db: Arc<Mutex<Database>>,
    pub tool_registry: Arc<ToolRegistry>,
    pub provider: Arc<dyn Provider>,
    pub model: String,
    pub work_dir: String,
    pub sandbox: Arc<SandboxEnforcer>,
}

impl AgentLoop {
    pub fn new() -> (Self, mpsc::Sender<Submission>) {
        let (tx_sub, rx_sub) = mpsc::channel(512);
        let agent = Self {
            rx_sub,
            pending_approvals: HashMap::new(),
        };
        (agent, tx_sub)
    }

    pub fn build_resources(
        db: Arc<Mutex<Database>>,
        tool_registry: Arc<ToolRegistry>,
        provider: Arc<dyn Provider>,
        model: String,
        work_dir: String,
    ) -> AgentResources {
        let sandbox_policy = crate::infra::sandbox::policy::SandboxPolicy::restricted();
        let sandbox = Arc::new(SandboxEnforcer::new(sandbox_policy, std::path::PathBuf::from(&work_dir)));
        AgentResources { db, tool_registry, provider, model, work_dir, sandbox }
    }

    pub async fn run(&mut self, resources: AgentResources, tx_event: mpsc::Sender<AgentEvent>) {
        tracing::info!(model = %resources.model, "Agent loop started");
        while let Some(submission) = self.rx_sub.recv().await {
            match submission.op {
                Op::UserInput {
                    session_id,
                    content,
                } => {
                    tracing::debug!(session_id = %session_id, content_len = content.len(), "User input received");
                    message_handler::handle_user_input(&resources, &tx_event, self, &session_id, &content).await;
                }
                Op::ToolApproval {
                    tool_call_id,
                    approved,
                } => {
                    tracing::info!(tool_call_id = %tool_call_id, approved, "Tool approval processed");
                    if let Some(tx) = self.pending_approvals.remove(&tool_call_id) {
                        let _ = tx.send(approved);
                    }
                }
                Op::Cancel { session_id } => {
                    tracing::warn!(session_id = %session_id, "Agent cancelled");
                    let _ = tx_event
                        .send(AgentEvent::Error {
                            session_id,
                            error: "Cancelled".into(),
                        })
                        .await;
                }
                Op::Compact { session_id } => {
                    tracing::info!(session_id = %session_id, "Compaction triggered");
                    let _ = tx_event
                        .send(AgentEvent::CompactionTriggered { session_id })
                        .await;
                }
            }
        }
        tracing::info!("Agent loop stopped");
    }
}
