use serde::Deserialize;

use crate::shared::error::AppError;

use super::policy::GlobalPolicy;
use super::rate_limiter::RatePolicy;
use super::resource_manager::ResourceQuota;

/// `resources/security.json` 顶层结构。
///
/// 仅声明当前已接入的字段；`fs_scope_defaults` / `network_endpoint_defaults` /
/// `approval_token` 等待对应模块提供构造入口后再扩展。
#[derive(Debug, Deserialize)]
pub struct SecurityConfig {
    pub version: u32,
    pub global_policy: GlobalPolicy,
    pub rate_policies: Vec<RatePolicy>,
    pub resource_quota: ResourceQuota,
}

/// 内置默认 security.json（编译期嵌入，随二进制发布）。
const DEFAULT_SECURITY_JSON: &str = include_str!("../../resources/security.json");

impl SecurityConfig {
    /// 解析内置默认安全配置。
    ///
    /// 配置以 `include_str!` 编译期嵌入，运行时不存在缺失情况；
    /// 若解析失败说明源文件损坏（开发期错误），返回 `AppError::internal`。
    pub fn load_default() -> Result<Self, AppError> {
        serde_json::from_str(DEFAULT_SECURITY_JSON).map_err(|e| {
            AppError::internal(format!("Failed to parse bundled security.json: {}", e))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_default_security_config_succeeds() {
        let config = SecurityConfig::load_default().expect("bundled security.json must parse");
        assert_eq!(config.version, 1);
        assert!(!config.rate_policies.is_empty());
        assert_eq!(config.resource_quota.token, Some(1_000_000));
    }

    #[test]
    fn default_global_policy_has_blocked_operations() {
        let config = SecurityConfig::load_default().expect("parse");
        assert!(config.global_policy.blocked_operations.contains("shell:rm"));
        assert!(config.global_policy.always_deny.contains("shell:mkfs"));
    }

    #[test]
    fn default_rate_policies_cover_core_operations() {
        let config = SecurityConfig::load_default().expect("parse");
        let ops: Vec<&str> = config.rate_policies.iter().map(|p| p.operation.as_str()).collect();
        assert!(ops.contains(&"ReadFile"));
        assert!(ops.contains(&"WriteFile"));
        assert!(ops.contains(&"ShellExec"));
        assert!(ops.contains(&"NetworkRequest"));
    }
}
