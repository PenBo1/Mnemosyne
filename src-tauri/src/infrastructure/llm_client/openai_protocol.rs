//! OpenAI 兼容协议共享工具 — 供 agnes.rs 和 openai.rs 复用。
//!
//! 这两个 provider 都使用 OpenAI 的 `/chat/completions` 协议，
//! 请求体构造逻辑完全相同，提取至此避免复制粘贴。

use super::types::{Message, ToolSpec};

/// 构造工具负载（OpenAI function calling 格式）
pub fn build_tools_payload(tools: &[ToolSpec]) -> Vec<serde_json::Value> {
    tools.iter().map(|t| {
        serde_json::json!({
            "type": "function",
            "function": { "name": t.name, "description": t.description, "parameters": t.parameters }
        })
    }).collect()
}

/// 构造 `/chat/completions` 请求体
pub fn build_request(
    model: &str,
    system: &str,
    messages: &[Message],
    tools: &[ToolSpec],
    stream: bool,
) -> serde_json::Value {
    let mut msgs = vec![serde_json::json!({ "role": "system", "content": system })];
    for m in messages {
        let mut entry = serde_json::json!({ "role": m.role, "content": m.content });
        if let Some(tc) = &m.tool_calls {
            entry["tool_calls"] = serde_json::to_value(tc).unwrap();
        }
        if let Some(tcid) = &m.tool_call_id {
            entry["tool_call_id"] = serde_json::Value::String(tcid.clone());
        }
        msgs.push(entry);
    }
    let mut body = serde_json::json!({ "model": model, "messages": msgs, "stream": stream });
    if !tools.is_empty() {
        body["tools"] = serde_json::json!(build_tools_payload(tools));
    }
    body
}
