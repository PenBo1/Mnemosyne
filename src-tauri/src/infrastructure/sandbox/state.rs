use std::path::PathBuf;
use std::sync::Mutex;
use super::heuristics;
use super::execpolicy::{self, evaluator::Evaluation, ExecPolicy, NetworkProtocol};
use super::policy::SandboxPolicy;
use super::types::SandboxStatus;

/// ExecPolicy 配置文件名（位于 data_dir 根目录）。
const EXEC_POLICY_CONF: &str = "exec_policy.conf";

pub struct SandboxState {
    root: PathBuf,
    policy: Mutex<SandboxPolicy>,
    exec_policy: Mutex<ExecPolicy>,
}

impl SandboxState {
    pub fn new(root: PathBuf) -> Self {
        let exec_policy = Self::load_or_init_exec_policy(&root);
        Self {
            root,
            policy: Mutex::new(SandboxPolicy::default()),
            exec_policy: Mutex::new(exec_policy),
        }
    }

    /// 加载 exec_policy.conf；文件不存在时用默认策略初始化并持久化。
    fn load_or_init_exec_policy(root: &PathBuf) -> ExecPolicy {
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

    /// 持久化当前 ExecPolicy 到 exec_policy.conf。
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

    pub fn get_status(&self) -> SandboxStatus {
        SandboxStatus {
            enabled: true,
            root_path: self.root.display().to_string(),
            mode: "strict".to_string(),
        }
    }

    pub fn validate_path(&self, path: &PathBuf, is_write: bool) -> Result<bool, crate::shared::error::AppError> {
        let policy = self.policy.lock().unwrap_or_else(|e| e.into_inner());

        // Protected Metadata 检查:即使路径在 root 下,也不允许写入受保护元数据
        if is_write && policy.is_protected_metadata_path(path) {
            tracing::warn!(
                path = %path.display(),
                "Blocked write to protected metadata path"
            );
            return Ok(false);
        }

        // 路径规范化：对已存在的路径直接 canonicalize；对不存在的路径（写入场景）
        // canonicalize 父目录再拼接文件名。原实现对不存在路径直接 canonicalize
        // 会返回 NotFound 错误，导致所有"写新文件"的校验失败。
        let canonical = if path.exists() {
            std::fs::canonicalize(path).map_err(|e| {
                crate::shared::error::AppError::internal(format!("Failed to canonicalize: {}", e))
            })?
        } else {
            let parent = path.parent().ok_or_else(|| {
                crate::shared::error::AppError::invalid_input("path has no parent directory")
            })?;
            let canonical_parent = std::fs::canonicalize(parent).map_err(|e| {
                crate::shared::error::AppError::internal(format!(
                    "Failed to canonicalize parent: {}",
                    e
                ))
            })?;
            let file_name = path.file_name().ok_or_else(|| {
                crate::shared::error::AppError::invalid_input("path has no file name")
            })?;
            canonical_parent.join(file_name)
        };

        if !canonical.starts_with(&self.root) {
            return Ok(false);
        }

        if is_write && !policy.allow_write {
            return Ok(false);
        }

        Ok(true)
    }

    pub fn validate_command(&self, command: &str) -> Result<bool, crate::shared::error::AppError> {
        // 拒绝含控制字符（换行/回车/NUL）的命令 —— 防止 "ls\nevil" 这类
        // 命令注入绕过单行分析。`;` `|` `&` 等元字符由 heuristics 层处理。
        if command.chars().any(|c| c == '\n' || c == '\r' || c == '\0') {
            return Ok(false);
        }

        // 0. ExecPolicy 精细化评估（优先于粗粒度策略）
        {
            let exec_policy = self.exec_policy.lock().unwrap_or_else(|e| e.into_inner());
            let eval = exec_policy.evaluate_command(command);
            // 有规则匹配时，以 ExecPolicy 决策为准
            if eval.matched_index.is_some() {
                return Ok(eval.decision.is_allowed_sync());
            }
            // 无匹配时 fall through 到原有逻辑
        }

        let policy = self.policy.lock().unwrap_or_else(|e| e.into_inner());

        // 1. 黑名单 token 前缀检查(立即拒绝)。
        // 原 starts_with 字符串前缀有误匹配风险：`blocked="rm"` 会匹配 `rmdir foo`。
        // 改用 token 前缀：blocked 的所有 token 须作为 command 的前缀 token 出现。
        let cmd_tokens: Vec<&str> = command.split_whitespace().collect();
        for blocked in &policy.blocked_commands {
            let blocked_tokens: Vec<&str> = blocked.split_whitespace().collect();
            if blocked_tokens.is_empty() {
                continue;
            }
            if cmd_tokens.len() >= blocked_tokens.len()
                && blocked_tokens
                    .iter()
                    .zip(cmd_tokens.iter())
                    .all(|(b, c)| b.eq_ignore_ascii_case(c))
            {
                return Ok(false);
            }
        }

        // 2. 启发式危险模式检查(拒绝)
        if let Some(reason) = heuristics::command_might_be_dangerous(command) {
            tracing::warn!(
                command = %command,
                reason = %reason,
                "Blocked command by heuristic"
            );
            return Ok(false);
        }

        // 3. 已知安全命令(允许,即使 allow_exec=false)
        if heuristics::is_known_safe_command(command) {
            return Ok(true);
        }

        // 4. 默认行为:按 allow_exec
        Ok(policy.allow_exec)
    }

    pub fn validate_url(&self, url: &str) -> Result<bool, crate::shared::error::AppError> {
        // 0. ExecPolicy 网络规则评估
        {
            let exec_policy = self.exec_policy.lock().unwrap_or_else(|e| e.into_inner());
            let (host, protocol) = execpolicy::extract_host_and_protocol(url);
            if !host.is_empty() {
                let eval = exec_policy.evaluate_network(&host, protocol);
                if eval.matched_index.is_some() {
                    return Ok(eval.decision.is_allowed_sync());
                }
            }
            // 无匹配时 fall through
        }

        let policy = self.policy.lock().unwrap_or_else(|e| e.into_inner());

        for blocked in &policy.blocked_domains {
            if url.contains(blocked) {
                return Ok(false);
            }
        }

        Ok(policy.allow_network)
    }

    pub fn get_policy(&self) -> SandboxPolicy {
        self.policy.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    // ── ExecPolicy 访问方法 ──

    /// 获取当前 ExecPolicy 的克隆。
    pub fn get_exec_policy(&self) -> ExecPolicy {
        self.exec_policy.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// 更新 ExecPolicy 并持久化。
    pub fn update_exec_policy(&self, policy: ExecPolicy) {
        self.persist_exec_policy(&policy);
        *self.exec_policy.lock().unwrap_or_else(|e| e.into_inner()) = policy;
    }

    /// 重置 ExecPolicy 为内置默认值并持久化。
    pub fn reset_exec_policy(&self) {
        let default = execpolicy::default_policy();
        self.persist_exec_policy(&default);
        *self.exec_policy.lock().unwrap_or_else(|e| e.into_inner()) = default;
    }

    /// 评估命令（返回完整 PolicyDecision，含 AskUser）。
    pub fn evaluate_command(&self, command: &str) -> Evaluation {
        self.exec_policy.lock().unwrap_or_else(|e| e.into_inner()).evaluate_command(command)
    }

    /// 评估路径（返回完整 PolicyDecision）。
    pub fn evaluate_path(&self, path: &str) -> Evaluation {
        self.exec_policy.lock().unwrap_or_else(|e| e.into_inner()).evaluate_path(path)
    }

    /// 评估网络请求（返回完整 PolicyDecision）。
    pub fn evaluate_network(&self, host: &str, protocol: NetworkProtocol) -> Evaluation {
        self.exec_policy
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .evaluate_network(host, protocol)
    }
}
