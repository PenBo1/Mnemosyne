//! ═══════════════════════════════════════════════════════════════════════════
//! smart - 智能审批模块
//! ═══════════════════════════════════════════════════════════════════════════

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::shared::error::AppError;

/// Smart approval 默认超时（30 秒）。
pub const DEFAULT_SMART_APPROVAL_TIMEOUT: Duration = Duration::from_secs(30);

// ── 智能审批决策 ────────────────────────────────────────────────────────────────

/// Smart approval 决策。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmartApprovalDecision {
    /// 批准执行。
    Approve,
    /// 拒绝执行。
    Deny { reason: String },
    /// 升级到人工审批（保守策略：LLM 不确定或失败时）。
    Escalate { reason: String },
}

// ── LLM trait ────────────────────────────────────────────────────────────────

/// Smart approval LLM trait（抽象 aux LLM，便于测试 mock）。
///
/// 实现方负责：构造 prompt（参考 `build_smart_approval_prompt`）、调用 LLM、解析 JSON 响应。
#[async_trait]
pub trait SmartApprovalLlm: Send + Sync {
    /// 评估命令安全性，返回决策。
    async fn evaluate(&self, command: &str, context: &str) -> Result<SmartApprovalDecision, AppError>;
}

/// 构造 smart approval LLM prompt。
///
/// LLM 实现方应使用此函数构造 prompt，保证一致性。
pub fn build_smart_approval_prompt(command: &str, context: &str) -> String {
    format!(
        "You are a security approval agent. Evaluate this command for execution. Command: {command}\n\
         Context: {context}\n\
         Respond with JSON: {{\"decision\": \"APPROVE\"|\"DENY\"|\"ESCALATE\", \"reason\": \"...\"}}"
    )
}

// ── 智能审批器 ────────────────────────────────────────────────────────────────

/// Smart approval：调用 aux LLM 评估命令安全性。
///
/// 持有 `SmartApprovalLlm` trait 对象，应用 30s 超时。
/// LLM 不可用或失败时默认 `Escalate`（保守策略）。
pub struct SmartApproval {
    llm: Arc<dyn SmartApprovalLlm>,
    timeout: Duration,
}

impl SmartApproval {
    /// 使用默认超时（30s）构造。
    pub fn new(llm: Arc<dyn SmartApprovalLlm>) -> Self {
        Self {
            llm,
            timeout: DEFAULT_SMART_APPROVAL_TIMEOUT,
        }
    }

    /// 使用自定义超时构造（测试用）。
    pub fn with_timeout(llm: Arc<dyn SmartApprovalLlm>, timeout: Duration) -> Self {
        Self { llm, timeout }
    }

    /// 评估命令安全性。
    ///
    /// 调用 aux LLM 评估，应用超时。LLM 失败或超时返回 `Ok(Escalate)`（保守策略），
    /// 不向调用方传播 Err——所有非预期路径统一升级到人工审批。
    pub async fn evaluate(
        &self,
        command: &str,
        context: &str,
    ) -> Result<SmartApprovalDecision, AppError> {
        match tokio::time::timeout(self.timeout, self.llm.evaluate(command, context)).await {
            Ok(Ok(decision)) => Ok(decision),
            Ok(Err(e)) => {
                // LLM 调用失败 → 保守 Escalate（非静默降级，记录原因）
                tracing::warn!(error = %e, "smart approval LLM failed, escalating");
                Ok(SmartApprovalDecision::Escalate {
                    reason: format!("LLM evaluation failed: {}", e),
                })
            }
            Err(_) => {
                // 超时 → 保守 Escalate
                tracing::warn!("smart approval LLM timeout, escalating");
                Ok(SmartApprovalDecision::Escalate {
                    reason: "LLM evaluation timeout".to_string(),
                })
            }
        }
    }
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// mock LLM：可配置返回决策或失败。
    struct MockSmartApprovalLlm {
        decision: Mutex<Result<SmartApprovalDecision, AppError>>,
        delay: Duration,
    }

    impl MockSmartApprovalLlm {
        fn approve() -> Arc<Self> {
            Arc::new(Self {
                decision: Mutex::new(Ok(SmartApprovalDecision::Approve)),
                delay: Duration::ZERO,
            })
        }

