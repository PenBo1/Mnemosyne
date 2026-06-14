use serde::{Deserialize, Serialize};
use super::status;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppError {
    pub status: u16,
    pub code: String,
    pub message: String,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}|{}] {}", self.status, self.code, self.message)
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        tracing::error!(error = %err, "IO error");
        Self::new(status::FILE_READ_ERROR, "IO_ERROR", err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        tracing::error!(error = %err, "JSON error");
        Self::new(status::INVALID_FORMAT, "JSON_ERROR", err.to_string())
    }
}

impl From<tauri::Error> for AppError {
    fn from(err: tauri::Error) -> Self {
        tracing::error!(error = %err, "Tauri error");
        Self::new(status::INTERNAL_ERROR, "TAURI_ERROR", err.to_string())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        tracing::error!(error = %err, "SQLite error");
        Self::new(status::DB_QUERY_FAILED, "DB_ERROR", err.to_string())
    }
}

impl AppError {
    pub fn new(status: u16, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self { status, code: code.into(), message: message.into() }
    }

    // ── 0xx: 成功 ──────────────────────────────────────────
    pub fn ok(message: impl Into<String>) -> Self { Self::new(status::OK, "OK", message) }
    pub fn created(message: impl Into<String>) -> Self { Self::new(status::CREATED, "CREATED", message) }
    pub fn updated(message: impl Into<String>) -> Self { Self::new(status::UPDATED, "UPDATED", message) }
    pub fn deleted(message: impl Into<String>) -> Self { Self::new(status::DELETED, "DELETED", message) }

    // ── 1xx: 验证/输入错误 ─────────────────────────────────
    pub fn invalid_input(message: impl Into<String>) -> Self { Self::new(status::INVALID_INPUT, "INVALID_INPUT", message) }
    pub fn bad_request(message: impl Into<String>) -> Self { Self::invalid_input(message) }
    pub fn missing_field(field: impl Into<String>) -> Self { Self::new(status::MISSING_FIELD, "MISSING_FIELD", format!("Missing field: {}", field.into())) }
    pub fn invalid_format(message: impl Into<String>) -> Self { Self::new(status::INVALID_FORMAT, "INVALID_FORMAT", message) }
    pub fn invalid_path(path: impl Into<String>) -> Self { Self::new(status::INVALID_PATH, "INVALID_PATH", format!("Invalid path: {}", path.into())) }
    pub fn path_traversal() -> Self { Self::new(status::PATH_TRAVERSAL, "PATH_TRAVERSAL", "Path traversal detected") }
    pub fn value_out_of_range(message: impl Into<String>) -> Self { Self::new(status::VALUE_OUT_OF_RANGE, "VALUE_OUT_OF_RANGE", message) }
    pub fn duplicate(entry: impl Into<String>) -> Self { Self::new(status::DUPLICATE_ENTRY, "DUPLICATE_ENTRY", format!("Duplicate: {}", entry.into())) }
    pub fn invalid_state(message: impl Into<String>) -> Self { Self::new(status::INVALID_STATE, "INVALID_STATE", message) }
    pub fn unprocessable(message: impl Into<String>) -> Self { Self::new(status::UNPROCESSABLE, "UNPROCESSABLE", message) }

    // ── 2xx: AI/LLM 错误 ──────────────────────────────────
    pub fn provider_not_found(name: impl Into<String>) -> Self { Self::new(status::PROVIDER_NOT_FOUND, "PROVIDER_NOT_FOUND", format!("Provider '{}' not found", name.into())) }
    pub fn provider_unavailable(name: impl Into<String>) -> Self { Self::new(status::PROVIDER_UNAVAILABLE, "PROVIDER_UNAVAILABLE", format!("Provider '{}' unavailable", name.into())) }
    pub fn model_not_found(name: impl Into<String>) -> Self { Self::new(status::MODEL_NOT_FOUND, "MODEL_NOT_FOUND", format!("Model '{}' not found", name.into())) }
    pub fn api_key_invalid() -> Self { Self::new(status::API_KEY_INVALID, "API_KEY_INVALID", "Invalid API key") }
    pub fn api_key_expired() -> Self { Self::new(status::API_KEY_EXPIRED, "API_KEY_EXPIRED", "API key expired") }
    pub fn api_quota_exceeded() -> Self { Self::new(status::API_QUOTA_EXCEEDED, "API_QUOTA_EXCEEDED", "API quota exceeded") }
    pub fn token_limit_exceeded(kind: impl Into<String>) -> Self { Self::new(status::TOKEN_LIMIT_INPUT, "TOKEN_LIMIT_EXCEEDED", format!("Token limit exceeded: {}", kind.into())) }
    pub fn stream_error(message: impl Into<String>) -> Self { Self::new(status::STREAM_ERROR, "STREAM_ERROR", message) }
    pub fn agent_not_running() -> Self { Self::new(status::AGENT_NOT_RUNNING, "AGENT_NOT_RUNNING", "Agent loop not running") }
    pub fn agent_busy() -> Self { Self::new(status::AGENT_BUSY, "AGENT_BUSY", "Agent is busy") }

