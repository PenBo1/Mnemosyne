//! 工具审批流程 — 基于Codex的ToolOrchestrator模式。
//!
//! 工具按风险级别分类：
//! - ReadOnly: 只读操作，自动批准
//! - Mutative: 修改操作，需要用户确认
//! - Destructive: 破坏性操作，需要用户确认并显示警告

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 工具风险级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolRiskLevel {
    /// 只读操作，自动批准
    ReadOnly,
    /// 修改操作，需要用户确认
    Mutative,
    /// 破坏性操作，需要用户确认并显示警告
    Destructive,
}

/// 工具审批配置
#[derive(Debug, Clone)]
pub struct ToolApprovalConfig {
    /// 各工具的风险级别
    tool_risk_levels: HashMap<String, ToolRiskLevel>,
    /// 是否启用审批（false = 所有工具自动批准）
    approval_enabled: bool,
    /// 自动批准的工具列表（即使风险级别高）
    auto_approve_tools: Vec<String>,
}

impl Default for ToolApprovalConfig {
    fn default() -> Self {
        let mut tool_risk_levels = HashMap::new();

        // ReadOnly 工具 — 自动批准
        for tool in &[
            "read_file", "list_files", "search_memory", "search_files",
            "web_search", "web_extract", "vision_analyze",
        ] {
            tool_risk_levels.insert(tool.to_string(), ToolRiskLevel::ReadOnly);
        }

        // Mutative 工具 — 需要确认
        for tool in &[
            "write_file", "patch", "todo", "memory",
            "skill_manage", "delegate_task",
        ] {
            tool_risk_levels.insert(tool.to_string(), ToolRiskLevel::Mutative);
        }

        // Destructive 工具 — 需要确认并警告
        for tool in &[
            "terminal", "bash", "execute_code",
            "delete_file", "move_file",
        ] {
            tool_risk_levels.insert(tool.to_string(), ToolRiskLevel::Destructive);
        }

        Self {
            tool_risk_levels,
            approval_enabled: true,
            auto_approve_tools: vec![],
        }
    }
}

impl ToolApprovalConfig {
    /// 获取工具的风险级别
    pub fn risk_level(&self, tool_name: &str) -> ToolRiskLevel {
        self.tool_risk_levels
            .get(tool_name)
            .copied()
            .unwrap_or(ToolRiskLevel::Mutative) // 未知工具默认需要确认
    }

    /// 检查工具是否需要审批
    pub fn requires_approval(&self, tool_name: &str) -> bool {
        if !self.approval_enabled {
            return false;
        }
        if self.auto_approve_tools.contains(&tool_name.to_string()) {
            return false;
        }
        matches!(
            self.risk_level(tool_name),
            ToolRiskLevel::Mutative | ToolRiskLevel::Destructive
        )
    }

    /// 添加工具到自动批准列表
    pub fn add_auto_approve(&mut self, tool_name: &str) {
        if !self.auto_approve_tools.contains(&tool_name.to_string()) {
            self.auto_approve_tools.push(tool_name.to_string());
        }
    }

    /// 移除工具的自动批准
    pub fn remove_auto_approve(&mut self, tool_name: &str) {
        self.auto_approve_tools.retain(|t| t != tool_name);
    }

    /// 启用/禁用审批
    pub fn set_approval_enabled(&mut self, enabled: bool) {
        self.approval_enabled = enabled;
    }
}

/// 工具审批请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolApprovalRequest {
    /// 工具调用ID
    pub tool_call_id: String,
    /// 工具名称
    pub tool_name: String,
    /// 工具参数
    pub args: serde_json::Value,
    /// 风险级别
    pub risk_level: ToolRiskLevel,
    /// 人类可读的描述
    pub description: String,
    /// 是否是破坏性操作
    pub is_destructive: bool,
}

/// 工具审批响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolApprovalResponse {
    /// 是否批准
    pub approved: bool,
    /// 用户输入的备注（可选）
    pub note: Option<String>,
}

