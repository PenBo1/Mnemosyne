//! 工具调用守卫 — 检测和防止工具调用循环。
//!
//! 移植自 Hermes Agent 的 `agent/tool_guardrails.py`。
//! 跟踪每轮工具调用的观察结果并返回决策：允许、警告、阻塞或停止。
//! 控制器本身是无副作用的纯逻辑层。

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

// ── 幂等/可变工具列表 ──────────────────────────────────────────────

/// 幂等工具（只读，不会改变状态）— 重复调用返回相同结果应被警告
pub const IDEMPOTENT_TOOLS: &[&str] = &[
    "read_file", "search_files", "web_search", "web_extract",
    "vision_analyze", "skills_list", "skill_view",
    "session_search",
];

/// 可变工具（会改变状态）— 不适用幂等检测
pub const MUTATING_TOOLS: &[&str] = &[
    "terminal", "write_file", "patch", "todo", "memory",
    "skill_manage", "delegate_task", "execute_code",
];

// ── 工具调用签名 ──────────────────────────────────────────────────

/// 工具调用签名 — 工具名 + 参数哈希，用于精确匹配重复调用
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ToolCallSignature {
    pub tool_name: String,
    pub args_hash: String,
}

impl ToolCallSignature {
    /// 从工具名和参数创建签名
    pub fn from_call(tool_name: &str, args: &serde_json::Value) -> Self {
        let canonical = canonical_tool_args(args);
        Self {
            tool_name: tool_name.to_string(),
            args_hash: sha256_hex(&canonical),
        }
    }
}

// ── 守卫配置 ──────────────────────────────────────────────────────

/// 工具调用守卫配置 — 控制循环检测的阈值
#[derive(Debug, Clone)]
pub struct ToolGuardrailConfig {
    /// 是否启用警告（默认启用，不阻止工具执行）
    pub warnings_enabled: bool,
    /// 是否启用硬停止（默认禁用，交互式会话应温和提示）
    pub hard_stop_enabled: bool,
    /// 相同调用失败多少次后警告
    pub exact_failure_warn_after: usize,
    /// 相同调用失败多少次后阻塞
    pub exact_failure_block_after: usize,
    /// 同一工具失败多少次后警告
    pub same_tool_failure_warn_after: usize,
    /// 同一工具失败多少次后停止
    pub same_tool_failure_halt_after: usize,
    /// 幂等工具无进展多少次后警告
    pub no_progress_warn_after: usize,
    /// 幂等工具无进展多少次后阻塞
    pub no_progress_block_after: usize,
}

impl Default for ToolGuardrailConfig {
    fn default() -> Self {
        Self {
            warnings_enabled: true,
            hard_stop_enabled: false,
            exact_failure_warn_after: 2,
            exact_failure_block_after: 5,
            same_tool_failure_warn_after: 3,
            same_tool_failure_halt_after: 8,
            no_progress_warn_after: 2,
            no_progress_block_after: 5,
        }
    }
}

// ── 守卫决策 ──────────────────────────────────────────────────────

/// 工具调用守卫决策
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolGuardrailDecision {
    /// 动作: "allow" | "warn" | "block" | "halt"
    pub action: String,
    /// 决策代码
    pub code: String,
    /// 人类可读的消息
    pub message: String,
    /// 涉及的工具名
    pub tool_name: String,
    /// 触发计数
    pub count: usize,
}

impl ToolGuardrailDecision {
    /// 是否允许工具执行
    pub fn allows_execution(&self) -> bool {
        self.action == "allow" || self.action == "warn"
    }

    /// 是否应停止循环
    pub fn should_halt(&self) -> bool {
        self.action == "block" || self.action == "halt"
    }

    /// 创建允许决策
    pub fn allow(tool_name: &str) -> Self {
        Self {
            action: "allow".to_string(),
            code: "allow".to_string(),
            message: String::new(),
            tool_name: tool_name.to_string(),
            count: 0,
        }
    }
}

// ── 守卫控制器 ──────────────────────────────────────────────────

