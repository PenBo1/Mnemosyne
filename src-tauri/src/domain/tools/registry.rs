use std::collections::HashMap;
use super::types::*;
use super::file::*;
use super::search::*;
use super::novel::*;
use crate::errors::AppError;
use crate::infra::llm::ToolSpec;

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn ToolExecutor>>,
}

impl ToolRegistry {
    pub fn new(project_root: std::path::PathBuf) -> Self {
        let mut reg = Self { tools: HashMap::new() };
        reg.register(Box::new(ReadFileTool));
        reg.register(Box::new(WriteFileTool));
        reg.register(Box::new(ListDirTool));
        reg.register(Box::new(GrepTool));
        reg.register(Box::new(GlobTool));
        reg.register(Box::new(NovelInfoTool { project_root: project_root.clone() }));
        reg.register(Box::new(ChapterReadTool { project_root: project_root.clone() }));
        reg.register(Box::new(ChapterListTool { project_root: project_root.clone() }));
        reg.register(Box::new(NovelListTool { project_root }));
        tracing::info!(count = reg.tools.len(), "Tool registry initialized");
        reg
    }

    pub fn register(&mut self, tool: Box<dyn ToolExecutor>) {
        let spec = tool.spec();
        tracing::debug!(tool = %spec.name, "Tool registered");
        self.tools.insert(spec.name.clone(), tool);
    }

    pub fn execute(&self, call: &ToolCall, ctx: &ToolContext) -> Result<ToolOutput, AppError> {
        tracing::info!(tool = %call.name, session_id = %ctx.session_id, "Tool execution started");
        let start = std::time::Instant::now();

        let tool = self.tools.get(&call.name)
            .ok_or_else(|| {
                tracing::warn!(tool = %call.name, "Tool not found");
                AppError::not_found(format!("Tool '{}' not found", call.name))
            })?;

        let result = tool.execute(call, ctx);

        let elapsed = start.elapsed().as_millis();
        match &result {
            Ok(output) => {
                tracing::info!(
                    tool = %call.name,
                    is_error = output.is_error,
                    output_len = output.content.len(),
                    elapsed_ms = elapsed,
                    "Tool execution completed"
                );
            }
            Err(e) => {
                tracing::error!(tool = %call.name, error = %e, elapsed_ms = elapsed, "Tool execution failed");
            }
        }

        result
    }

    pub fn tool_specs(&self) -> Vec<ToolSpec> {
        self.tools.values().map(|t| t.spec()).collect()
    }

    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }
}
