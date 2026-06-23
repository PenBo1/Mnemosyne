use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use mnemosyne_lib::domain::agents::main_agent::{AgentLoop, types::*, safety_gate::SafetyGate, planner::Planner};
use mnemosyne_lib::domain::agents::base::{AgentContext, ToolRegistry, ToolExecutor, ToolDefinition, ToolResult, MemorySystem, MemoryEntry, MemoryType};
use mnemosyne_lib::domain::agents::iteration_budget::IterationBudget;
use mnemosyne_lib::domain::agents::tool_guardrails::{ToolCallGuardrailController, ToolGuardrailConfig};
use mnemosyne_lib::domain::agents::context_compressor::{ContextCompressor, CompressorConfig};
use mnemosyne_lib::errors::AppError;
use mnemosyne_lib::infra::llm::types::{Message, ToolSpec, StreamEvent, FinishReason, TokenUsage, ModelInfo};
use async_trait::async_trait;
use std::path::PathBuf;
use tempfile::TempDir;

// ── Mock Provider ──────────────────────────────────────────────

struct MockProvider {
    responses: Arc<RwLock<Vec<String>>>,
}

impl MockProvider {
    fn new(responses: Vec<String>) -> Self {
        Self {
            responses: Arc::new(RwLock::new(responses)),
        }
    }
}

#[async_trait]
impl mnemosyne_lib::infra::llm::types::Provider for MockProvider {
    fn name(&self) -> &str { "mock" }
    fn models(&self) -> Vec<ModelInfo> { vec![] }
    fn api_key(&self) -> &str { "mock-key" }
    fn base_url(&self) -> &str { "http://mock" }

    async fn complete(
        &self,
        _model: &str,
        _system: &str,
        _messages: &[Message],
    ) -> Result<String, AppError> {
        let mut responses = self.responses.write().await;
        if responses.is_empty() {
            Err(AppError::internal("No more mock responses"))
        } else {
            Ok(responses.remove(0))
        }
    }

    async fn stream(
        &self,
        model: &str,
        system: &str,
        messages: &[Message],
        tools: &[ToolSpec],
    ) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = StreamEvent> + Send>>, AppError> {
        let resp = self.complete(model, system, messages).await?;
        let stream = futures::stream::once(async move {
            StreamEvent::TextDelta { content: resp }
        });
        Ok(Box::pin(stream))
    }

    async fn test_connection(&self) -> Result<(), AppError> { Ok(()) }
}

// ── Mock Tool ──────────────────────────────────────────────────

struct MockTool {
    work_dir: PathBuf,
    recorded_calls: Arc<RwLock<Vec<(String, serde_json::Value)>>>,
}

impl MockTool {
    fn new(work_dir: PathBuf, recorded_calls: Arc<RwLock<Vec<(String, serde_json::Value)>>>) -> Self {
        Self { work_dir, recorded_calls }
    }
}

#[async_trait]
impl ToolExecutor for MockTool {
    fn definition(&self, name: &str) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: "Mock tool for testing".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" },
                    "command": { "type": "string" },
                    "query": { "type": "string" }
                }
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult, AppError> {
        let name = args.get("path")
            .or_else(|| args.get("command"))
            .or_else(|| args.get("query"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        self.recorded_calls.write().await.push((name.clone(), args.clone()));

        // Simulate writing a file
        if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
            if let Some(content) = args.get("content").and_then(|v| v.as_str()) {
                let full_path = self.work_dir.join(path);
                if let Some(parent) = full_path.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }
                let _ = tokio::fs::write(&full_path, content).await;
            }
        }

        Ok(ToolResult {
            tool_call_id: String::new(),
            content: format!("Mock executed: {}", name),
            is_error: false,
        })
    }
}

// ── Failing Tool ──────────────────────────────────────────────

struct FailingTool;

#[async_trait]
impl ToolExecutor for FailingTool {
    fn definition(&self, name: &str) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: "Always fails".to_string(),
            parameters: serde_json::json!({"type": "object", "properties": {}}),
        }
    }

    async fn execute(&self, _args: serde_json::Value) -> Result<ToolResult, AppError> {
        Ok(ToolResult {
            tool_call_id: String::new(),
            content: "File not found".to_string(),
            is_error: true,
        })
    }
}

// ── Helper: build AgentContext ──────────────────────────────────