        fn deny(reason: &str) -> Arc<Self> {
            Arc::new(Self {
                decision: Mutex::new(Ok(SmartApprovalDecision::Deny {
                    reason: reason.to_string(),
                })),
                delay: Duration::ZERO,
            })
        }

        fn escalate(reason: &str) -> Arc<Self> {
            Arc::new(Self {
                decision: Mutex::new(Ok(SmartApprovalDecision::Escalate {
                    reason: reason.to_string(),
                })),
                delay: Duration::ZERO,
            })
        }

        fn failing(err: AppError) -> Arc<Self> {
            Arc::new(Self {
                decision: Mutex::new(Err(err)),
                delay: Duration::ZERO,
            })
        }

        fn with_delay(decision: SmartApprovalDecision, delay: Duration) -> Arc<Self> {
            Arc::new(Self {
                decision: Mutex::new(Ok(decision)),
                delay,
            })
        }
    }

    #[async_trait]
    impl SmartApprovalLlm for MockSmartApprovalLlm {
        async fn evaluate(&self, _command: &str, _context: &str) -> Result<SmartApprovalDecision, AppError> {
            if self.delay > Duration::ZERO {
                tokio::time::sleep(self.delay).await;
            }
            self.decision.lock().unwrap().clone()
        }
    }

    #[test]
    fn test_build_prompt_contains_command_and_context() {
        let prompt = build_smart_approval_prompt("rm -rf /tmp", "cleanup task");
        assert!(prompt.contains("rm -rf /tmp"));
        assert!(prompt.contains("cleanup task"));
        assert!(prompt.contains("APPROVE"));
        assert!(prompt.contains("DENY"));
        assert!(prompt.contains("ESCALATE"));
    }

    #[tokio::test]
    async fn test_smart_approval_approve() {
        let approval = SmartApproval::new(MockSmartApprovalLlm::approve());
        let decision = approval.evaluate("ls -la", "list files").await.unwrap();
        assert_eq!(decision, SmartApprovalDecision::Approve);
    }

    #[tokio::test]
    async fn test_smart_approval_deny() {
        let approval = SmartApproval::new(MockSmartApprovalLlm::deny("destructive"));
        let decision = approval.evaluate("rm -rf /", "cleanup").await.unwrap();
        match decision {
            SmartApprovalDecision::Deny { reason } => assert_eq!(reason, "destructive"),
            _ => panic!("expected Deny"),
        }
    }

    #[tokio::test]
    async fn test_smart_approval_escalate() {
        let approval = SmartApproval::new(MockSmartApprovalLlm::escalate("uncertain"));
        let decision = approval.evaluate("complex cmd", "ctx").await.unwrap();
        match decision {
            SmartApprovalDecision::Escalate { reason } => assert_eq!(reason, "uncertain"),
            _ => panic!("expected Escalate"),
        }
    }

    #[tokio::test]
    async fn test_smart_approval_llm_failure_escalates() {
        // LLM 失败 → 保守 Escalate（不传播 Err）
        let approval = SmartApproval::new(MockSmartApprovalLlm::failing(AppError::internal("llm down")));
        let decision = approval.evaluate("cmd", "ctx").await.unwrap();
        match decision {
            SmartApprovalDecision::Escalate { reason } => {
                assert!(reason.contains("LLM evaluation failed"));
            }
            _ => panic!("expected Escalate on LLM failure"),
        }
    }

    #[tokio::test]
    async fn test_smart_approval_timeout_escalates() {
        // LLM 超时 → 保守 Escalate
        let approval = SmartApproval::with_timeout(
            MockSmartApprovalLlm::with_delay(
                SmartApprovalDecision::Approve,
                Duration::from_secs(2),
            ),
            Duration::from_millis(50),
        );
        let decision = approval.evaluate("cmd", "ctx").await.unwrap();
        match decision {
            SmartApprovalDecision::Escalate { reason } => {
                assert_eq!(reason, "LLM evaluation timeout");
            }
            _ => panic!("expected Escalate on timeout"),
        }
    }
}