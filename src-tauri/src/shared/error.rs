use serde::{Deserialize, Serialize};

pub mod status {
    pub const OK: u16 = 0;
    pub const CREATED: u16 = 1;
    pub const UPDATED: u16 = 2;
    pub const DELETED: u16 = 3;
    pub const ACCEPTED: u16 = 5;
    pub const NO_CONTENT: u16 = 4;

    pub const INVALID_INPUT: u16 = 100;
    pub const MISSING_FIELD: u16 = 101;
    pub const INVALID_FORMAT: u16 = 102;
    pub const INVALID_PATH: u16 = 103;
    pub const PATH_TRAVERSAL: u16 = 104;
    pub const VALUE_OUT_OF_RANGE: u16 = 105;
    pub const DUPLICATE_ENTRY: u16 = 106;
    pub const INVALID_STATE: u16 = 107;
    pub const UNPROCESSABLE: u16 = 108;

    pub const PROVIDER_NOT_FOUND: u16 = 200;
    pub const PROVIDER_UNAVAILABLE: u16 = 201;
    pub const MODEL_NOT_FOUND: u16 = 202;
    pub const API_KEY_INVALID: u16 = 203;
    pub const API_KEY_EXPIRED: u16 = 204;
    pub const API_QUOTA_EXCEEDED: u16 = 205;
    pub const TOKEN_LIMIT_INPUT: u16 = 206;
    pub const STREAM_ERROR: u16 = 207;
    pub const AGENT_NOT_RUNNING: u16 = 208;
    pub const AGENT_BUSY: u16 = 209;

    pub const FILE_NOT_FOUND: u16 = 300;
    pub const FILE_ALREADY_EXISTS: u16 = 301;
    pub const FILE_READ_ERROR: u16 = 302;
    pub const FILE_WRITE_ERROR: u16 = 303;
    pub const FILE_PERMISSION_DENIED: u16 = 304;
    pub const DIRECTORY_NOT_FOUND: u16 = 305;
    pub const DISK_FULL: u16 = 306;

    pub const DB_CONNECTION_FAILED: u16 = 400;
    pub const DB_QUERY_FAILED: u16 = 401;
    pub const DB_CONSTRAINT_VIOLATION: u16 = 402;
    pub const DB_BUSY: u16 = 403;
    pub const DB_MIGRATION_FAILED: u16 = 404;
    pub const DB_CORRUPTION: u16 = 405;

    pub const NETWORK_TIMEOUT: u16 = 500;
    pub const NETWORK_UNREACHABLE: u16 = 501;
    pub const DNS_RESOLUTION_FAILED: u16 = 502;
    pub const CONNECTION_REFUSED: u16 = 503;

    pub const INTERNAL_ERROR: u16 = 600;
    pub const NOT_IMPLEMENTED: u16 = 601;
    pub const UNAVAILABLE: u16 = 602;
    pub const CONFIG_ERROR: u16 = 603;
    pub const NOT_FOUND: u16 = 604;
    pub const NOVEL_NOT_FOUND: u16 = 605;
    pub const CHAPTER_NOT_FOUND: u16 = 606;
    pub const SESSION_NOT_FOUND: u16 = 607;
    pub const WORKSPACE_NOT_FOUND: u16 = 608;
    pub const SKILL_NOT_FOUND: u16 = 609;
    pub const PROMPT_NOT_FOUND: u16 = 610;
    pub const PERMISSION_DENIED: u16 = 611;

    pub const SANDBOX_VIOLATION: u16 = 700;
    pub const SANDBOX_TIMEOUT: u16 = 701;
    pub const RATE_LIMIT_EXCEEDED: u16 = 702;

    pub const RESOURCE_QUOTA_EXCEEDED: u16 = 750;
    pub const RESOURCE_NOT_AVAILABLE: u16 = 751;

    pub const TASK_CANCELLED: u16 = 800;
    pub const TASK_FAILED: u16 = 801;
    pub const TASK_TIMEOUT: u16 = 802;
}

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

impl AppError {
    pub fn new(status: u16, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self { status, code: code.into(), message: message.into() }
    }