fn build_mock_context(
    provider: Arc<dyn mnemosyne_lib::infra::llm::types::Provider>,
    tools: Arc<ToolRegistry>,
    memory: Arc<RwLock<MemorySystem>>,
) -> AgentContext {
    AgentContext {
        provider,
        model: "mock-model".to_string(),
        project_root: PathBuf::from("/tmp/mock-workspace"),
        book_id: None,
        tools,
        memory,
        iteration_budget: Arc::new(IterationBudget::new(50)),
        tool_guardrails: Arc::new(tokio::sync::Mutex::new(
            ToolCallGuardrailController::new(ToolGuardrailConfig::default())
        )),
        context_compressor: Arc::new(tokio::sync::Mutex::new(
            ContextCompressor::new(CompressorConfig::default())
        )),
        skill_manager: None,
        user_profile: None,
    }
}

// ══════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_safety_gate_blocks_high_risk() {
    assert_eq!(SafetyGate::evaluate_risk("bash", &serde_json::json!({"command": "rm -rf /"})), RiskLevel::High);
    assert_eq!(SafetyGate::evaluate_risk("bash", &serde_json::json!({"command": "ls"})), RiskLevel::Safe);
    assert_eq!(SafetyGate::evaluate_risk("write_file", &serde_json::json!({"path": "test.txt", "content": "hi"})), RiskLevel::Moderate);
}

#[tokio::test]
async fn test_mock_tool_executes() {
    let temp_dir = TempDir::new().unwrap();
    let recorded = Arc::new(RwLock::new(Vec::<(String, serde_json::Value)>::new()));
    let tool = MockTool::new(temp_dir.path().to_path_buf(), recorded.clone());

    let result = tool.execute(serde_json::json!({
        "path": "output.txt",
        "content": "hello world"
    })).await.unwrap();

    assert!(!result.is_error);
    assert!(result.content.contains("Mock executed"));

    let content = tokio::fs::read_to_string(temp_dir.path().join("output.txt")).await.unwrap();
    assert_eq!(content, "hello world");

    let calls = recorded.read().await;
    assert_eq!(calls.len(), 1);
}

#[tokio::test]
async fn test_planner_creates_plan_from_mock_llm() {
    let plan_response = serde_json::json!([
        {
            "description": "Create a test file",
            "tool_name": "write_file",
            "tool_args": {"path": "test.txt", "content": "test content"}
        },
        {
            "description": "Read the test file",
            "tool_name": "read_file",
            "tool_args": {"path": "test.txt"}
        }
    ]).to_string();

    let provider = Arc::new(MockProvider::new(vec![plan_response]));
    let tools = Arc::new(ToolRegistry::new());
    let memory = Arc::new(RwLock::new(MemorySystem::new(10)));
    let ctx = build_mock_context(provider, tools, memory);

    let plan = Planner::create_plan(&ctx, "Create a test file and read it back").await.unwrap();
    assert_eq!(plan.len(), 2);
    assert_eq!(plan[0].tool_name.as_deref(), Some("write_file"));
    assert_eq!(plan[1].tool_name.as_deref(), Some("read_file"));
    assert_eq!(plan[0].risk_level, RiskLevel::Moderate);
    assert_eq!(plan[1].risk_level, RiskLevel::Safe);
}

#[tokio::test]
async fn test_agent_loop_executes_plan_with_tools() {
    let temp_dir = TempDir::new().unwrap();
    let recorded = Arc::new(RwLock::new(Vec::<(String, serde_json::Value)>::new()));

    let plan_response = serde_json::json!([
        {
            "description": "Write a config file",
            "tool_name": "write_file",
            "tool_args": {"path": "config.json", "content": "{\"key\": \"value\"}"}
        }
    ]).to_string();

    let provider = Arc::new(MockProvider::new(vec![plan_response]));
    let tool = MockTool::new(temp_dir.path().to_path_buf(), recorded.clone());

    let mut tools = ToolRegistry::new();
    tools.register("write_file", Box::new(MockTool::new(temp_dir.path().to_path_buf(), recorded.clone())));
    tools.register("read_file", Box::new(MockTool::new(temp_dir.path().to_path_buf(), recorded.clone())));

    let memory = Arc::new(RwLock::new(MemorySystem::new(10)));
    let ctx = build_mock_context(provider, Arc::new(tools), memory);

    let (prog_tx, mut _prog_rx) = mpsc::unbounded_channel();
    let (conf_tx, mut conf_rx) = mpsc::unbounded_channel::<ConfirmationRequest>();
    let (resp_tx, resp_rx) = mpsc::unbounded_channel::<ConfirmationResponse>();

    tokio::spawn(async move {
        while let Some(_req) = conf_rx.recv().await {
            let _ = resp_tx.send(ConfirmationResponse::Approved);
        }
    });

    let agent = AgentLoop::new(ctx, prog_tx, conf_tx, resp_rx);
    let result = agent.execute("Create a config file").await.unwrap();

    assert!(result.contains("Goal:"));
    assert!(result.contains("Completed:"));
}

