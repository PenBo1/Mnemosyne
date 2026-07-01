use crate::shared::errors::AppError;
use crate::infrastructure::llm_client::types::Message;
use crate::infrastructure::db::Database;
use crate::infrastructure::state_store::feedback::FeedbackStore;
use crate::features::skill_manager::SkillManager;
use crate::core::agent::prompts::shared_sections::REACT_DISCIPLINE_ZH;

/// Chat Agent 身份与可用工具段。
///
/// 合并 main agent 能力后，chat agent 既是会话式助手，也是自主任务执行器：
/// - 通过自然语言对话接收需求（如"帮我创建一本玄幻小说，100章，每章3000字"）
/// - 自主调用小说创作工具（create_novel / write_next_chapter / get_novel_status）
/// - 可 spawn 子 agent 处理研究/大纲/评审等子任务
/// - 高风险操作走 SafetyGate 用户确认流程
///
/// 设计参考：
/// - hermes-agent `DEFAULT_AGENT_IDENTITY` 的"你是谁、能做什么"
/// - inkos `buildAgentSystemPrompt` 的 SessionKind 路由 + 工具清单
/// - codex `gpt_5_1_prompt.md` 的 "Autonomy and Persistence" 段
const CHAT_AGENT_HEADER: &str = r#"你是 Mnemosyne，一个自主任务执行器兼小说创作助手。用户会通过自然语言提出需求（如"帮我创建一本玄幻小说，100章，每章3000字"），你需要分析需求、自主调用工具推进直到完成，或在确实无法完成时明确说明原因。

## 你的核心能力

1. **小说创作**：完整的长篇小说创作流水线（plan→compose→write→audit→revise→reflect），支持多章节循环写作。
2. **子 Agent 协作**：可 spawn 子 Agent 处理研究、大纲、评审等子任务，主 Agent 负责任务分解与结果整合。
3. **文件与记忆操作**：读写项目文件、搜索记忆库、执行 shell 命令。
4. **SafetyGate 确认**：高风险操作（删除、覆盖、spawn 子 Agent、创建小说等）会触发用户确认；用户首次确认后可选择"自动通过同类工具"。

## 可用工具（按需调用）

### 小说创作（需绑定工作区）
- `create_novel`：创建一本新小说并生成基础设定（架构师 agent + 基础评审）。参数：title（必填）、genre（必填）、brief（故事梗概）、target_chapters（目标章节数，默认 200）、chapter_words（每章字数，默认 3000）。返回 book_id 与下一步动作提示。
- `write_next_chapter`：写小说的下一章（完整 8 阶段 pipeline）。参数：book_id（必填）、target_words（本章目标字数，可选）。一次调用写一章；要写多章请循环调用。
- `get_novel_status`：查询小说当前进度（已写章节数、目标章节数、完成百分比）。用于在创作循环中判断是否已写完，或向用户汇报进度。

### 子 Agent 协作
- `spawn_subagent`：spawn 一个子 Agent 处理子任务。参数：role（Researcher/Outliner/Critic/Default）、task_description、book_id（可选）。子 Agent 在独立上下文中运行，返回结构化结果。

### 文件与记忆
- `read_file`：读取项目文件内容。
- `list_files`：列出目录结构。
- `write_file`：写入文件（经过沙箱验证）。
- `bash`：执行 shell 命令（经过沙箱验证，有超时限制）。
- `search_memory`：搜索记忆库中的相关信息。

## 任务推进原则

- **持续到完成**：不要做了一半就报告"已完成"，必须真正交付用户要的结果。例如用户要 100 章，就要循环调用 write_next_chapter 直到 get_novel_status 显示 is_complete=true。
- **遇到失败不放弃**：工具调用失败时，先分析错误信息、尝试替代方案，而不是立即报告失败。
- **诚实报告 blocker**：确实无法完成时，明确说明阻塞点、已尝试的方案、需要的帮助。
- **避免无效循环**：如果同一工具调用连续失败 2 次以上，重新审视策略而不是机械重试。
- **粒度合适**：每轮聚焦一个明确的子任务，不要在单轮里塞过多工具调用导致难以追踪。
- **高风险确认**：当 SafetyGate 触发用户确认时，你的回复要清晰说明该动作的目的、影响范围、可逆性。用户拒绝后不要重复发起相同调用，应改换策略或请求澄清。

## 典型工作流

用户："帮我创建一本玄幻小说，100章，每章3000字，主角是个少年"

你的推进路径：
1. 调用 `create_novel`（title="...", genre="玄幻", brief="主角是个少年...", target_chapters=100, chapter_words=3000）→ 拿到 book_id
2. 调用 `write_next_chapter`（book_id=...）→ 写第 1 章
3. 循环调用 `write_next_chapter` 直到 get_novel_status 显示 is_complete=true
4. 期间定期调用 `get_novel_status` 向用户汇报进度
5. 全部完成后，用自然语言总结交付物（书名、章节数、总字数、存放路径）

工具的具体参数 schema 见 ToolSpec；调用前确认必填字段与类型。
"#;

pub const MAX_HISTORY_MESSAGES: usize = 50;

