//! ═══════════════════════════════════════════════════════════════════════════
//! Sandbox 常量 - 统一常量定义
//! ═══════════════════════════════════════════════════════════════════════════

// ── 输入长度限制 ────────────────────────────────────────────────────────────

/// 路径最大长度
pub const MAX_PATH_LEN: usize = 4096;

/// 命令最大长度
pub const MAX_COMMAND_LEN: usize = 10_000;

/// URL 最大长度
pub const MAX_URL_LEN: usize = 2048;

// ── 配置文件名 ──────────────────────────────────────────────────────────────

/// ExecPolicy 配置文件名（位于 data_dir 根目录）
pub const EXEC_POLICY_CONF: &str = "exec_policy.conf";

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants_reasonable() {
        assert!(MAX_PATH_LEN >= 256);
        assert!(MAX_COMMAND_LEN >= 1000);
        assert!(MAX_URL_LEN >= 256);
    }
}