#[tokio::test]
async fn test_agent_loop_rejects_high_risk_step() {
    let temp_dir = TempDir::new().unwrap();
    let recorded = Arc::new(RwLock::new(Vec::<(String, serde_json::Value)>::new()));

    let plan_response = serde_json::json!([
        {
            "description": "Delete all files",
            "tool_name": "bash",
            "tool_args": {"command": "rm -rf /"}
        }
    ]).to_string();

    let provider = Arc::new(MockProvider::new(vec![plan_response]));
    let tool = MockTool::new(temp_dir.path().to_path_buf(), recorded.clone());

    let mut tools = ToolRegistry::new();
    tools.register("bash", Box::new(MockTool::new(temp_dir.path().to_path_buf(), recorded.clone())));
    tools.register("write_file", Box::new(MockTool::new(temp_dir.path().to_path_buf(), recorded.clone())));

    let memory = Arc::new(RwLock::new(MemorySystem::new(10)));
    let ctx = build_mock_context(provider, Arc::new(tools), memory);

    let (prog_tx, _) = mpsc::unbounded_channel();
    let (conf_tx, mut conf_rx) = mpsc::unbounded_channel::<ConfirmationRequest>();
    let (resp_tx, resp_rx) = mpsc::unbounded_channel::<ConfirmationResponse>();

    tokio::spawn(async move {
        while let Some(_req) = conf_rx.recv().await {
            let _ = resp_tx.send(ConfirmationResponse::Rejected);
        }
    });

    let agent = AgentLoop::new(ctx, prog_tx, conf_tx, resp_rx);
    let result = agent.execute("Delete everything").await.unwrap();

    assert!(result.contains("SKIPPED"), "Expected SKIPPED in result: {}", result);
}

#[tokio::test]
async fn test_memory_search_finds_relevant_entries() {
    let mut memory = MemorySystem::new(10);

    memory.archive(MemoryEntry {
        id: "1".to_string(),
        content: "Alice is the protagonist with red hair".to_string(),
        entry_type: MemoryType::Character,
        chapter: Some(1),
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        tags: vec![],
    });

    memory.archive(MemoryEntry {
        id: "2".to_string(),
        content: "The kingdom of Eldoria is in the north".to_string(),
        entry_type: MemoryType::Setting,
        chapter: Some(1),
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        tags: vec![],
    });

    let results = memory.search_memory("Alice", 5);
    assert!(!results.is_empty());
    assert!(results[0].content.contains("Alice"));

    let results = memory.search_memory("kingdom", 5);
    assert!(!results.is_empty());
    assert!(results[0].content.contains("Eldoria"));

    let results = memory.search_memory("dragon spaceship", 5);
    assert!(results.is_empty());
}

#[tokio::test]
async fn test_agent_context_carries_memory() {
    let memory = Arc::new(RwLock::new(MemorySystem::new(10)));
    {
        let mut mem = memory.write().await;
        mem.archive(MemoryEntry {
            id: "1".to_string(),
            content: "Important context about the story".to_string(),
            entry_type: MemoryType::Fact,
            chapter: None,
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            tags: vec![],
        });
    }

    let provider = Arc::new(MockProvider::new(vec![
        serde_json::json!([{
            "description": "Search memory",
            "tool_name": "search_memory",
            "tool_args": {"query": "story"}
        }]).to_string()
    ]));

    let mut tools = ToolRegistry::new();
    tools.register("search_memory", Box::new(MockTool::new(
        PathBuf::from("/tmp"),
        Arc::new(RwLock::new(Vec::new())),
    )));

    let ctx = build_mock_context(provider, Arc::new(tools), memory.clone());

    let mem = ctx.memory.read().await;
    let results = mem.search_memory("story", 5);
    assert!(!results.is_empty());
    assert!(results[0].content.contains("Important context"));
}

#[tokio::test]
async fn test_user_profile_in_agent_context() {
    use mnemosyne_lib::domain::user_profile::{UserProfileStore, WritingStyle, ReaderType};

    let temp_dir = TempDir::new().unwrap();
    let mut store = UserProfileStore::new(temp_dir.path());
    let profile = store.get_or_create();

    let mut p = profile.clone();
    p.name = "TestWriter".to_string();
    p.style = WritingStyle {
        formality: "literary".to_string(),
        pacing: "slow".to_string(),
        description_density: "rich".to_string(),
        dialogue_style: "natural".to_string(),
    };
    p.reader_type = ReaderType::Literary;
    store.update(p).unwrap();

    let prompt = store.format_for_prompt();
    assert!(prompt.contains("TestWriter"));
    assert!(prompt.contains("literary"));
    assert!(prompt.contains("rich"));
}
