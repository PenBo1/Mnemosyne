//! ═══════════════════════════════════════════════════════════════════════════
//! Engine - Agent 引擎核心
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 核心 AI 代理引擎, 管理 LLM 提供者注册表、数据库、数据目录、
//! 子代理缓存、Hook 调度器和取消令牌。

use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::{watch, RwLock};

use crate::infrastructure::db::connection::Database;
use crate::infrastructure::fs::data_dir::DataDir;
use crate::infrastructure::llm::registry::ProviderRegistry;
use crate::infrastructure::llm::types::Message;
use crate::infrastructure::telemetry::Tracer;
use crate::security_kernel::hooks::OptionalHookDispatcher;
use crate::shared::error::AppError;

use super::effort::EffortLevel;
use super::prompts;
use super::subagent::{SubAgentCache, SubAgentExecutor, TokenCounter};
use super::tools::book_ops::{
    BookEditOps, PipelineDelegateOps, ResearchOps, StubBookEditOps,
    StubPipelineDelegateOps, StubResearchOps,
};
use super::user_profile::UserProfileSnapshot;

mod send_message;
mod stream;
mod summary;
mod tools;

/// 默认 Effort 级别
const DEFAULT_EFFORT: EffortLevel = EffortLevel::Medium;
/// 最大并发子代理数
const MAX_CONCURRENT_SUBAGENTS: usize = 3;

// ── Agent 引擎 ──────────────────────────────────────────────────────────────

/// Agent 引擎实例
#[derive(Clone)]
pub struct AgentEngine {
    /// 提供者注册表
    registry: ProviderRegistry,
    /// 数据库连接
    db: Database,
    /// 数据目录
    data_dir: DataDir,
    /// 子代理缓存
    subagent_cache: Arc<SubAgentCache>,
    /// Token 计数器
    token_counter: Arc<TokenCounter>,
    /// Hook 引擎
    hook_engine: OptionalHookDispatcher,
    /// 取消令牌映射
    cancellation_tokens: Arc<DashMap<String, Arc<watch::Sender<bool>>>>,
    /// 缓存的系统提示词
    cached_system_prompt: Arc<RwLock<prompts::tiered::CachedSystemPrompt>>,
    /// 书籍编辑操作
    book_edit_ops: Arc<dyn BookEditOps>,
    /// Pipeline 委托操作
    pipeline_delegate_ops: Arc<dyn PipelineDelegateOps>,
    /// 研究操作
    research_ops: Arc<dyn ResearchOps>,
}

impl AgentEngine {
    /// 创建 Agent 引擎实例
    /// 
    /// # 参数
    /// - `registry`: 提供者注册表
    /// - `db`: 数据库连接
    /// - `data_dir`: 数据目录
    /// - `_workspace_root`: 工作区根目录
    /// - `hook_engine`: Hook 调度器
    pub fn new(
        registry: ProviderRegistry,
        db: Database,
        data_dir: DataDir,
        _workspace_root: PathBuf,
        hook_engine: OptionalHookDispatcher,
    ) -> Self {
        let cache = Arc::new(SubAgentCache::new(100, 3600));
        let token_counter = Arc::new(TokenCounter::new(1000));

        Self {
            registry,
            db,
            data_dir,
            subagent_cache: cache,
            token_counter,
            hook_engine,
            cancellation_tokens: Arc::new(DashMap::new()),
            cached_system_prompt: Arc::new(RwLock::new(None)),
            book_edit_ops: Arc::new(StubBookEditOps),
            pipeline_delegate_ops: Arc::new(StubPipelineDelegateOps),
            research_ops: Arc::new(StubResearchOps),
        }
    }

    /// 注入真实的工具操作实现
    /// 
    /// `AgentEngine::new` 默认装填 stub, 调用本方法替换为真实实现。
    /// 未调用时, 相关工具调用会显式报错。
    pub fn with_tool_ops(
        mut self,
        book_edit_ops: Arc<dyn BookEditOps>,
        pipeline_delegate_ops: Arc<dyn PipelineDelegateOps>,
        research_ops: Arc<dyn ResearchOps>,
    ) -> Self {
        self.book_edit_ops = book_edit_ops;
        self.pipeline_delegate_ops = pipeline_delegate_ops;
        self.research_ops = research_ops;
        self
    }

