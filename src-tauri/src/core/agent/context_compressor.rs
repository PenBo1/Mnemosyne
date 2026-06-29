//! 上下文压缩器 — 自动压缩长对话上下文。
//!
//! 移植自 Hermes Agent 的 `agent/context_compressor.py`。
//! 使用辅助模型（廉价/快速）摘要中间轮次，同时保护头部和尾部上下文。
//! 支持迭代摘要更新（跨多次压缩保留信息）。

use serde::{Serialize, Deserialize};
use crate::shared::errors::AppError;
use crate::infrastructure::llm_client::{Provider, types::Message};

// ── 常量 ──────────────────────────────────────────────────────────

/// 压缩摘要前缀 — 标记这是历史上下文
pub const SUMMARY_PREFIX: &str = "[上下文压缩 — 仅供参考] 之前的对话已压缩为以下摘要。这是从上一个上下文窗口的交接——将其视为背景参考，而非活跃指令。请仅回复摘要后出现的最新用户消息——它是当前应做之事的唯一真实来源。";

/// 摘要结束标记
pub const SUMMARY_END_MARKER: &str = "--- 上下文摘要结束 — 请回复下方的消息，而非摘要上方的内容 ---";

/// 旧工具输出占位符
pub const PRUNED_TOOL_PLACEHOLDER: &str = "[旧工具输出已清除以节省上下文空间]";

// ── 配置 ──────────────────────────────────────────────────────────

/// 上下文压缩配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressorConfig {
    /// 触发压缩的 token 阈值（占上下文窗口的比例，默认 0.75）
    pub threshold_ratio: f64,
    /// 压缩后保留的最近消息数（默认 10）
    pub preserve_recent_count: usize,
    /// 摘要最大 token 数（默认 4000）
    pub max_summary_tokens: usize,
    /// 最小摘要 token 数（默认 500）
    pub min_summary_tokens: usize,
    /// 摘要占压缩内容的比例（默认 0.20）
    pub summary_ratio: f64,
    /// 工具输出截断阈值（字符数，默认 2000）
    pub tool_output_truncate_at: usize,
}

impl Default for CompressorConfig {
    fn default() -> Self {
        Self {
            threshold_ratio: 0.75,
            preserve_recent_count: 10,
            max_summary_tokens: 4000,
            min_summary_tokens: 500,
            summary_ratio: 0.20,
            tool_output_truncate_at: 2000,
        }
    }
}

// ── 可压缩消息 ──────────────────────────────────────────────────

/// 可压缩的消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressibleMessage {
    /// 角色: "system" | "user" | "assistant" | "tool"
    pub role: String,
    /// 消息内容
    pub content: String,
    /// 工具调用（仅 assistant 消息）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallRef>>,
    /// 工具调用 ID（仅 tool 消息）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// 工具调用引用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRef {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

impl CompressibleMessage {
    pub fn user(content: &str) -> Self {
        Self { role: "user".into(), content: content.to_string(), tool_calls: None, tool_call_id: None }
    }
    pub fn assistant(content: &str) -> Self {
        Self { role: "assistant".into(), content: content.to_string(), tool_calls: None, tool_call_id: None }
    }
    pub fn system(content: &str) -> Self {
        Self { role: "system".into(), content: content.to_string(), tool_calls: None, tool_call_id: None }
    }
    pub fn tool(content: &str, tool_call_id: &str) -> Self {
        Self { role: "tool".into(), content: content.to_string(), tool_calls: None, tool_call_id: Some(tool_call_id.to_string()) }
    }

    /// 大致 token 估算（中文约 1.5 字符/token，英文约 4 字符/token）
    pub fn estimate_tokens(&self) -> usize {
        let chars = self.content.len();
        // 粗略估算：混合中英文场景
        chars / 3
    }
}

// ── 压缩摘要 ──────────────────────────────────────────────────

/// 压缩后的摘要消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedSummary {
    /// 摘要文本
    pub summary: String,
    /// 被压缩的消息数量
    pub messages_compressed: usize,
    /// 压缩前的大致 token 数
    pub tokens_before: usize,
    /// 压缩后的大致 token 数
    pub tokens_after: usize,
    /// 压缩时间戳
    pub compressed_at: String,
    /// 历史任务快照
    pub historical_tasks: Vec<String>,
    /// 待处理问题
    pub pending_questions: Vec<String>,
}

// ── 上下文压缩器 ──────────────────────────────────────────────