    pub fn ok(message: impl Into<String>) -> Self { Self::new(status::OK, "OK", message) }
    pub fn created(message: impl Into<String>) -> Self { Self::new(status::CREATED, "CREATED", message) }
    pub fn updated(message: impl Into<String>) -> Self { Self::new(status::UPDATED, "UPDATED", message) }
    pub fn deleted(message: impl Into<String>) -> Self { Self::new(status::DELETED, "DELETED", message) }

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

    pub fn file_not_found(path: impl Into<String>) -> Self { Self::new(status::FILE_NOT_FOUND, "FILE_NOT_FOUND", format!("File not found: {}", path.into())) }
    pub fn file_exists(path: impl Into<String>) -> Self { Self::new(status::FILE_ALREADY_EXISTS, "FILE_EXISTS", format!("File already exists: {}", path.into())) }
    pub fn file_read_error(path: impl Into<String>) -> Self { Self::new(status::FILE_READ_ERROR, "FILE_READ_ERROR", format!("Failed to read: {}", path.into())) }
    pub fn file_write_error(path: impl Into<String>) -> Self { Self::new(status::FILE_WRITE_ERROR, "FILE_WRITE_ERROR", format!("Failed to write: {}", path.into())) }
    pub fn file_permission(path: impl Into<String>) -> Self { Self::new(status::FILE_PERMISSION_DENIED, "FILE_PERMISSION_DENIED", format!("Permission denied: {}", path.into())) }
    pub fn directory_not_found(path: impl Into<String>) -> Self { Self::new(status::DIRECTORY_NOT_FOUND, "DIRECTORY_NOT_FOUND", format!("Directory not found: {}", path.into())) }
    pub fn disk_full() -> Self { Self::new(status::DISK_FULL, "DISK_FULL", "Disk full") }

    pub fn db_connection(message: impl Into<String>) -> Self { Self::new(status::DB_CONNECTION_FAILED, "DB_CONNECTION_FAILED", message) }
    pub fn db_query(message: impl Into<String>) -> Self { Self::new(status::DB_QUERY_FAILED, "DB_QUERY_FAILED", message) }
    pub fn db_constraint(message: impl Into<String>) -> Self { Self::new(status::DB_CONSTRAINT_VIOLATION, "DB_CONSTRAINT", message) }
    pub fn db_busy() -> Self { Self::new(status::DB_BUSY, "DB_BUSY", "Database busy") }
    pub fn db_migration(message: impl Into<String>) -> Self { Self::new(status::DB_MIGRATION_FAILED, "DB_MIGRATION", message) }
    pub fn db_corruption(message: impl Into<String>) -> Self { Self::new(status::DB_CORRUPTION, "DB_CORRUPTION", message) }

    pub fn network_timeout() -> Self { Self::new(status::NETWORK_TIMEOUT, "NETWORK_TIMEOUT", "Network timeout") }
    pub fn network_unreachable() -> Self { Self::new(status::NETWORK_UNREACHABLE, "NETWORK_UNREACHABLE", "Network unreachable") }
    pub fn dns_failed(host: impl Into<String>) -> Self { Self::new(status::DNS_RESOLUTION_FAILED, "DNS_FAILED", format!("DNS resolution failed: {}", host.into())) }
    pub fn connection_refused(addr: impl Into<String>) -> Self { Self::new(status::CONNECTION_REFUSED, "CONNECTION_REFUSED", format!("Connection refused: {}", addr.into())) }

    pub fn not_found(message: impl Into<String>) -> Self { Self::new(status::NOT_FOUND, "NOT_FOUND", message) }
    pub fn conflict(message: impl Into<String>) -> Self { Self::new(status::DUPLICATE_ENTRY, "CONFLICT", message) }
    pub fn forbidden(message: impl Into<String>) -> Self { Self::new(status::PERMISSION_DENIED, "FORBIDDEN", message) }
    pub fn novel_not_found() -> Self { Self::new(status::NOVEL_NOT_FOUND, "NOVEL_NOT_FOUND", "Novel not found") }
    pub fn chapter_not_found() -> Self { Self::new(status::CHAPTER_NOT_FOUND, "CHAPTER_NOT_FOUND", "Chapter not found") }
    pub fn session_not_found() -> Self { Self::new(status::SESSION_NOT_FOUND, "SESSION_NOT_FOUND", "Session not found") }
    pub fn workspace_not_found() -> Self { Self::new(status::WORKSPACE_NOT_FOUND, "WORKSPACE_NOT_FOUND", "Workspace not found") }
    pub fn skill_not_found(name: impl Into<String>) -> Self { Self::new(status::SKILL_NOT_FOUND, "SKILL_NOT_FOUND", format!("Skill '{}' not found", name.into())) }
    pub fn prompt_not_found() -> Self { Self::new(status::PROMPT_NOT_FOUND, "PROMPT_NOT_FOUND", "Prompt not found") }

