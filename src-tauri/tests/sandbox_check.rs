//! 沙箱策略测试
//!
//! 测试 SandboxPolicy 的基本功能：
//! - 默认策略
//! - 序列化/反序列化

mod common;

use mnemosyne_lib::infrastructure::sandbox::policy::SandboxPolicy;

/// 测试默认沙箱策略
///
/// 验证：
/// - 默认策略安全配置
#[test]
fn test_default_sandbox_policy() {
    let policy = SandboxPolicy::default();
    
    assert!(policy.allow_read);
    assert!(policy.allow_write);
    assert!(!policy.allow_exec);
    assert!(policy.allow_network);
    
    assert!(!policy.blocked_commands.is_empty());
    assert!(!policy.blocked_domains.is_empty());
}

/// 测试沙箱策略克隆
///
/// 验证：
/// - SandboxPolicy 实现 Clone
#[test]
fn test_sandbox_policy_clone() {
    let policy = SandboxPolicy::default();
    let cloned = policy.clone();
    
    assert_eq!(policy.allow_read, cloned.allow_read);
    assert_eq!(policy.allow_write, cloned.allow_write);
    assert_eq!(policy.allow_exec, cloned.allow_exec);
    assert_eq!(policy.allow_network, cloned.allow_network);
}

/// 测试沙箱策略序列化
///
/// 验证：
/// - 能正确序列化为 JSON
#[test]
fn test_sandbox_policy_serialize() {
    let policy = SandboxPolicy::default();
    let json = serde_json::to_string(&policy).unwrap();
    
    assert!(json.contains("allow_read"));
    assert!(json.contains("allow_write"));
    assert!(json.contains("blocked_commands"));
}

/// 测试沙箱策略反序列化
///
/// 验证：
/// - 能从 JSON 正确解析
#[test]
fn test_sandbox_policy_deserialize() {
    let json = r#"{
        "allow_read": true,
        "allow_write": false,
        "allow_exec": false,
        "allow_network": true,
        "blocked_commands": ["rm", "del"],
        "blocked_domains": []
    }"#;
    
    let policy: SandboxPolicy = serde_json::from_str(json).unwrap();
    
    assert!(policy.allow_read);
    assert!(!policy.allow_write);
    assert!(!policy.allow_exec);
    assert!(policy.allow_network);
    assert_eq!(policy.blocked_commands.len(), 2);
}

/// 测试沙箱策略默认命令黑名单
///
/// 验证：
/// - 默认黑名单包含危险命令
#[test]
fn test_default_blocked_commands() {
    let policy = SandboxPolicy::default();
    
    assert!(policy.blocked_commands.contains(&"rm".to_string()));
    assert!(policy.blocked_commands.contains(&"format".to_string()));
    assert!(policy.blocked_commands.contains(&"shutdown".to_string()));
}

/// 测试沙箱策略默认域名黑名单
///
/// 验证：
/// - 默认黑名单包含元数据服务地址
#[test]
fn test_default_blocked_domains() {
    let policy = SandboxPolicy::default();
    
    assert!(policy.blocked_domains.iter().any(|d| d.contains("metadata.google.internal")));
    assert!(policy.blocked_domains.iter().any(|d| d.contains("169.254.169.254")));
}