/// 上下文压缩器
pub struct ContextCompressor {
    config: CompressorConfig,
    /// 上一次压缩的摘要（用于迭代更新）
    previous_summary: Option<CompressedSummary>,
}

impl ContextCompressor {
    pub fn new(config: CompressorConfig) -> Self {
        Self { config, previous_summary: None }
    }

    /// 检查是否需要压缩
    pub fn should_compress(&self, estimated_tokens: usize, context_length: usize) -> bool {
        estimated_tokens as f64 > context_length as f64 * self.config.threshold_ratio
    }

    /// 执行上下文压缩：调用 LLM 摘要中间消息，保留尾部最近消息。
    ///
    /// 流程：
    /// 1. 估算总 token，若未超阈值则原样返回
    /// 2. 分割为"待压缩"和"最近保留"两部分（保留尾部 preserve_recent_count 条）
    /// 3. 构建摘要提示词（含上一次摘要以支持迭代更新）
    /// 4. 调用 LLM 生成摘要
    /// 5. 解析摘要并构建压缩后上下文（摘要系统消息 + 最近消息）
    ///
    /// 若消息数 ≤ preserve_recent_count，无法分割，原样返回
    /// （避免压缩全部后无"最近"上下文可供 LLM 续接）。
    pub async fn compress(
        &mut self,
        messages: &[CompressibleMessage],
        provider: &dyn Provider,
        model: &str,
        context_length: usize,
    ) -> Result<Vec<CompressibleMessage>, AppError> {
        // 1. 检查是否需要压缩
        let total_tokens: usize = messages.iter().map(|m| m.estimate_tokens()).sum();
        if !self.should_compress(total_tokens, context_length) {
            return Ok(messages.to_vec());
        }

        // 2. 分割：保留尾部 preserve_recent_count 条，压缩其余
        let preserve = self.config.preserve_recent_count;
        if messages.len() <= preserve {
            return Ok(messages.to_vec());
        }
        let split = messages.len() - preserve;
        let (to_compress, recent) = messages.split_at(split);

        // 3. 构建摘要提示词（含上一次摘要以支持迭代更新）
        let prev = self.previous_summary.as_ref().map(|s| s.summary.as_str());
        let prompt = self.build_summarizer_prompt(to_compress, prev);

        // 4. 调用 LLM 生成摘要
        let system = "你是对话摘要助手。请按用户指定的格式输出结构化摘要。";
        let user_msg = Message {
            role: "user".to_string(),
            content: prompt,
            tool_calls: None,
            tool_call_id: None,
        };
        let summary_text = provider.complete(model, system, &[user_msg]).await?;

        // 5. 解析摘要并构建压缩后上下文
        let summary = self.parse_summary_response(&summary_text, to_compress.len(), total_tokens);
        tracing::info!(
            messages_compressed = summary.messages_compressed,
            tokens_before = summary.tokens_before,
            tokens_after = summary.tokens_after,
            "Context compressed via LLM summary"
        );
        Ok(self.build_compressed_context(&summary, recent))
    }

    /// 构建摘要提示词 — 供 LLM 摘要使用
    pub fn build_summarizer_prompt(
        &self,
        messages: &[CompressibleMessage],
        previous_summary: Option<&str>,
    ) -> String {
        let mut prompt = String::new();

        prompt.push_str("你是一个对话摘要专家。请将以下对话压缩为结构化摘要。\n\n");
        prompt.push_str("要求：\n");
        prompt.push_str("1. 保留关键决策、结论和行动项\n");
        prompt.push_str("2. 保留角色、设定、情节等小说创作相关信息\n");
        prompt.push_str("3. 保留待处理的问题和未完成的任务\n");
        prompt.push_str("4. 删除冗余信息、重复内容和不重要的细节\n");
        prompt.push_str("5. 使用中文输出\n\n");

        if let Some(prev) = previous_summary {
            prompt.push_str("## 上一次压缩的摘要（请在此基础上更新）\n\n");
            prompt.push_str(prev);
            prompt.push_str("\n\n---\n\n");
        }

        prompt.push_str("## 待压缩的对话\n\n");
        for msg in messages {
            let role_label = match msg.role.as_str() {
                "system" => "系统",
                "user" => "用户",
                "assistant" => "助手",
                "tool" => "工具",
                _ => &msg.role,
            };
            prompt.push_str(&format!("**{}**: ", role_label));
            // 截断过长的工具输出
            if msg.role == "tool" && msg.content.len() > self.config.tool_output_truncate_at {
                prompt.push_str(&msg.content[..self.config.tool_output_truncate_at]);
                prompt.push_str("... [已截断]");
            } else {
                prompt.push_str(&msg.content);
            }
            prompt.push('\n');

            // 包含工具调用信息
            if let Some(ref tool_calls) = msg.tool_calls {
                for tc in tool_calls {
                    prompt.push_str(&format!("  → 调用工具: {}({})\n", tc.name, tc.arguments));
                }
            }
            prompt.push('\n');
        }

        prompt.push_str("\n## 输出格式\n\n");
        prompt.push_str("请按以下格式输出摘要：\n\n");
        prompt.push_str("### 任务概览\n[主要任务和目标]\n\n");
        prompt.push_str("### 关键决策\n[已做出的重要决定]\n\n");
        prompt.push_str("### 进行中的工作\n[当前正在处理的内容]\n\n");
        prompt.push_str("### 待处理问题\n[用户提出的尚未完全解决的问题]\n\n");
        prompt.push_str("### 剩余工作\n[还需要完成的任务]\n");

        prompt
    }