    pub fn sandbox_violation(message: impl Into<String>) -> Self { Self::new(status::SANDBOX_VIOLATION, "SANDBOX_VIOLATION", message) }
    pub fn sandbox_timeout() -> Self { Self::new(status::SANDBOX_TIMEOUT, "SANDBOX_TIMEOUT", "Sandbox timeout") }
    pub fn rate_limit_exceeded(operation: &str, window: &str, current: u32, limit: u32) -> Self {
        Self::new(status::RATE_LIMIT_EXCEEDED, "RATE_LIMIT_EXCEEDED", 
            format!("Rate limit exceeded for '{}' in {} window: {} / {}", operation, window, current, limit))
    }
    pub fn permission_denied(message: impl Into<String>) -> Self { Self::new(status::PERMISSION_DENIED, "PERMISSION_DENIED", message) }

    pub fn resource_quota_exceeded(resource: impl Into<String>) -> Self { Self::new(status::RESOURCE_QUOTA_EXCEEDED, "RESOURCE_QUOTA_EXCEEDED", format!("Resource quota exceeded: {}", resource.into())) }
    pub fn resource_not_available(resource: impl Into<String>) -> Self { Self::new(status::RESOURCE_NOT_AVAILABLE, "RESOURCE_NOT_AVAILABLE", format!("Resource not available: {}", resource.into())) }

    pub fn task_cancelled() -> Self { Self::new(status::TASK_CANCELLED, "TASK_CANCELLED", "Task cancelled") }
    pub fn task_failed(message: impl Into<String>) -> Self { Self::new(status::TASK_FAILED, "TASK_FAILED", message) }
    pub fn task_timeout() -> Self { Self::new(status::TASK_TIMEOUT, "TASK_TIMEOUT", "Task timeout") }

    pub fn internal(message: impl Into<String>) -> Self { Self::new(status::INTERNAL_ERROR, "INTERNAL_ERROR", message) }
    pub fn not_implemented(message: impl Into<String>) -> Self { Self::new(status::NOT_IMPLEMENTED, "NOT_IMPLEMENTED", message) }
    pub fn unavailable(message: impl Into<String>) -> Self { Self::new(status::UNAVAILABLE, "UNAVAILABLE", message) }
    pub fn config_error(message: impl Into<String>) -> Self { Self::new(status::CONFIG_ERROR, "CONFIG_ERROR", message) }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse<T> {
    pub status: u16,
    pub code: String,
    pub message: String,
    pub data: Option<T>,
}

impl<T: Serialize> IpcResponse<T> {
    pub fn ok(data: T) -> Self {
        Self { status: status::OK, code: "OK".into(), message: "Success".into(), data: Some(data) }
    }

    pub fn created(data: T) -> Self {
        Self { status: status::CREATED, code: "CREATED".into(), message: "Resource created".into(), data: Some(data) }
    }

    pub fn updated(data: T) -> Self {
        Self { status: status::UPDATED, code: "UPDATED".into(), message: "Resource updated".into(), data: Some(data) }
    }

    pub fn deleted(data: T) -> Self {
        Self { status: status::DELETED, code: "DELETED".into(), message: "Resource deleted".into(), data: Some(data) }
    }

    pub fn accepted(data: T) -> Self {
        Self { status: status::ACCEPTED, code: "ACCEPTED".into(), message: "Request accepted".into(), data: Some(data) }
    }
}

impl IpcResponse<()> {
    pub fn no_content() -> Self {
        Self { status: status::NO_CONTENT, code: "NO_CONTENT".into(), message: "Success".into(), data: None }
    }

    pub fn ok_void() -> Self {
        Self::no_content()
    }
}