/// 工具审批管理器
pub struct ToolApprovalManager {
    /// 待审批的请求
    pending: HashMap<String, ToolApprovalRequest>,
    /// 审批配置
    config: ToolApprovalConfig,
}

impl ToolApprovalManager {
    pub fn new(config: ToolApprovalConfig) -> Self {
        Self {
            pending: HashMap::new(),
            config,
        }
    }

    /// 检查工具是否需要审批，如果需要则创建审批请求
    pub fn check_approval_needed(
        &mut self,
        tool_call_id: &str,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> Option<ToolApprovalRequest> {
        if !self.config.requires_approval(tool_name) {
            return None;
        }

        let risk_level = self.config.risk_level(tool_name);
        let is_destructive = risk_level == ToolRiskLevel::Destructive;

        let description = match risk_level {
            ToolRiskLevel::ReadOnly => format!("读取操作: {}", tool_name),
            ToolRiskLevel::Mutative => format!("修改操作: {} — 将修改文件或状态", tool_name),
            ToolRiskLevel::Destructive => format!(
                "⚠️ 破坏性操作: {} — 此操作可能不可逆",
                tool_name
            ),
        };

        let request = ToolApprovalRequest {
            tool_call_id: tool_call_id.to_string(),
            tool_name: tool_name.to_string(),
            args: args.clone(),
            risk_level,
            description,
            is_destructive,
        };

        self.pending.insert(tool_call_id.to_string(), request.clone());
        Some(request)
    }

    /// 处理审批响应
    pub fn handle_response(
        &mut self,
        tool_call_id: &str,
        response: ToolApprovalResponse,
    ) -> Option<ToolApprovalRequest> {
        self.pending.remove(tool_call_id)
    }

    /// 获取所有待审批的请求
    pub fn pending_requests(&self) -> Vec<&ToolApprovalRequest> {
        self.pending.values().collect()
    }

    /// 清除所有待审批的请求
    pub fn clear(&mut self) {
        self.pending.clear();
    }

    /// 获取配置
    pub fn config(&self) -> &ToolApprovalConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_levels() {
        let config = ToolApprovalConfig::default();
        assert_eq!(config.risk_level("read_file"), ToolRiskLevel::ReadOnly);
        assert_eq!(config.risk_level("write_file"), ToolRiskLevel::Mutative);
        assert_eq!(config.risk_level("bash"), ToolRiskLevel::Destructive);
        assert_eq!(config.risk_level("unknown_tool"), ToolRiskLevel::Mutative);
    }

    #[test]
    fn test_requires_approval() {
        let config = ToolApprovalConfig::default();
        assert!(!config.requires_approval("read_file"));
        assert!(config.requires_approval("write_file"));
        assert!(config.requires_approval("bash"));
    }

    #[test]
    fn test_auto_approve() {
        let mut config = ToolApprovalConfig::default();
        config.add_auto_approve("write_file");
        assert!(!config.requires_approval("write_file"));
        assert!(config.requires_approval("bash"));
    }

    #[test]
    fn test_approval_disabled() {
        let mut config = ToolApprovalConfig::default();
        config.set_approval_enabled(false);
        assert!(!config.requires_approval("write_file"));
        assert!(!config.requires_approval("bash"));
    }

    #[test]
    fn test_approval_manager() {
        let mut manager = ToolApprovalManager::new(ToolApprovalConfig::default());
        let args = serde_json::json!({"path": "test.txt"});

        // ReadOnly tool - no approval needed
        assert!(manager.check_approval_needed("call-1", "read_file", &args).is_none());

        // Mutative tool - approval needed
        let request = manager.check_approval_needed("call-2", "write_file", &args);
        assert!(request.is_some());
        let request = request.unwrap();
        assert_eq!(request.tool_name, "write_file");
        assert_eq!(request.risk_level, ToolRiskLevel::Mutative);

        // Handle response
        let response = ToolApprovalResponse {
            approved: true,
            note: None,
        };
        let removed = manager.handle_response("call-2", response);
        assert!(removed.is_some());
        assert!(manager.pending.is_empty());
    }
}
