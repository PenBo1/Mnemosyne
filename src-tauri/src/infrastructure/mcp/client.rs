// MCP stdio 客户端 —— JSON-RPC 2.0 over newline-delimited stdio。
//
// 最小可行实现：
// - 仅支持 stdio 传输（HTTP/SSE 返回 NOT_IMPLEMENTED）
// - 同步请求-响应（一次只处理一个请求，按 id 匹配）
// - 支持 initialize / list_tools / call_tool 三个核心方法
// - 使用 tokio::process::Command 启动子进程
//
// 不实现：notifications 主动推送、resources/prompts/sampling、
// 连接重连、请求超时（由调用方控制）。

use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};

use crate::infrastructure::mcp::types::{McpContent, McpTool, McpToolCallResult, McpTransport};
use crate::shared::error::AppError;

/// MCP 协议版本（2024-11-05）
const PROTOCOL_VERSION: &str = "2024-11-05";

/// MCP stdio 客户端
pub struct McpClient {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    child: Child,
    next_id: u64,
}

impl McpClient {
    /// 连接到 MCP server。
    ///
    /// Stdio 传输：启动子进程，获取 stdin/stdout 句柄。
    /// HTTP/SSE 传输：返回 NOT_IMPLEMENTED。
    pub async fn connect(transport: &McpTransport) -> Result<Self, AppError> {
        match transport {
            McpTransport::Stdio { command, args, env } => {
                let mut cmd = tokio::process::Command::new(command);
                cmd.args(args);
                for (k, v) in env {
                    cmd.env(k, v);
                }
                cmd.stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .kill_on_drop(true);

                let mut child = cmd.spawn().map_err(|e| {
                    AppError::internal(format!(
                        "Failed to spawn MCP server process '{}': {}",
                        command, e
                    ))
                })?;

                let stdin = child.stdin.take().ok_or_else(|| {
                    AppError::internal("MCP server child stdin unavailable")
                })?;
                let stdout = child.stdout.take().ok_or_else(|| {
                    AppError::internal("MCP server child stdout unavailable")
                })?;

                Ok(Self {
                    stdin,
                    stdout: BufReader::new(stdout),
                    child,
                    next_id: 1,
                })
            }
            McpTransport::Http { .. } | McpTransport::Sse { .. } => {
                Err(AppError::not_implemented(
                    "HTTP/SSE MCP transport not yet supported",
                ))
            }
        }
    }

