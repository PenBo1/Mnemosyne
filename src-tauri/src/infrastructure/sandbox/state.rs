//! ═══════════════════════════════════════════════════════════════════════════
//! 沙箱状态 - 状态管理
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 仅负责状态持有和访问，验证逻辑委托给 validator 模块。

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use super::constants::EXEC_POLICY_CONF;
use super::validator::{PathValidator, CommandValidator, UrlValidator};
use super::execpolicy::{self, evaluator::Evaluation, ExecPolicy, NetworkProtocol};
use super::policy::SandboxPolicy;
use super::types::SandboxStatus;

// ── 沙箱状态 ────────────────────────────────────────────────────────────────

/// 沙箱状态容器
///
/// 管理沙箱的策略配置和执行策略。
/// 验证逻辑委托给 validator 模块。
pub struct SandboxState {
    root: PathBuf,
    policy: Mutex<SandboxPolicy>,
    exec_policy: Mutex<ExecPolicy>,
}

impl SandboxState {
    /// 创建沙箱状态
    pub fn new(root: PathBuf) -> Self {
        let exec_policy = Self::load_or_init_exec_policy(&root);
        Self {
            root,
            policy: Mutex::new(SandboxPolicy::default()),
            exec_policy: Mutex::new(exec_policy),
        }
    }

    // ── 状态访问 ────────────────────────────────────────────────────────────

    /// 获取沙箱状态
    pub fn get_status(&self) -> SandboxStatus {
        SandboxStatus {
            enabled: true,
            root_path: self.root.display().to_string(),
            mode: "strict".to_string(),
        }
    }

    /// 获取沙箱策略
    pub fn get_policy(&self) -> SandboxPolicy {
        self.policy.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// 获取执行策略
    pub fn get_exec_policy(&self) -> ExecPolicy {
        self.exec_policy.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    // ── 验证方法（委托给 validator）────────────────────────────────────────

    /// 验证路径
    pub fn validate_path(&self, path: &PathBuf, is_write: bool) -> Result<bool, crate::shared::error::AppError> {
        let policy = self.policy.lock().unwrap_or_else(|e| e.into_inner());
        let validator = PathValidator::new(&self.root, &policy);
        validator.validate(path, is_write)
    }

    /// 验证命令
    pub fn validate_command(&self, command: &str) -> Result<bool, crate::shared::error::AppError> {
        let policy = self.policy.lock().unwrap_or_else(|e| e.into_inner());
        let exec_policy = self.exec_policy.lock().unwrap_or_else(|e| e.into_inner());
        let validator = CommandValidator::new(&policy, &exec_policy);
        validator.validate(command)
    }

    /// 验证 URL
    pub fn validate_url(&self, url: &str) -> Result<bool, crate::shared::error::AppError> {
        let policy = self.policy.lock().unwrap_or_else(|e| e.into_inner());
        let exec_policy = self.exec_policy.lock().unwrap_or_else(|e| e.into_inner());
        let validator = UrlValidator::new(&policy, &exec_policy);
        validator.validate(url)
    }

    // ── 评估方法 ────────────────────────────────────────────────────────────

    /// 评估命令（返回完整 PolicyDecision）
    pub fn evaluate_command(&self, command: &str) -> Evaluation {
        self.exec_policy.lock().unwrap_or_else(|e| e.into_inner()).evaluate_command(command)
    }

    /// 评估路径
    pub fn evaluate_path(&self, path: &str) -> Evaluation {
        self.exec_policy.lock().unwrap_or_else(|e| e.into_inner()).evaluate_path(path)
    }

    /// 评估网络请求
    pub fn evaluate_network(&self, host: &str, protocol: NetworkProtocol) -> Evaluation {
        self.exec_policy
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .evaluate_network(host, protocol)
    }

    // ── 策略更新 ────────────────────────────────────────────────────────────

    /// 更新执行策略并持久化
    pub fn update_exec_policy(&self, policy: ExecPolicy) {
        self.persist_exec_policy(&policy);
        *self.exec_policy.lock().unwrap_or_else(|e| e.into_inner()) = policy;
    }

    /// 重置执行策略为默认值
    pub fn reset_exec_policy(&self) {
        let default = execpolicy::default_policy();
        self.persist_exec_policy(&default);
        *self.exec_policy.lock().unwrap_or_else(|e| e.into_inner()) = default;
    }

    // ── 内部方法 ────────────────────────────────────────────────────────────

    /// 加载或初始化执行策略
    fn load_or_init_exec_policy(root: &Path) -> ExecPolicy {
        let conf_path = root.join(EXEC_POLICY_CONF);
        if conf_path.exists() {
            match std::fs::read_to_string(&conf_path) {
                Ok(text) => match execpolicy::parse(&text) {
                    Ok(p) => {
                        tracing::info!(path = %conf_path.display(), "ExecPolicy loaded");
                        return p;
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            path = %conf_path.display(),
                            "Failed to parse exec_policy.conf; using default"
                        );
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        path = %conf_path.display(),
                        "Failed to read exec_policy.conf; using default"
                    );
                }
            }
        }
        // 文件不存在或解析失败：用默认策略并持久化
        let default = execpolicy::default_policy();
        let text = execpolicy::serialize(&default);
        if let Err(e) = std::fs::write(&conf_path, &text) {
            tracing::warn!(
                error = %e,
                path = %conf_path.display(),
                "Failed to write default exec_policy.conf"
            );
        }
        default
    }

    /// 持久化执行策略
    fn persist_exec_policy(&self, policy: &ExecPolicy) {
        let conf_path = self.root.join(EXEC_POLICY_CONF);
        let text = execpolicy::serialize(policy);
        if let Err(e) = std::fs::write(&conf_path, &text) {
            tracing::warn!(
                error = %e,
                path = %conf_path.display(),
                "Failed to persist exec_policy.conf"
            );
        }
    }
}