    /// 访问 Hook 引擎
    pub fn hook_engine(&self) -> &OptionalHookDispatcher {
        &self.hook_engine
    }

    /// 获取子代理执行器
    pub fn subagent_executor(&self) -> SubAgentExecutor<'_> {
        SubAgentExecutor::new(
            &self.registry,
            Arc::clone(&self.subagent_cache),
            Arc::clone(&self.token_counter),
            MAX_CONCURRENT_SUBAGENTS,
            self.hook_engine.clone(),
        )
    }

    /// 返回 Tracer 实例
    /// 
    /// 用于在 agent 流程中创建 trace span。
    pub fn tracer(&self) -> Tracer {
        Tracer::new(self.db.clone())
    }

    /// 构建系统提示词
    /// 
    /// 三层架构, 带会话级缓存。
    /// 首次调用加载 SOUL/CONTEXT/MEMORY 并构建 Stable/Context/Volatile 三层,
    /// 渲染后缓存。后续调用直接返回缓存字符串。
    /// 
    /// # 参数
    /// - `role`: 角色名称
    /// - `custom_instructions`: 自定义指令
    /// - `user_profile`: 用户画像
    /// - `load_extended_context`: 是否加载扩展上下文
    pub async fn build_system_prompt(
        &self,
        role: &str,
        custom_instructions: Option<&str>,
        user_profile: Option<&UserProfileSnapshot>,
        load_extended_context: bool,
    ) -> String {
        prompts::tiered::get_or_build_cached_prompt(
            &self.cached_system_prompt,
            &self.data_dir,
            role,
            custom_instructions,
            user_profile,
            load_extended_context,
        )
        .await
    }

    /// 使系统提示词缓存失效
    /// 
    /// 仅在 context compression 后调用。
    pub async fn invalidate_system_prompt(&self) {
        prompts::tiered::invalidate_cached_prompt(&self.cached_system_prompt).await;
    }

    /// 停止指定会话的 agent 流
    /// 
    /// 通过 cancellation_tokens 查找会话的 watch sender, 发送 true 触发取消。
    /// 若 session 不存在 (已结束/未启动), 返回 Ok (no-op)。
    /// 
    /// # 参数
    /// - `session_id`: 会话标识符
    pub async fn stop(&self, session_id: &str) -> Result<(), AppError> {
        let Some(token) = self.cancellation_tokens.get(session_id).map(|r| Arc::clone(&r)) else {
            tracing::debug!(
                session_id,
                "stop: no active cancellation token for session (already ended?)"
            );
            return Ok(());
        };
        if token.send(true).is_err() {
            tracing::debug!(
                session_id,
                "stop: cancellation receiver already dropped"
            );
        }
        Ok(())
    }

    /// 一次性 LLM 调用
    /// 
    /// 非流式、无工具, 返回纯文本。
    /// 供雷达扫描等不需要工具链的简单场景使用。
    /// 
    /// # 参数
    /// - `system_prompt`: 系统提示词
    /// - `user_message`: 用户消息
    /// 
    /// # 返回值
    /// 返回 LLM 响应文本
    pub async fn prompt_once(
        &self,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<String, AppError> {
        let config = self
            .registry
            .active_model_config()
            .ok_or_else(AppError::no_active_model)?;
        let model = config.model.clone();

        let provider = self.registry.active_provider()?;

        let max_tokens = EffortLevel::default().params().max_tokens_per_call;

        let response = provider
            .complete(
                &model,
                system_prompt,
                &[Message {
                    role: "user".to_string(),
                    content: user_message.to_string(),
                    tool_calls: None,
                    tool_call_id: None,
                }],
                max_tokens,
            )
            .await?;

        tracing::debug!(
            response_len = response.len(),
            response_preview = &response[..response.len().min(500)],
            "prompt_once response received"
        );
        Ok(response)
    }
}