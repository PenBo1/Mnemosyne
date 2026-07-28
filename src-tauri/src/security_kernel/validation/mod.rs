//! ═══════════════════════════════════════════════════════════════════════════
//! validation - 输入验证模块
//! ═══════════════════════════════════════════════════════════════════════════

pub mod path;
pub mod url;
pub mod id;
pub mod endpoint;
pub mod layer;
pub mod context_files;
pub mod command_patterns;
pub mod sudo_guard;
pub mod patch_safety;

pub use path::{CanonicalPath, validate_path, validate_path_with_base, validate_path_for_creation};
pub use url::{validate_url, UrlValidationConfig, validate_url_for_ssrf};
pub use id::{validate_uuid, validate_uuid_v4, validate_uuid_v7};
pub use endpoint::{NetworkEndpoint, validate_endpoint, create_ai_endpoint, create_strict_endpoint, create_internal_endpoint};
pub use layer::{ValidationLayer, ValidationConfig};
pub use context_files::{scan_context_file, Severity, ThreatScanReport, PatternMatch};
pub use command_patterns::{CommandApprovalDecision, HARDLINE_PATTERNS, DANGEROUS_PATTERNS, check_command};
pub use sudo_guard::check_sudo_usage;
pub use patch_safety::{
    PatchPathChange, PatchSafetyDecision,
    is_write_patch_constrained_to_writable_paths,
    is_path_in_writable_roots,
    is_hardlink_attack, check_hardlink_attacks,
    normalize_path_lexical,
    evaluate_patch_safety,
};