/// 工具调用守卫控制器 — 每轮跟踪工具调用观察结果并返回决策
pub struct ToolCallGuardrailController {
    config: ToolGuardrailConfig,
    /// 精确失败计数（相同工具+相同参数）
    exact_failure_counts: HashMap<ToolCallSignature, usize>,
    /// 同一工具失败计数
    same_tool_failure_counts: HashMap<String, usize>,
    /// 幂等工具无进展记录 (signature -> (result_hash, repeat_count))
    no_progress: HashMap<ToolCallSignature, (String, usize)>,
    /// 停止决策
    halt_decision: Option<ToolGuardrailDecision>,
}

impl ToolCallGuardrailController {
    pub fn new(config: ToolGuardrailConfig) -> Self {
        Self {
            config,
            exact_failure_counts: HashMap::new(),
            same_tool_failure_counts: HashMap::new(),
            no_progress: HashMap::new(),
            halt_decision: None,
        }
    }

    /// 重置为新一轮
    pub fn reset_for_turn(&mut self) {
        self.exact_failure_counts.clear();
        self.same_tool_failure_counts.clear();
        self.no_progress.clear();
        self.halt_decision = None;
    }

    /// 获取停止决策
    pub fn halt_decision(&self) -> Option<&ToolGuardrailDecision> {
        self.halt_decision.as_ref()
    }