    /// 发送 JSON-RPC 请求并等待响应。
    ///
    /// 读取 stdout 直到匹配的 id 出现，跳过 notification（无 id 的消息）。
    async fn send_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, AppError> {
        let id = self.next_id;
        self.next_id += 1;

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params.unwrap_or(serde_json::Value::Null),
        });
        let line = serde_json::to_string(&request)? + "\n";
        self.stdin.write_all(line.as_bytes()).await.map_err(|e| {
            AppError::internal(format!("Failed to write to MCP server stdin: {}", e))
        })?;
        self.stdin.flush().await.map_err(|e| {
            AppError::internal(format!("Failed to flush MCP server stdin: {}", e))
        })?;

        // 读取响应行，跳过 notification（无 id）和不匹配的响应
        loop {
            let mut buf = Vec::new();
            let n = self
                .stdout
                .read_until(b'\n', &mut buf)
                .await
                .map_err(|e| {
                    AppError::internal(format!("Failed to read MCP server stdout: {}", e))
                })?;
            if n == 0 {
                return Err(AppError::internal(
                    "MCP server closed stdout before responding",
                ));
            }
            // 跳过空行
            if buf.iter().all(|&b| b == b'\n' || b == b'\r') {
                continue;
            }

            let msg: serde_json::Value = serde_json::from_slice(&buf).map_err(|e| {
                AppError::internal(format!("Failed to parse MCP response: {}", e))
            })?;

            // 跳过 notification（无 id 字段）
            if msg.get("id").is_none() {
                continue;
            }

            let resp_id = msg.get("id");
            if resp_id != Some(&serde_json::Value::from(id)) && resp_id != Some(&serde_json::Value::from(id as i64)) {
                // 不匹配的响应，跳过
                continue;
            }

            if let Some(error) = msg.get("error") {
                return Err(AppError::internal(format!(
                    "MCP server returned error: {}",
                    serde_json::to_string(error).unwrap_or_else(|_| "unknown".into())
                )));
            }

            return msg
                .get("result")
                .cloned()
                .ok_or_else(|| AppError::internal("MCP response missing 'result' field"));
        }
    }

    /// 发送 notification（无 id，不等待响应）。
    async fn send_notification(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<(), AppError> {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        let line = serde_json::to_string(&notification)? + "\n";
        self.stdin.write_all(line.as_bytes()).await.map_err(|e| {
            AppError::internal(format!("Failed to write notification to MCP stdin: {}", e))
        })?;
        self.stdin.flush().await.map_err(|e| {
            AppError::internal(format!("Failed to flush MCP stdin: {}", e))
        })?;
        Ok(())
    }

    /// initialize 握手。
    ///
    /// 返回 server 的 InitializeResult（serverInfo / capabilities / protocolVersion）。
    pub async fn initialize(&mut self) -> Result<serde_json::Value, AppError> {
        let result = self
            .send_request(
                "initialize",
                Some(serde_json::json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": {},
                    "clientInfo": {
                        "name": "mnemosyne",
                        "version": env!("CARGO_PKG_VERSION"),
                    },
                })),
            )
            .await?;

        // 发送 initialized notification（MCP 协议要求）
        self.send_notification("notifications/initialized", serde_json::Value::Object(Default::default()))
            .await?;

        Ok(result)
    }

    /// 列出 server 暴露的工具。
    pub async fn list_tools(&mut self) -> Result<Vec<McpTool>, AppError> {
        let result = self.send_request("tools/list", None).await?;
        let tools_value = result.get("tools").cloned().unwrap_or(serde_json::Value::Array(vec![]));

        let raw_tools: Vec<RawMcpTool> = serde_json::from_value(tools_value).map_err(|e| {
            AppError::internal(format!("Failed to parse MCP tools/list response: {}", e))
        })?;

        Ok(raw_tools
            .into_iter()
            .map(|t| McpTool {
                name: t.name,
                description: t.description.unwrap_or_default(),
                input_schema: t.input_schema.unwrap_or(serde_json::Value::Object(Default::default())),
                server_id: String::new(), // 由 registry 注入
            })
            .collect())
    }

    /// 调用工具。
    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolCallResult, AppError> {
        let result = self
            .send_request(
                "tools/call",
                Some(serde_json::json!({
                    "name": name,
                    "arguments": arguments,
                })),
            )
            .await?;

        parse_tool_call_result(&result)
    }

    /// 关闭连接（kill 子进程）。
    pub fn shutdown(&mut self) {
        let _ = self.child.start_kill();
    }

    /// 检查子进程是否仍在运行。
    pub fn is_alive(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(None) => true,
            Ok(Some(_)) => false,
            Err(_) => false,
        }
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// 解析 tools/call 响应为 McpToolCallResult。
fn parse_tool_call_result(result: &serde_json::Value) -> Result<McpToolCallResult, AppError> {
    let is_error = result
        .get("isError")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let content_value = result
        .get("content")
        .cloned()
        .unwrap_or(serde_json::Value::Array(vec![]));

    let raw_contents: Vec<RawMcpContent> = serde_json::from_value(content_value).map_err(|e| {
        AppError::internal(format!("Failed to parse MCP tool call content: {}", e))
    })?;

    let content = raw_contents
        .into_iter()
        .map(|c| match c.kind.as_str() {
            "image" => McpContent::Image {
                data: c.data.unwrap_or_default(),
                mime_type: c.mime_type.unwrap_or_default(),
            },
            _ => McpContent::Text {
                text: c.text.unwrap_or_default(),
            },
        })
        .collect();

    Ok(McpToolCallResult { content, is_error })
}

// ── 反序列化辅助类型（适配 MCP 协议的 camelCase 字段） ────────

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMcpTool {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    input_schema: Option<serde_json::Value>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMcpContent {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    data: Option<String>,
    #[serde(default)]
    mime_type: Option<String>,
}

// ── 测试 ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_text_content() {
        let result = serde_json::json!({
            "content": [
                { "type": "text", "text": "hello" }
            ],
            "isError": false
        });
        let parsed = parse_tool_call_result(&result).unwrap();
        assert!(!parsed.is_error);
        assert_eq!(parsed.content.len(), 1);
        match &parsed.content[0] {
            McpContent::Text { text } => assert_eq!(text, "hello"),
            _ => panic!("expected Text variant"),
        }
    }

    #[test]
    fn parse_image_content() {
        let result = serde_json::json!({
            "content": [
                { "type": "image", "data": "base64...", "mimeType": "image/png" }
            ],
            "isError": true
        });
        let parsed = parse_tool_call_result(&result).unwrap();
        assert!(parsed.is_error);
        match &parsed.content[0] {
            McpContent::Image { data, mime_type } => {
                assert_eq!(data, "base64...");
                assert_eq!(mime_type, "image/png");
            }
            _ => panic!("expected Image variant"),
        }
    }

    #[test]
    fn parse_empty_content() {
        let result = serde_json::json!({});
        let parsed = parse_tool_call_result(&result).unwrap();
        assert!(!parsed.is_error);
        assert!(parsed.content.is_empty());
    }

    #[test]
    fn parse_multiple_content_blocks() {
        let result = serde_json::json!({
            "content": [
                { "type": "text", "text": "a" },
                { "type": "text", "text": "b" },
                { "type": "image", "data": "d", "mimeType": "image/jpeg" }
            ]
        });
        let parsed = parse_tool_call_result(&result).unwrap();
        assert_eq!(parsed.content.len(), 3);
    }
}
