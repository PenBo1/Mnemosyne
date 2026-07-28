//! ═══════════════════════════════════════════════════════════════════════════
//! Context Engine - 上下文引擎抽象
//! ═══════════════════════════════════════════════════════════════════════════

use async_trait::async_trait;

use crate::infrastructure::llm::types::Message;
use crate::shared::error::AppError;

/// 上下文引擎 trait
/// 
/// 可插拔抽象, 为后续压缩算法提供统一入口。
#[async_trait]
pub trait ContextEngine: Send + Sync {
    /// 获取引擎名称
    fn name(&self) -> &str;

    /// 更新 token 使用信息
    /// 
    /// # 参数
    /// - `prompt_tokens`: 提示词 token 数
    /// - `completion_tokens`: 补全 token 数
    async fn update_from_response(
        &self,
        prompt_tokens: u32,
        completion_tokens: u32,
    ) -> Result<(), AppError>;

    /// 判断是否需要压缩
    /// 
    /// # 参数
    /// - `current_tokens`: 当前 token 数
    async fn should_compress(&self, current_tokens: u32) -> bool;

    /// 执行压缩
    /// 
    /// # 参数
    /// - `messages`: 消息列表
    /// - `current_tokens`: 当前 token 数
    /// - `focus_topic`: 焦点主题
    /// 
    /// # 返回值
    /// 返回压缩后的消息列表
    async fn compress(
        &self,
        messages: Vec<Message>,
        current_tokens: u32,
        focus_topic: Option<&str>,
    ) -> Result<Vec<Message>, AppError>;

    /// API 调用前的快速估算
    async fn should_compress_preflight(&self, _messages: &[Message]) -> bool {
        false
    }

    /// 判断是否有内容可压缩
    async fn has_content_to_compress(&self, messages: &[Message]) -> bool {
        !messages.is_empty()
    }

    /// 会话生命周期钩子
    async fn on_session_start(&self) {}
    async fn on_session_end(&self) {}
    async fn on_session_reset(&self) {}

    /// 获取状态信息
    async fn get_status(&self) -> ContextEngineStatus;
}

/// 上下文引擎状态
/// 
/// 标准化 token 字段, 用于跨实现统一暴露上下文引擎的运行时状态。
#[derive(Debug, Clone, Default)]
pub struct ContextEngineStatus {
    /// 上一次 API 响应的 prompt token 数
    pub last_prompt_tokens: u32,
    /// 上一次 API 响应的 completion token 数
    pub last_completion_tokens: u32,
    /// 上一次 API 响应的合计 token 数
    pub last_total_tokens: u32,
    /// 触发压缩的 token 阈值
    pub threshold_tokens: u32,
    /// 当前模型上下文窗口长度
    pub context_length: u32,
    /// 累计压缩次数
    pub compression_count: u32,
}