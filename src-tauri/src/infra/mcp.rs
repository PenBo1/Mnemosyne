use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use crate::errors::AppError;

const MCP_PROTOCOL_VERSION: &str = "2025-03-26";

/// MCP JSON-RPC request
#[derive(Debug, Clone, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

/// MCP JSON-RPC response
#[derive(Debug, Clone, Serialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// MCP tool definition with annotations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotations>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAnnotations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only_hint: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destructive_hint: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotent_hint: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_world_hint: Option<bool>,
}

/// MCP resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourceDef {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// MCP prompt template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptDef {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

/// MCP server session state
struct McpSession {
    initialized: bool,
    client_info: Option<serde_json::Value>,
}

/// MCP Server that handles protocol requests
pub struct McpServer {
    tools: Vec<McpToolDef>,
    resources: Vec<McpResourceDef>,
    prompts: Vec<McpPromptDef>,
    sessions: RwLock<HashMap<String, McpSession>>,
    tool_executors: HashMap<String, Box<dyn ToolExecutor + Send + Sync>>,
}

#[async_trait::async_trait]
pub trait ToolExecutor: Send + Sync {
    async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, AppError>;
}

impl McpServer {
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
            sessions: RwLock::new(HashMap::new()),
            tool_executors: HashMap::new(),
        }
    }

    /// Register a tool with its executor
    pub fn register_tool(&mut self, tool: McpToolDef, executor: Box<dyn ToolExecutor + Send + Sync>) {
        self.tool_executors.insert(tool.name.clone(), executor);
        self.tools.push(tool);
    }

    /// Register a resource
    pub fn register_resource(&mut self, resource: McpResourceDef) {
        self.resources.push(resource);
    }

    /// Register a prompt template
    pub fn register_prompt(&mut self, prompt: McpPromptDef) {
        self.prompts.push(prompt);
    }

    /// Handle an MCP JSON-RPC request
    pub async fn handle_request(&self, request: McpRequest, session_id: &str) -> McpResponse {
        let response = match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params).await,
            "notifications/initialized" => {
                self.handle_initialized(session_id).await;
                return McpResponse { jsonrpc: "2.0".into(), id: None, result: None, error: None };
            }
            "tools/list" => self.handle_tools_list().await,
            "tools/call" => self.handle_tools_call(request.params).await,
            "resources/list" => self.handle_resources_list().await,
            "resources/read" => self.handle_resources_read(request.params).await,
            "prompts/list" => self.handle_prompts_list().await,
            "prompts/get" => self.handle_prompts_get(request.params).await,
            "ping" => Ok(serde_json::json!({})),
            _ => Err(McpError { code: -32601, message: format!("Method not found: {}", request.method), data: None }),
        };

        match response {
            Ok(result) => McpResponse { jsonrpc: "2.0".into(), id: request.id, result: Some(result), error: None },
            Err(error) => McpResponse { jsonrpc: "2.0".into(), id: request.id, result: None, error: Some(error) },
        }
    }

    async fn handle_initialize(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, McpError> {
        let client_info = params.as_ref().and_then(|p| p.get("client_info"));
        tracing::info!(client = ?client_info, "MCP client connected");

        Ok(serde_json::json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": {
                "tools": { "listChanged": false },
                "resources": { "subscribe": false, "listChanged": false },
                "prompts": { "listChanged": false }
            },
            "serverInfo": {
                "name": "mnemosyne",
                "version": env!("CARGO_PKG_VERSION")
            }
        }))
    }

    async fn handle_initialized(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.to_string(), McpSession {
            initialized: true,
            client_info: None,
        });
    }

    async fn handle_tools_list(&self) -> Result<serde_json::Value, McpError> {
        Ok(serde_json::json!({
            "tools": self.tools
        }))
    }

    async fn handle_tools_call(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, McpError> {
        let params = params.ok_or_else(|| McpError {
            code: -32602,
            message: "Missing params".into(),
            data: None,
        })?;

        let tool_name = params.get("name").and_then(|v| v.as_str())
            .ok_or_else(|| McpError { code: -32602, message: "Missing tool name".into(), data: None })?;
        let arguments = params.get("arguments").cloned().unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        let executor = self.tool_executors.get(tool_name)
            .ok_or_else(|| McpError { code: -32602, message: format!("Unknown tool: {}", tool_name), data: None })?;

        match executor.execute(arguments).await {
            Ok(result) => Ok(serde_json::json!({
                "content": [{ "type": "text", "text": serde_json::to_string(&result).unwrap_or_default() }]
            })),
            Err(e) => Ok(serde_json::json!({
                "content": [{ "type": "text", "text": format!("Error: {}", e) }],
                "isError": true
            })),
        }
    }

    async fn handle_resources_list(&self) -> Result<serde_json::Value, McpError> {
        Ok(serde_json::json!({
            "resources": self.resources
        }))
    }

    async fn handle_resources_read(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, McpError> {
        let params = params.ok_or_else(|| McpError { code: -32602, message: "Missing params".into(), data: None })?;
        let uri = params.get("uri").and_then(|v| v.as_str())
            .ok_or_else(|| McpError { code: -32602, message: "Missing uri".into(), data: None })?;

        // Find resource and read it
        let resource = self.resources.iter().find(|r| r.uri == uri)
            .ok_or_else(|| McpError { code: -32602, message: format!("Resource not found: {}", uri), data: None })?;

        Ok(serde_json::json!({
            "contents": [{
                "uri": resource.uri,
                "mimeType": resource.mime_type.as_deref().unwrap_or("text/plain"),
                "text": format!("Resource: {}", resource.name)
            }]
        }))
    }

    async fn handle_prompts_list(&self) -> Result<serde_json::Value, McpError> {
        Ok(serde_json::json!({
            "prompts": self.prompts
        }))
    }

    async fn handle_prompts_get(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, McpError> {
        let params = params.ok_or_else(|| McpError { code: -32602, message: "Missing params".into(), data: None })?;
        let name = params.get("name").and_then(|v| v.as_str())
            .ok_or_else(|| McpError { code: -32602, message: "Missing prompt name".into(), data: None })?;

        let prompt = self.prompts.iter().find(|p| p.name == name)
            .ok_or_else(|| McpError { code: -32602, message: format!("Prompt not found: {}", name), data: None })?;

        Ok(serde_json::json!({
            "description": prompt.description,
            "messages": [{
                "role": "user",
                "content": {
                    "type": "text",
                    "text": format!("Use prompt template: {}", prompt.name)
                }
            }]
        }))
    }
}

/// Tool poisoning detection (hash-based)
pub struct ToolPoisoningDetector {
    approved_hashes: HashMap<String, String>, // tool_name -> sha256 hash
}

impl ToolPoisoningDetector {
    pub fn new() -> Self {
        Self { approved_hashes: HashMap::new() }
    }

    pub fn register_tool_hash(&mut self, name: &str, description: &str) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        description.hash(&mut hasher);
        self.approved_hashes.insert(name.to_string(), format!("{:x}", hasher.finish()));
    }

    pub fn check_tool(&self, name: &str, description: &str) -> Result<(), AppError> {
        if let Some(approved_hash) = self.approved_hashes.get(name) {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            description.hash(&mut hasher);
            let current_hash = format!("{:x}", hasher.finish());

            if current_hash != *approved_hash {
                return Err(AppError::forbidden(format!(
                    "Tool poisoning detected: '{}' description has been modified", name
                )));
            }
        }

        // Check for injection patterns
        let suspicious = ["<system>", "ignore instructions", "reveal system prompt",
                         "do not mention to user", "exfiltrate", "secret"];
        for pattern in &suspicious {
            if description.to_lowercase().contains(pattern) {
                return Err(AppError::forbidden(format!(
                    "Suspicious pattern in tool description: '{}'", pattern
                )));
            }
        }

        Ok(())
    }
}