    /// 解析摘要响应
    pub fn parse_summary_response(&mut self, response: &str, messages_compressed: usize, tokens_before: usize) -> CompressedSummary {
        let summary = CompressedSummary {
            summary: response.to_string(),
            messages_compressed,
            tokens_before,
            tokens_after: response.len() / 3, // 粗略估算
            compressed_at: chrono::Utc::now().to_rfc3339(),
            historical_tasks: extract_section(response, "任务概览"),
            pending_questions: extract_section(response, "待处理问题"),
        };
        self.previous_summary = Some(summary.clone());
        summary
    }

    /// 构建压缩后的上下文
    pub fn build_compressed_context(
        &self,
        summary: &CompressedSummary,
        recent_messages: &[CompressibleMessage],
    ) -> Vec<CompressibleMessage> {
        let mut result = Vec::new();

        // 添加摘要作为系统消息
        let summary_text = format!(
            "{}\n\n{}\n\n{}\n\n{}",
            SUMMARY_PREFIX,
            summary.summary,
            SUMMARY_END_MARKER,
            format!("（已压缩 {} 条消息，从约 {} token 压缩至约 {} token）",
                summary.messages_compressed, summary.tokens_before, summary.tokens_after)
        );
        result.push(CompressibleMessage::system(&summary_text));

        // 添加最近的消息
        result.extend(recent_messages.iter().cloned());

        result
    }

    /// 预处理：截断旧的工具输出
    pub fn prune_tool_outputs(&self, messages: &mut Vec<CompressibleMessage>) {
        // 保留最近 N 条消息不动，截断更早的工具输出
        let preserve = self.config.preserve_recent_count;
        if messages.len() <= preserve {
            return;
        }
        let cutoff = messages.len() - preserve;
        for msg in &mut messages[..cutoff] {
            if msg.role == "tool" && msg.content.len() > self.config.tool_output_truncate_at {
                msg.content = PRUNED_TOOL_PLACEHOLDER.to_string();
            }
        }
    }

    /// 获取上一次压缩的摘要
    pub fn previous_summary(&self) -> Option<&CompressedSummary> {
        self.previous_summary.as_ref()
    }
}

// ── 辅助函数 ──────────────────────────────────────────────────