    /// 工具调用前检查 — 硬停止模式下的预检查
    pub fn before_call(
        &mut self,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> ToolGuardrailDecision {
        let signature = ToolCallSignature::from_call(tool_name, args);

        if !self.config.hard_stop_enabled {
            return ToolGuardrailDecision::allow(tool_name);
        }

        // 检查精确失败阻塞
        if let Some(&count) = self.exact_failure_counts.get(&signature) {
            if count >= self.config.exact_failure_block_after {
                let decision = ToolGuardrailDecision {
                    action: "block".to_string(),
                    code: "repeated_exact_failure_block".to_string(),
                    message: format!(
                        "已阻止 {}: 相同的工具调用已失败 {} 次（参数完全相同）。请停止重试，改变策略或解释障碍。",
                        tool_name, count
                    ),
                    tool_name: tool_name.to_string(),
                    count,
                };
                self.halt_decision = Some(decision.clone());
                return decision;
            }
        }

        // 检查幂等工具无进展阻塞
        if is_idempotent(tool_name) {
            if let Some((_, repeat_count)) = self.no_progress.get(&signature) {
                if *repeat_count >= self.config.no_progress_block_after {
                    let decision = ToolGuardrailDecision {
                        action: "block".to_string(),
                        code: "idempotent_no_progress_block".to_string(),
                        message: format!(
                            "已阻止 {}: 此只读调用已返回相同结果 {} 次。请使用已提供的结果或尝试不同查询。",
                            tool_name, repeat_count
                        ),
                        tool_name: tool_name.to_string(),
                        count: *repeat_count,
                    };
                    self.halt_decision = Some(decision.clone());
                    return decision;
                }
            }
        }

        ToolGuardrailDecision::allow(tool_name)
    }

    /// 工具调用后检查 — 更新计数并返回决策
    pub fn after_call(
        &mut self,
        tool_name: &str,
        args: &serde_json::Value,
        result: &str,
        failed: Option<bool>,
    ) -> ToolGuardrailDecision {
        let signature = ToolCallSignature::from_call(tool_name, args);
        let is_failed = failed.unwrap_or_else(|| classify_tool_failure(tool_name, result).0);

        if is_failed {
            // 更新精确失败计数
            let exact_count = self.exact_failure_counts
                .entry(signature.clone())
                .and_modify(|c| *c += 1)
                .or_insert(1);

            // 清除无进展记录
            self.no_progress.remove(&signature);

            // 更新同一工具失败计数
            let same_count = self.same_tool_failure_counts
                .entry(tool_name.to_string())
                .and_modify(|c| *c += 1)
                .or_insert(1);

            // 硬停止检查
            if self.config.hard_stop_enabled && *same_count >= self.config.same_tool_failure_halt_after {
                let decision = ToolGuardrailDecision {
                    action: "halt".to_string(),
                    code: "same_tool_failure_halt".to_string(),
                    message: format!(
                        "已停止 {}: 本轮已失败 {} 次。请停止重试相同工具，选择不同方法。",
                        tool_name, same_count
                    ),
                    tool_name: tool_name.to_string(),
                    count: *same_count,
                };
                self.halt_decision = Some(decision.clone());
                return decision;
            }

            // 警告检查
            if self.config.warnings_enabled && *exact_count >= self.config.exact_failure_warn_after {
                return ToolGuardrailDecision {
                    action: "warn".to_string(),
                    code: "repeated_exact_failure_warning".to_string(),
                    message: format!(
                        "{} 已以相同参数失败 {} 次。这看起来像循环，请检查错误并改变策略。",
                        tool_name, exact_count
                    ),
                    tool_name: tool_name.to_string(),
                    count: *exact_count,
                };
            }

            if self.config.warnings_enabled && *same_count >= self.config.same_tool_failure_warn_after {
                return ToolGuardrailDecision {
                    action: "warn".to_string(),
                    code: "same_tool_failure_warning".to_string(),
                    message: tool_failure_recovery_hint(tool_name, *same_count),
                    tool_name: tool_name.to_string(),
                    count: *same_count,
                };
            }

            return ToolGuardrailDecision {
                action: "allow".to_string(),
                code: "allow".to_string(),
                message: String::new(),
                tool_name: tool_name.to_string(),
                count: *exact_count,
            };
        }

        // 成功调用 — 清除失败计数
        self.exact_failure_counts.remove(&signature);
        self.same_tool_failure_counts.remove(tool_name);

        // 幂等工具无进展检测
        if !is_idempotent(tool_name) {
            self.no_progress.remove(&signature);
            return ToolGuardrailDecision::allow(tool_name);
        }

        let result_hash = result_hash(result);
        let repeat_count = if let Some((prev_hash, count)) = self.no_progress.get(&signature) {
            if *prev_hash == result_hash { count + 1 } else { 1 }
        } else {
            1
        };
        self.no_progress.insert(signature, (result_hash, repeat_count));

        if self.config.warnings_enabled && repeat_count >= self.config.no_progress_warn_after {
            return ToolGuardrailDecision {
                action: "warn".to_string(),
                code: "idempotent_no_progress_warning".to_string(),
                message: format!(
                    "{} 已返回相同结果 {} 次。请使用已提供的结果或更改查询。",
                    tool_name, repeat_count
                ),
                tool_name: tool_name.to_string(),
                count: repeat_count,
            };
        }

        ToolGuardrailDecision {
            action: "allow".to_string(),
            code: "allow".to_string(),
            message: String::new(),
            tool_name: tool_name.to_string(),
            count: repeat_count,
        }
    }
}

// ── 辅助函数 ──────────────────────────────────────────────────────

/// 生成阻塞工具调用的合成错误 JSON
pub fn toolguard_synthetic_result(decision: &ToolGuardrailDecision) -> String {
    serde_json::json!({
        "error": decision.message,
        "guardrail": {
            "action": decision.action,
            "code": decision.code,
            "tool_name": decision.tool_name,
            "count": decision.count,
        }
    })
    .to_string()
}

/// 向工具结果追加守卫指导
pub fn append_toolguard_guidance(result: &str, decision: &ToolGuardrailDecision) -> String {
    if decision.action != "warn" && decision.action != "halt" || decision.message.is_empty() {
        return result.to_string();
    }
    let label = if decision.action == "halt" { "工具循环硬停止" } else { "工具循环警告" };
    format!(
        "{}\n\n[{}: {}; count={}; {}]",
        result, label, decision.code, decision.count, decision.message
    )
}

/// 基本工具失败分类
pub fn classify_tool_failure(tool_name: &str, result: &str) -> (bool, String) {
    if result.is_empty() {
        return (false, String::new());
    }

    if tool_name == "terminal" {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(result) {
            if let Some(exit_code) = data.get("exit_code").and_then(|v| v.as_i64()) {
                if exit_code != 0 {
                    return (true, format!(" [exit {}]", exit_code));
                }
            }
        }
        return (false, String::new());
    }

    let lower = result[..result.len().min(500)].to_lowercase();
    if lower.contains("\"error\"") || lower.contains("\"failed\"") || result.starts_with("Error") {
        return (true, " [error]".to_string());
    }

    (false, String::new())
}

// ── 内部辅助 ──────────────────────────────────────────────────────

/// 排序的紧凑 JSON 序列化
fn canonical_tool_args(args: &serde_json::Value) -> String {
    match serde_json::to_string(args) {
        Ok(s) => s,
        Err(_) => args.to_string(),
    }
}

/// SHA-256 哈希（十六进制）
fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// 结果哈希（用于幂等检测）
fn result_hash(result: &str) -> String {
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(result) {
        if let Ok(canonical) = serde_json::to_string(&parsed) {
            return sha256_hex(&canonical);
        }
    }
    sha256_hex(result)
}

/// 判断工具是否为幂等（只读）
fn is_idempotent(tool_name: &str) -> bool {
    if MUTATING_TOOLS.contains(&tool_name) {
        return false;
    }
    IDEMPOTENT_TOOLS.contains(&tool_name)
}

/// 工具失败恢复提示
fn tool_failure_recovery_hint(tool_name: &str, count: usize) -> String {
    let common = format!(
        "{} 已失败 {} 次。这看起来像循环。不要切换到纯文本回复，继续使用工具，但先诊断再重试。",
        tool_name, count
    );
    if tool_name == "terminal" {
        return format!(
            "{} 对于终端失败，先在同一工具中运行诊断命令（如 `pwd && ls -la`），然后尝试绝对路径、更简单的命令或不同工具。",
            common
        );
    }
    format!(
        "{} 尝试不同参数、更窄的查询/路径、绝对路径，或使用不同工具取得进展。",
        common
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_signature_deterministic() {
        let sig1 = ToolCallSignature::from_call("terminal", &json!({"command": "ls"}));
        let sig2 = ToolCallSignature::from_call("terminal", &json!({"command": "ls"}));
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_signature_different_args() {
        let sig1 = ToolCallSignature::from_call("terminal", &json!({"command": "ls"}));
        let sig2 = ToolCallSignature::from_call("terminal", &json!({"command": "pwd"}));
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_controller_warn_on_repeated_failure() {
        let config = ToolGuardrailConfig::default();
        let mut ctrl = ToolCallGuardrailController::new(config);

        let args = json!({"command": "ls"});
        let d1 = ctrl.after_call("terminal", &args, r#"{"exit_code":1}"#, Some(true));
        assert_eq!(d1.action, "allow");

        let d2 = ctrl.after_call("terminal", &args, r#"{"exit_code":1}"#, Some(true));
        assert_eq!(d2.action, "warn");
    }

    #[test]
    fn test_classify_tool_failure() {
        let (failed, _) = classify_tool_failure("terminal", r#"{"exit_code":1}"#);
        assert!(failed);

        let (failed, _) = classify_tool_failure("terminal", r#"{"exit_code":0}"#);
        assert!(!failed);

        let (failed, _) = classify_tool_failure("read_file", "Error: file not found");
        assert!(failed);
    }

    #[test]
    fn test_allows_execution() {
        let allow = ToolGuardrailDecision::allow("test");
        assert!(allow.allows_execution());
        assert!(!allow.should_halt());

        let warn = ToolGuardrailDecision {
            action: "warn".to_string(),
            ..ToolGuardrailDecision::allow("test")
        };
        assert!(warn.allows_execution());
        assert!(!warn.should_halt());

        let block = ToolGuardrailDecision {
            action: "block".to_string(),
            ..ToolGuardrailDecision::allow("test")
        };
        assert!(!block.allows_execution());
        assert!(block.should_halt());
    }
}