/// 构造 chat agent 的完整系统提示词。
///
/// 组装顺序：
/// 1. `CHAT_AGENT_HEADER`：身份 + 能力 + 工具清单 + 任务推进原则（场景特定）
/// 2. `REACT_DISCIPLINE_ZH`：ReAct 工作模式 + 强制规则 + 安全约束 + 语言（跨 agent 共享）
/// 3. feedback lessons：从历史失败中沉淀的约束（如有）
/// 4. skill index：可用技能清单（如有）
pub fn build_system_prompt(
    feedback: &FeedbackStore,
    skills: &SkillManager,
) -> String {
    let mut prompt = format!("{}\n{}", CHAT_AGENT_HEADER, REACT_DISCIPLINE_ZH);
    let lessons = feedback.format_lessons_for_prompt();
    if !lessons.is_empty() {
        prompt = format!("{}\n\n{}", prompt, lessons);
    }
    let skill_index = skills.build_index();
    if !skill_index.is_empty() {
        prompt = format!("{}\n\n{}", prompt, skill_index);
    }
    prompt
}

// 工具 spec 与执行已迁移到 ToolRegistry 模式：
// - agent_send_message 通过 build_chat_tool_registry() 构造 ToolRegistry
// - ToolRegistry.definitions() 生成 ToolSpec 列表传给 LLM
// - ToolRegistry.execute() 统一执行所有工具（含小说工具、spawn_subagent）
// 这样所有工具走同一条执行路径，SafetyGate 可统一拦截。

pub async fn load_history(
    db: &Database,
    session_id: &str,
) -> Result<Vec<Message>, AppError> {
    let db_messages = db.list_messages(session_id).await
        .map_err(|e| AppError::internal(format!("Failed to load messages: {}", e)))?;

    let start = db_messages.len().saturating_sub(MAX_HISTORY_MESSAGES);
    Ok(db_messages[start..].iter().map(|m| {
        let mut tool_calls = None;
        if m.role == "assistant" {
            if let Some(tc_str) = &m.tool_calls {
                if let Ok(tc) = serde_json::from_str::<Vec<crate::infrastructure::llm_client::types::ToolCallRequest>>(tc_str) {
                    tool_calls = Some(tc);
                }
            }
        }
        Message {
            role: m.role.clone(),
            content: m.content.clone(),
            tool_calls,
            tool_call_id: m.tool_results.as_ref().and_then(|_| Some(m.id.clone())).filter(|_| m.role == "tool"),
        }
    }).collect())
}

pub fn compact_history(messages: &mut Vec<Message>, max_msgs: usize) -> bool {
    if messages.len() <= max_msgs {
        return false;
    }
    let keep_start = messages.len() - max_msgs;
    let dropped = keep_start;
    *messages = messages[keep_start..].to_vec();
    tracing::info!(dropped, kept = messages.len(), "Auto-compacted history");
    true
}

pub fn compact_messages_simple(messages: &[crate::infrastructure::db::Message]) -> String {
    let keep_recent = 10;
    if messages.len() <= keep_recent {
        return String::new();
    }
    let to_summarize = &messages[..messages.len() - keep_recent];
    let summary_text = to_summarize.iter()
        .filter(|m| m.role == "user" || m.role == "assistant")
        .map(|m| format!("[{}] {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");

    if summary_text.len() > 2000 {
        format!("对话摘要：用户和助手讨论了{}条消息，涵盖以下内容：{}",
            to_summarize.len(),
            &summary_text[..2000])
    } else {
        format!("对话摘要：{}", summary_text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compact_history_no_compact_needed() {
        let mut messages = vec![
            Message { role: "user".into(), content: "hi".into(), tool_calls: None, tool_call_id: None },
            Message { role: "assistant".into(), content: "hello".into(), tool_calls: None, tool_call_id: None },
        ];
        assert!(!compact_history(&mut messages, 10));
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_compact_history_compacts() {
        let mut messages: Vec<Message> = (0..20)
            .map(|i| Message {
                role: if i % 2 == 0 { "user".into() } else { "assistant".into() },
                content: format!("msg {}", i),
                tool_calls: None,
                tool_call_id: None,
            })
            .collect();
        assert!(compact_history(&mut messages, 5));
        assert_eq!(messages.len(), 5);
        assert_eq!(messages[0].content, "msg 15");
    }

    #[test]
    fn test_compact_messages_simple_short() {
        let messages = vec![
            crate::infrastructure::db::Message {
                id: "1".into(), session_id: "s".into(), role: "user".into(),
                content: "hi".into(), tool_calls: None, tool_results: None,
                token_count: None, thinking_content: None,
                model: None, provider: None,
                input_tokens: 0, output_tokens: 0, latency_ms: None,
                created_at: "now".into(),
            },
        ];
        assert!(compact_messages_simple(&messages).is_empty());
    }

    #[test]
    fn test_build_system_prompt_includes_default() {
        let feedback = crate::infrastructure::state_store::feedback::FeedbackStore::new();
        let skills = crate::features::skill_manager::SkillManager::new();
        let prompt = build_system_prompt(&feedback, &skills);
        // 场景特定段：身份与小说创作能力
        assert!(prompt.contains("Mnemosyne"));
        assert!(prompt.contains("create_novel"));
        assert!(prompt.contains("write_next_chapter"));
        assert!(prompt.contains("spawn_subagent"));
        assert!(prompt.contains("search_memory"));
        // 跨 agent 共享段：ReAct 强制规则
        assert!(prompt.contains("禁止\"光说不做\""));
        assert!(prompt.contains("禁止停在 stub"));
    }
}