/// 从摘要文本中提取指定段落的内容
fn extract_section(text: &str, section_name: &str) -> Vec<String> {
    let mut results = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut in_section = false;

    for line in &lines {
        if line.contains(section_name) && line.starts_with('#') {
            in_section = true;
            continue;
        }
        if in_section {
            if line.starts_with('#') {
                break;
            }
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                results.push(trimmed.to_string());
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::infrastructure::llm_client::types::{ModelInfo, ToolSpec, StreamEvent};

    /// 测试用 Provider — complete 返回固定摘要文本，stream 未实现。
    struct MockProvider;

    #[async_trait]
    impl Provider for MockProvider {
        fn name(&self) -> &str { "mock" }
        fn models(&self) -> Vec<ModelInfo> { vec![] }
        fn api_key(&self) -> &str { "" }
        fn base_url(&self) -> &str { "" }
        async fn complete(&self, _: &str, _: &str, _: &[Message]) -> Result<String, AppError> {
            Ok("### 任务概览\n测试任务\n\n### 关键决策\n决策1\n".to_string())
        }
        async fn stream(
            &self, _: &str, _: &str, _: &[Message], _: &[ToolSpec],
        ) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = StreamEvent> + Send>>, AppError> {
            Err(AppError::not_implemented("mock does not stream"))
        }
        async fn test_connection(&self) -> Result<(), AppError> { Ok(()) }
    }

    #[test]
    fn test_should_compress() {
        let compressor = ContextCompressor::new(CompressorConfig::default());
        assert!( compressor.should_compress(160000, 200000));
        assert!(!compressor.should_compress(100000, 200000));
    }

    #[test]
    fn test_build_summarizer_prompt() {
        let compressor = ContextCompressor::new(CompressorConfig::default());
        let messages = vec![
            CompressibleMessage::user("帮我写第一章"),
            CompressibleMessage::assistant("好的，我来规划第一章的内容。"),
        ];
        let prompt = compressor.build_summarizer_prompt(&messages, None);
        assert!(prompt.contains("对话摘要专家"));
        assert!(prompt.contains("帮我写第一章"));
    }

    #[test]
    fn test_build_compressed_context() {
        let compressor = ContextCompressor::new(CompressorConfig::default());
        let summary = CompressedSummary {
            summary: "测试摘要".to_string(),
            messages_compressed: 5,
            tokens_before: 10000,
            tokens_after: 2000,
            compressed_at: "2026-01-01T00:00:00Z".to_string(),
            historical_tasks: vec![],
            pending_questions: vec![],
        };
        let recent = vec![CompressibleMessage::user("继续写")];
        let context = compressor.build_compressed_context(&summary, &recent);
        assert_eq!(context.len(), 2);
        assert_eq!(context[0].role, "system");
        assert!(context[0].content.contains("测试摘要"));
        assert_eq!(context[1].role, "user");
    }

    #[test]
    fn test_prune_tool_outputs() {
        let compressor = ContextCompressor::new(CompressorConfig::default());
        let mut messages = vec![
            CompressibleMessage::tool("short", "call_1"),
            CompressibleMessage::tool(&"x".repeat(5000), "call_2"),
            CompressibleMessage::tool(&"y".repeat(5000), "call_3"),
            CompressibleMessage::user("继续"),
        ];
        compressor.prune_tool_outputs(&mut messages);
        // 最近 10 条消息不截断，但这里只有 4 条，所以不截断
        assert_eq!(messages[1].content.len(), 5000);
    }

    #[test]
    fn test_extract_section() {
        let text = "### 任务概览\n写一部玄幻小说\n主角是李明\n\n### 关键决策\n使用第三人称视角\n";
        let tasks = extract_section(text, "任务概览");
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0], "写一部玄幻小说");
    }

    #[tokio::test]
    async fn test_compress_below_threshold_returns_original() {
        let mut compressor = ContextCompressor::new(CompressorConfig::default());
        let messages = vec![
            CompressibleMessage::user("短消息1"),
            CompressibleMessage::user("短消息2"),
        ];
        // 未超阈值 — 不调用 LLM，原样返回
        let result = compressor.compress(&messages, &MockProvider, "mock", 200_000).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_compress_too_few_messages_returns_original() {
        let mut compressor = ContextCompressor::new(CompressorConfig {
            preserve_recent_count: 10,
            ..Default::default()
        });
        // 超阈值但消息数 ≤ preserve_recent_count — 无法分割，原样返回
        let big = "x".repeat(200_000);
        let messages = vec![CompressibleMessage::user(&big)];
        let result = compressor.compress(&messages, &MockProvider, "mock", 100_000).await.unwrap();
        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn test_compress_invokes_llm_and_replaces_middle() {
        let mut compressor = ContextCompressor::new(CompressorConfig {
            preserve_recent_count: 2,
            ..Default::default()
        });
        // 5 条消息，每条足够大以触发压缩（estimate_tokens = len/3，1000 chars → ~333 tokens）
        // threshold_ratio=0.75，context_length=1000 → 阈值 750 tokens
        // 5 条 × 333 = 1665 tokens > 750，触发压缩
        let big = "x".repeat(1000);
        let messages: Vec<CompressibleMessage> = (0..5).map(|i| {
            CompressibleMessage::user(&format!("{}-{}", i, big))
        }).collect();
        let result = compressor.compress(&messages, &MockProvider, "mock", 1000).await.unwrap();
        // 期望：1 条摘要 system 消息 + 2 条最近消息 = 3 条
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].role, "system");
        assert!(result[0].content.contains("上下文压缩"));
        // 最近 2 条应为原消息 3、4
        assert!(result[1].content.starts_with("3-"));
        assert!(result[2].content.starts_with("4-"));
        // previous_summary 应已更新
        assert!(compressor.previous_summary().is_some());
    }
}
