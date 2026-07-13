use crate::shared::error::{AppError, status};
use crate::security_kernel::permission::Operation;
use crate::security_kernel::WorkspaceId;

use super::token::{ApprovalId, ApprovalToken, calculate_action_hash};

#[derive(Debug, Clone)]
pub enum ValidationError {
    TokenNotFound(ApprovalId),
    TokenExpired(ApprovalId),
    TokenNotApproved(ApprovalId),
    ActionHashMismatch { expected: String, actual: String },
    WorkspaceMismatch { expected: WorkspaceId, actual: WorkspaceId },
    TokenAlreadyUsed(ApprovalId),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TokenNotFound(id) => write!(f, "Approval token not found: {}", id.0),
            Self::TokenExpired(id) => write!(f, "Approval token expired: {}", id.0),
            Self::TokenNotApproved(id) => write!(f, "Approval token not approved: {}", id.0),
            Self::ActionHashMismatch { expected, actual } => {
                write!(f, "Action hash mismatch: expected {}, got {}", expected, actual)
            }
            Self::WorkspaceMismatch { expected, actual } => {
                write!(f, "Workspace mismatch: expected {}, got {}", expected.0, actual.0)
            }
            Self::TokenAlreadyUsed(id) => write!(f, "Approval token already used: {}", id.0),
        }
    }
}

impl From<ValidationError> for AppError {
    fn from(err: ValidationError) -> Self {
        match err {
            ValidationError::TokenNotFound(_id) => {
                AppError::new(status::SANDBOX_VIOLATION, "APPROVAL_NOT_FOUND", err.to_string())
            }
            ValidationError::TokenExpired(_) => {
                AppError::sandbox_timeout()
            }
            ValidationError::TokenNotApproved(_) => {
                AppError::permission_denied(err.to_string())
            }
            ValidationError::ActionHashMismatch { .. } => {
                AppError::sandbox_violation(err.to_string())
            }
            ValidationError::WorkspaceMismatch { .. } => {
                AppError::sandbox_violation(err.to_string())
            }
            ValidationError::TokenAlreadyUsed(_) => {
                AppError::sandbox_violation(err.to_string())
            }
        }
    }
}

pub struct ApprovalValidator;

impl ApprovalValidator {
    pub fn validate_token(
        token: &ApprovalToken,
        op: &Operation,
        workspace: &WorkspaceId,
    ) -> Result<(), ValidationError> {
        Self::validate_ttl(token)?;
        Self::validate_action_hash(token, op)?;
        Self::validate_workspace(token, workspace)?;
        Ok(())
    }

    pub fn validate_ttl(token: &ApprovalToken) -> Result<(), ValidationError> {
        if token.is_expired() {
            Err(ValidationError::TokenExpired(token.id))
        } else {
            Ok(())
        }
    }

    pub fn validate_action_hash(token: &ApprovalToken, op: &Operation) -> Result<(), ValidationError> {
        let actual_hash = calculate_action_hash(op);
        if token.action_hash != actual_hash {
            Err(ValidationError::ActionHashMismatch {
                expected: token.action_hash.clone(),
                actual: actual_hash,
            })
        } else {
            Ok(())
        }
    }

    pub fn validate_workspace(token: &ApprovalToken, workspace: &WorkspaceId) -> Result<(), ValidationError> {
        if token.workspace != *workspace {
            Err(ValidationError::WorkspaceMismatch {
                expected: token.workspace,
                actual: *workspace,
            })
        } else {
            Ok(())
        }
    }

    pub fn calculate_hash(op: &Operation) -> String {
        calculate_action_hash(op)
    }

    pub fn verify_hash(token: &ApprovalToken, op: &Operation) -> bool {
        token.matches_operation(op)
    }
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
}

impl ValidationResult {
    pub fn ok() -> Self {
        Self { valid: true, errors: vec![] }
    }

    pub fn with_errors(errors: Vec<ValidationError>) -> Self {
        Self { valid: false, errors }
    }

    pub fn single_error(error: ValidationError) -> Self {
        Self { valid: false, errors: vec![error] }
    }
}

impl ApprovalValidator {
    pub fn full_validate(
        token: &ApprovalToken,
        op: &Operation,
        workspace: &WorkspaceId,
    ) -> ValidationResult {
        let mut errors = Vec::new();

        if let Err(e) = Self::validate_ttl(token) {
            errors.push(e);
        }

        if let Err(e) = Self::validate_action_hash(token, op) {
            errors.push(e);
        }

        if let Err(e) = Self::validate_workspace(token, workspace) {
            errors.push(e);
        }

        if errors.is_empty() {
            ValidationResult::ok()
        } else {
            ValidationResult::with_errors(errors)
        }
    }
}