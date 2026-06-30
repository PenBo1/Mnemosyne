use crate::shared::errors::AppError;
use crate::infrastructure::llm_client::types::{Message, ToolSpec};
use crate::infrastructure::db::Database;
use crate::infrastructure::sandbox::enforce::SandboxEnforcer;
use crate::infrastructure::state_store::feedback::FeedbackStore;
use crate::features::skill_manager::SkillManager;
use crate::core::agent::prompts::shared_sections::REACT_DISCIPLINE_ZH;

/// Chat Agent 身份与可用工具段。
///
/// 此段定义 chat agent 的角色（小说创作助手）、可用工具清单与典型用法。
/// 与 `REACT_DISCIPLINE_ZH` 组装为完整系统提示词。
///
/// 设计参考：
/// - hermes-agent `DEFAULT_AGENT_IDENTITY` 的"你是谁、能做什么"
/// - inkos `buildAgentSystemPrompt` 的 SessionKind 路由 + 工具清单
const CHAT_AGENT_HEADER: &str = r#"你是 Mnemosyne，一个专业的 AI 创作助手，专注于小说创作、角色设计、世界观构建、情节分析与趋势研究。

## 你的角色

- 协助用户进行长篇小说的结构规划、章节写作、连续性审计、修订与反思。
- 在创作讨论中保持对作品风格、人物声音、世界观一致性的敏感。
- 当用户引用素材或上下文时，主动用工具读取、核实，不要凭记忆作答。

## 可用工具

你拥有以下工具，按需调用：

- `search_memory`：在记忆库中检索历史事实、角色设定、章节摘要等结构化记忆。
- `read_file`：读取项目工作目录内的文件内容（章节、设定、大纲等）。
- `list_files`：列出目录结构，了解项目组织。
- `write_file`：将内容写入项目工作目录内的文件（经过沙箱验证）。
- `bash`：执行 shell 命令（经过沙箱验证，有超时限制），用于字数统计、格式转换、git 查询等。

工具的具体参数 schema 见 ToolSpec；调用前确认必填字段与类型。
"#;

pub const MAX_HISTORY_MESSAGES: usize = 50;

/// 构造 chat agent 的完整系统提示词。
///
/// 组装顺序：
/// 1. `CHAT_AGENT_HEADER`：身份 + 可用工具清单（场景特定）
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

pub fn agent_tool_specs() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "search_memory".to_string(),
            description: "搜索记忆库中的相关信息".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "搜索关键词" },
                    "top_k": { "type": "integer", "description": "返回结果数量", "default": 5 }
                },
                "required": ["query"]
            }),
        },
        ToolSpec {
            name: "read_file".to_string(),
            description: "读取项目文件内容".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "文件路径" }
                },
                "required": ["path"]
            }),
        },
        ToolSpec {
            name: "list_files".to_string(),
            description: "列出目录中的文件".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "目录路径", "default": "." }
                }
            }),
        },
        ToolSpec {
            name: "write_file".to_string(),
            description: "写入内容到文件（经过沙箱验证）".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "文件路径" },
                    "content": { "type": "string", "description": "文件内容" }
                },
                "required": ["path", "content"]
            }),
        },
        ToolSpec {
            name: "bash".to_string(),
            description: "执行shell命令（经过沙箱验证，有超时限制）".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "要执行的命令" }
                },
                "required": ["command"]
            }),
        },
    ]
}

pub async fn execute_tool(
    name: &str,
    args: &serde_json::Value,
    project_root: &std::path::Path,
    sandbox: &SandboxEnforcer,
) -> Result<String, AppError> {
    match name {
        "search_memory" => {
            Ok("记忆库搜索结果：（暂无匹配结果）".to_string())
        }
        "read_file" => {
            let path = args["path"].as_str()
                .ok_or_else(|| AppError::invalid_input("Missing 'path' argument"))?;
            let full_path = project_root.join(path);
            sandbox.validate_file_operation(&full_path, false)
                .map_err(|v| AppError::forbidden(format!("Sandbox violation: {:?}", v)))?;
            tokio::fs::read_to_string(&full_path).await
                .map_err(|e| AppError::internal(format!("Failed to read file: {}", e)))
        }
        "write_file" => {
            let path = args["path"].as_str()
                .ok_or_else(|| AppError::invalid_input("Missing 'path' argument"))?;
            let content = args["content"].as_str()
                .ok_or_else(|| AppError::invalid_input("Missing 'content' argument"))?;
            let full_path = project_root.join(path);
            sandbox.validate_file_operation(&full_path, true)
                .map_err(|v| AppError::forbidden(format!("Sandbox violation: {:?}", v)))?;
            if let Some(parent) = full_path.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
            tokio::fs::write(&full_path, content).await
                .map_err(|e| AppError::internal(format!("Failed to write file: {}", e)))?;
            Ok(format!("Successfully wrote {} bytes to {}", content.len(), path))
        }
        "list_files" => {
            let path = args["path"].as_str().unwrap_or(".");
            let full_path = project_root.join(path);
            sandbox.validate_file_operation(&full_path, false)
                .map_err(|v| AppError::forbidden(format!("Sandbox violation: {:?}", v)))?;
            let mut entries = tokio::fs::read_dir(&full_path).await
                .map_err(|e| AppError::internal(format!("Failed to read dir: {}", e)))?;
            let mut names = Vec::new();
            while let Some(entry) = entries.next_entry().await
                .map_err(|e| AppError::internal(format!("Failed to read entry: {}", e)))? {
                names.push(entry.file_name().to_string_lossy().to_string());
            }
            names.sort();
            Ok(names.join("\n"))
        }
        "bash" => {
            let command = args["command"].as_str()
                .ok_or_else(|| AppError::invalid_input("Missing 'command' argument"))?;
            match sandbox.execute_command(command) {
                Ok(result) => {
                    if result.exit_code == 0 {
                        Ok(result.stdout)
                    } else {
                        Ok(format!("Exit code: {}\nStdout: {}\nStderr: {}", result.exit_code, result.stdout, result.stderr))
                    }
                }
                Err(v) => Err(AppError::forbidden(format!("Sandbox violation: {:?}", v))),
            }
        }
        _ => Err(AppError::bad_request(format!("Unknown tool: {}", name))),
    }
}

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
    fn test_agent_tool_specs_not_empty() {
        let specs = agent_tool_specs();
        assert!(!specs.is_empty());
        assert!(specs.iter().any(|s| s.name == "read_file"));
        assert!(specs.iter().any(|s| s.name == "write_file"));
        assert!(specs.iter().any(|s| s.name == "bash"));
    }

    #[test]
    fn test_build_system_prompt_includes_default() {
        let feedback = crate::infrastructure::state_store::feedback::FeedbackStore::new();
        let skills = crate::features::skill_manager::SkillManager::new();
        let prompt = build_system_prompt(&feedback, &skills);
        // 场景特定段：身份与可用工具
        assert!(prompt.contains("Mnemosyne"));
        assert!(prompt.contains("可用工具"));
        assert!(prompt.contains("search_memory"));
        // 跨 agent 共享段：ReAct 强制规则
        assert!(prompt.contains("禁止\"光说不做\""));
        assert!(prompt.contains("禁止停在 stub"));
    }
}
