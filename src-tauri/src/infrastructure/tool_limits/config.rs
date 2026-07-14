// ToolLimitsConfig —— 工具执行上限配置数据模型。
//
// 序列化为 <data_dir>/tool_limits.json,pretty-printed 便于人工查看。

use serde::{Deserialize, Serialize};

use crate::infrastructure::fs::data_dir::DataDir;
use crate::shared::error::AppError;

/// 每对话工具执行上限配置。
///
/// "对话" = 一次 send_message 调用,从用户发送消息到 Agent 返回 finish 事件。
/// 此配置控制在该范围内工具调用的安全阀,防止异常循环。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolLimitsConfig {
    /// 单次对话中工具调用总次数上限(0 = 不限制)
    ///
    /// 默认 50。达到上限时 Agent 应停止调用工具并返回当前结果。
    pub max_tool_calls_per_dialog: u32,

    /// 连续失败次数上限(0 = 不限制)
    ///
    /// 默认 5。当连续 N 次工具调用返回错误时,Agent 应停止并报告失败原因。
    /// 防止 agent 卡在反复重试同一失败工具的循环中。
    pub max_consecutive_failures: u32,

    /// 单次工具调用超时(毫秒,0 = 不限制)
    ///
    /// 默认 60000(60 秒)。超过此时间未返回的工具调用应被中止。
    pub timeout_per_call_ms: u64,
}

impl Default for ToolLimitsConfig {
    fn default() -> Self {
        Self {
            max_tool_calls_per_dialog: 50,
            max_consecutive_failures: 5,
            timeout_per_call_ms: 60_000,
        }
    }
}

impl ToolLimitsConfig {
    /// 校验配置项是否在合理范围内。
    ///
    /// 失败策略(对齐 "no silent fallback"):任何字段越界都返回 Err,
    /// 不静默 clamp 到默认值。
    pub fn validate(&self) -> Result<(), AppError> {
        // 上限 1000 防止误填超大值导致性能问题
        if self.max_tool_calls_per_dialog > 1000 {
            return Err(AppError::bad_request(format!(
                "max_tool_calls_per_dialog exceeds hard limit ({} > 1000)",
                self.max_tool_calls_per_dialog
            )));
        }
        if self.max_consecutive_failures > 100 {
            return Err(AppError::bad_request(format!(
                "max_consecutive_failures exceeds hard limit ({} > 100)",
                self.max_consecutive_failures
            )));
        }
        // 超时上限 10 分钟,防止误填导致永久阻塞
        if self.timeout_per_call_ms > 600_000 {
            return Err(AppError::bad_request(format!(
                "timeout_per_call_ms exceeds hard limit ({} > 600000)",
                self.timeout_per_call_ms
            )));
        }
        Ok(())
    }

    /// 从 <data_dir>/tool_limits.json 加载配置。
    ///
    /// - 文件不存在 → 返回 Default
    /// - 文件存在但解析失败 → 返回 Err(不静默回退默认,让用户感知配置损坏)
    pub fn load(data_dir: &DataDir) -> Result<Self, AppError> {
        let path = data_dir.root().join("tool_limits.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path).map_err(|e| {
            AppError::internal(format!("Failed to read tool_limits.json: {}", e))
        })?;
        let config: Self = serde_json::from_str(&content).map_err(|e| {
            AppError::internal(format!("Failed to parse tool_limits.json: {}", e))
        })?;
        config.validate()?;
        Ok(config)
    }

    /// 保存配置到 <data_dir>/tool_limits.json(pretty-printed)。
    ///
    /// 保存前会先 validate,任一字段越界都返回 Err。
    pub fn save(&self, data_dir: &DataDir) -> Result<(), AppError> {
        self.validate()?;
        let path = data_dir.root().join("tool_limits.json");
        let content = serde_json::to_string_pretty(self).map_err(|e| {
            AppError::internal(format!("Failed to serialize tool_limits.json: {}", e))
        })?;
        std::fs::write(&path, content).map_err(|e| {
            AppError::internal(format!("Failed to write tool_limits.json: {}", e))
        })?;
        tracing::info!(path = %path.display(), "tool_limits.json saved");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values_are_reasonable() {
        let c = ToolLimitsConfig::default();
        assert_eq!(c.max_tool_calls_per_dialog, 50);
        assert_eq!(c.max_consecutive_failures, 5);
        assert_eq!(c.timeout_per_call_ms, 60_000);
    }

    #[test]
    fn validate_accepts_default() {
        let c = ToolLimitsConfig::default();
        assert!(c.validate().is_ok());
    }

    #[test]
    fn validate_rejects_oversized_tool_calls() {
        let mut c = ToolLimitsConfig::default();
        c.max_tool_calls_per_dialog = 1001;
        assert!(c.validate().is_err());
    }

    #[test]
    fn validate_rejects_oversized_failures() {
        let mut c = ToolLimitsConfig::default();
        c.max_consecutive_failures = 101;
        assert!(c.validate().is_err());
    }

    #[test]
    fn validate_rejects_oversized_timeout() {
        let mut c = ToolLimitsConfig::default();
        c.timeout_per_call_ms = 600_001;
        assert!(c.validate().is_err());
    }

    #[test]
    fn validate_accepts_zero_limits() {
        let c = ToolLimitsConfig {
            max_tool_calls_per_dialog: 0,
            max_consecutive_failures: 0,
            timeout_per_call_ms: 0,
        };
        assert!(c.validate().is_ok());
    }

    #[test]
    fn serde_roundtrip() {
        let c = ToolLimitsConfig::default();
        let json = serde_json::to_string(&c).unwrap();
        let parsed: ToolLimitsConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.max_tool_calls_per_dialog, c.max_tool_calls_per_dialog);
        assert_eq!(parsed.max_consecutive_failures, c.max_consecutive_failures);
        assert_eq!(parsed.timeout_per_call_ms, c.timeout_per_call_ms);
    }

    #[test]
    fn serde_uses_camel_case() {
        let c = ToolLimitsConfig::default();
        let json = serde_json::to_string(&c).unwrap();
        // rename_all = "camelCase"
        assert!(json.contains("maxToolCallsPerDialog"));
        assert!(json.contains("maxConsecutiveFailures"));
        assert!(json.contains("timeoutPerCallMs"));
        assert!(!json.contains("max_tool_calls_per_dialog"));
    }
}
