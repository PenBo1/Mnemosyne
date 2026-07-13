mod store;
mod token;
mod validator;

pub use store::{ApprovalStore, ApprovalStats};
pub use token::{
    ApprovalId, ApprovalToken, ApprovalRequest, ApprovalResult,
    calculate_action_hash,
};
pub use validator::{ApprovalValidator, ValidationError, ValidationResult};

use std::sync::{Arc, Mutex};

use crate::shared::error::AppError;
use crate::security_kernel::permission::Operation;
use crate::security_kernel::policy::calculate_operation_risk;
use crate::security_kernel::WorkspaceId;

pub struct ApprovalManager {
    store: Arc<Mutex<ApprovalStore>>,
}

impl ApprovalManager {
    pub fn new() -> Self {
        Self {
            store: ApprovalStore::shared(),
        }
    }

    pub fn with_store(store: Arc<Mutex<ApprovalStore>>) -> Self {
        Self { store }
    }

    pub fn create_token(&self, op: &Operation, workspace: WorkspaceId) -> ApprovalToken {
        let risk = calculate_operation_risk(op);
        let token = ApprovalToken::new(op, workspace, risk);

        let mut store = self.store.lock().expect("Failed to lock approval store");
        store.add_pending(token.clone());

        token
    }

    pub fn create_token_with_ttl(
        &self,
        op: &Operation,
        workspace: WorkspaceId,
        ttl_seconds: i64,
    ) -> ApprovalToken {
        let risk = calculate_operation_risk(op);
        let token = ApprovalToken::with_ttl(op, workspace, risk, ttl_seconds);

        let mut store = self.store.lock().expect("Failed to lock approval store");
        store.add_pending(token.clone());

        token
    }

    pub fn approve(&self, id: ApprovalId, approved_by: impl Into<String>) -> Result<ApprovalToken, AppError> {
        let mut store = self.store.lock().expect("Failed to lock approval store");

        if let Some(token) = store.get_pending(&id) {
            if token.is_expired() {
                store.mark_expired(id);
                return Err(AppError::sandbox_timeout());
            }

            let approved_by_str = approved_by.into();
            let token = store.approve(id)
                .map(|t| t.with_approved_by(approved_by_str))
                .ok_or_else(|| AppError::not_found(format!("Approval token not found: {}", id.0)))?;

            Ok(token)
        } else {
            Err(AppError::not_found(format!("Approval token not found: {}", id.0)))
        }
    }

    pub fn reject(&self, id: ApprovalId, reason: impl Into<String>) -> Result<ApprovalToken, AppError> {
        let mut store = self.store.lock().expect("Failed to lock approval store");

        let _reason = reason.into();
        let token = store.reject(id)
            .ok_or_else(|| AppError::not_found(format!("Approval token not found: {}", id.0)))?;

        Ok(token)
    }

    pub fn validate(&self, id: &ApprovalId, op: &Operation, workspace: &WorkspaceId) -> Result<(), AppError> {
        let store = self.store.lock().expect("Failed to lock approval store");

        if !store.is_approved(id) {
            if store.is_rejected(id) {
                return Err(ValidationError::TokenNotApproved(*id).into());
            }
            if store.is_expired(id) {
                return Err(ValidationError::TokenExpired(*id).into());
            }
            if store.is_pending(id) {
                return Err(ValidationError::TokenNotApproved(*id).into());
            }
            return Err(ValidationError::TokenNotFound(*id).into());
        }

        let pending_token = if let Some(token) = store.get_pending(id) {
            token.clone()
        } else {
            return Err(ValidationError::TokenNotFound(*id).into());
        };

        ApprovalValidator::validate_token(&pending_token, op, workspace)?;

        Ok(())
    }

    pub fn get_token(&self, id: &ApprovalId) -> Option<ApprovalToken> {
        let store = self.store.lock().expect("Failed to lock approval store");
        store.get_pending(id).cloned()
    }

    pub fn is_pending(&self, id: &ApprovalId) -> bool {
        let store = self.store.lock().expect("Failed to lock approval store");
        store.is_pending(id)
    }

    pub fn is_approved(&self, id: &ApprovalId) -> bool {
        let store = self.store.lock().expect("Failed to lock approval store");
        store.is_approved(id)
    }

    pub fn cleanup_expired(&self) -> Vec<ApprovalToken> {
        let mut store = self.store.lock().expect("Failed to lock approval store");
        store.cleanup_expired()
    }

    pub fn stats(&self) -> ApprovalStats {
        let store = self.store.lock().expect("Failed to lock approval store");
        store.stats()
    }

    pub fn pending_count(&self) -> usize {
        let store = self.store.lock().expect("Failed to lock approval store");
        store.pending_count()
    }

    pub fn approved_count(&self) -> usize {
        let store = self.store.lock().expect("Failed to lock approval store");
        store.approved_count()
    }

    pub fn get_pending_tokens(&self) -> Vec<ApprovalToken> {
        let store = self.store.lock().expect("Failed to lock approval store");
        store.get_pending_tokens().into_iter().cloned().collect()
    }

    pub fn get_pending_for_workspace(&self, workspace: &WorkspaceId) -> Vec<ApprovalToken> {
        let store = self.store.lock().expect("Failed to lock approval store");
        store.get_pending_for_workspace(workspace).into_iter().cloned().collect()
    }

    pub fn calculate_hash(op: &Operation) -> String {
        calculate_action_hash(op)
    }
}

impl Default for ApprovalManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_approval_request(
    op: Operation,
    workspace: WorkspaceId,
    reason: String,
) -> ApprovalRequest {
    let risk = calculate_operation_risk(&op);
    let token = ApprovalToken::new(&op, workspace, risk);

    ApprovalRequest {
        token,
        operation: op,
        workspace,
        reason,
    }
}