    // ── 3xx: 文件系统错误 ──────────────────────────────────
    pub fn file_not_found(path: impl Into<String>) -> Self { Self::new(status::FILE_NOT_FOUND, "FILE_NOT_FOUND", format!("File not found: {}", path.into())) }
    pub fn file_exists(path: impl Into<String>) -> Self { Self::new(status::FILE_ALREADY_EXISTS, "FILE_EXISTS", format!("File already exists: {}", path.into())) }
    pub fn file_read_error(path: impl Into<String>) -> Self { Self::new(status::FILE_READ_ERROR, "FILE_READ_ERROR", format!("Failed to read: {}", path.into())) }
    pub fn file_write_error(path: impl Into<String>) -> Self { Self::new(status::FILE_WRITE_ERROR, "FILE_WRITE_ERROR", format!("Failed to write: {}", path.into())) }
    pub fn file_permission(path: impl Into<String>) -> Self { Self::new(status::FILE_PERMISSION_DENIED, "FILE_PERMISSION_DENIED", format!("Permission denied: {}", path.into())) }
    pub fn directory_not_found(path: impl Into<String>) -> Self { Self::new(status::DIRECTORY_NOT_FOUND, "DIRECTORY_NOT_FOUND", format!("Directory not found: {}", path.into())) }
    pub fn disk_full() -> Self { Self::new(status::DISK_FULL, "DISK_FULL", "Disk full") }

    // ── 4xx: 数据库错误 ────────────────────────────────────
    pub fn db_connection(message: impl Into<String>) -> Self { Self::new(status::DB_CONNECTION_FAILED, "DB_CONNECTION_FAILED", message) }
    pub fn db_query(message: impl Into<String>) -> Self { Self::new(status::DB_QUERY_FAILED, "DB_QUERY_FAILED", message) }
    pub fn db_constraint(message: impl Into<String>) -> Self { Self::new(status::DB_CONSTRAINT_VIOLATION, "DB_CONSTRAINT", message) }
    pub fn db_busy() -> Self { Self::new(status::DB_BUSY, "DB_BUSY", "Database busy") }
    pub fn db_corruption(message: impl Into<String>) -> Self { Self::new(status::DB_CORRUPTION, "DB_CORRUPTION", message) }

    // ── 5xx: 网络错误 ──────────────────────────────────────
    pub fn network_timeout() -> Self { Self::new(status::NETWORK_TIMEOUT, "NETWORK_TIMEOUT", "Network timeout") }
    pub fn network_unreachable() -> Self { Self::new(status::NETWORK_UNREACHABLE, "NETWORK_UNREACHABLE", "Network unreachable") }
    pub fn dns_failed(host: impl Into<String>) -> Self { Self::new(status::DNS_RESOLUTION_FAILED, "DNS_FAILED", format!("DNS resolution failed: {}", host.into())) }
    pub fn connection_refused(addr: impl Into<String>) -> Self { Self::new(status::CONNECTION_REFUSED, "CONNECTION_REFUSED", format!("Connection refused: {}", addr.into())) }

    // ── 6xx: 业务逻辑错误 ──────────────────────────────────
    pub fn not_found(message: impl Into<String>) -> Self { Self::new(status::INTERNAL_ERROR, "NOT_FOUND", message) }
    pub fn conflict(message: impl Into<String>) -> Self { Self::new(status::DUPLICATE_ENTRY, "CONFLICT", message) }
    pub fn forbidden(message: impl Into<String>) -> Self { Self::new(status::PERMISSION_DENIED, "FORBIDDEN", message) }
    pub fn novel_not_found() -> Self { Self::new(status::NOVEL_NOT_FOUND, "NOVEL_NOT_FOUND", "Novel not found") }
    pub fn chapter_not_found() -> Self { Self::new(status::CHAPTER_NOT_FOUND, "CHAPTER_NOT_FOUND", "Chapter not found") }
    pub fn session_not_found() -> Self { Self::new(status::SESSION_NOT_FOUND, "SESSION_NOT_FOUND", "Session not found") }
    pub fn workspace_not_found() -> Self { Self::new(status::WORKSPACE_NOT_FOUND, "WORKSPACE_NOT_FOUND", "Workspace not found") }
    pub fn skill_not_found(name: impl Into<String>) -> Self { Self::new(status::SKILL_NOT_FOUND, "SKILL_NOT_FOUND", format!("Skill '{}' not found", name.into())) }
    pub fn prompt_not_found() -> Self { Self::new(status::PROMPT_NOT_FOUND, "PROMPT_NOT_FOUND", "Prompt not found") }

    // ── 7xx: 沙箱/安全错误 ─────────────────────────────────
    pub fn sandbox_violation(message: impl Into<String>) -> Self { Self::new(status::SANDBOX_VIOLATION, "SANDBOX_VIOLATION", message) }
    pub fn sandbox_timeout() -> Self { Self::new(status::SANDBOX_TIMEOUT, "SANDBOX_TIMEOUT", "Sandbox timeout") }
    pub fn permission_denied(message: impl Into<String>) -> Self { Self::new(status::PERMISSION_DENIED, "PERMISSION_DENIED", message) }

    // ── 8xx: 任务/作业错误 ─────────────────────────────────
    pub fn task_cancelled() -> Self { Self::new(status::TASK_CANCELLED, "TASK_CANCELLED", "Task cancelled") }
    pub fn task_failed(message: impl Into<String>) -> Self { Self::new(status::TASK_FAILED, "TASK_FAILED", message) }
    pub fn task_timeout() -> Self { Self::new(status::TASK_TIMEOUT, "TASK_TIMEOUT", "Task timeout") }

    // ── 9xx: 系统/内部错误 ─────────────────────────────────
    pub fn internal(message: impl Into<String>) -> Self { Self::new(status::INTERNAL_ERROR, "INTERNAL_ERROR", message) }
    pub fn not_implemented(message: impl Into<String>) -> Self { Self::new(status::NOT_IMPLEMENTED, "NOT_IMPLEMENTED", message) }
    pub fn unavailable(message: impl Into<String>) -> Self { Self::new(status::UNAVAILABLE, "UNAVAILABLE", message) }
    pub fn config_error(message: impl Into<String>) -> Self { Self::new(status::CONFIG_ERROR, "CONFIG_ERROR", message) }
}
