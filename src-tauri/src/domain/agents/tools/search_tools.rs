use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::errors::AppError;
use crate::domain::agents::base::{
    MemoryEntry, MemorySystem, MemoryType, ToolDefinition, ToolExecutor, ToolResult,
};

pub struct SearchMemoryTool {
    memory: Arc<RwLock<MemorySystem>>,
}

impl SearchMemoryTool {
    pub fn new(memory: Arc<RwLock<MemorySystem>>) -> Self {
        Self { memory }
    }
}

#[async_trait]
impl ToolExecutor for SearchMemoryTool {
    fn definition(&self, name: &str) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: "Search the archival memory store for entries matching a query. Returns the most relevant memory entries ranked by relevance.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query to find relevant memory entries"
                    },
                    "top_k": {
                        "type": "integer",
                        "description": "Maximum number of results to return (default: 5)",
                        "minimum": 1,
                        "maximum": 20
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, AppError> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::missing_field("query"))?;

        let top_k = args
            .get("top_k")
            .and_then(|v| v.as_u64())
            .unwrap_or(5) as usize;

        if query.trim().is_empty() {
            return Err(AppError::bad_request("query must not be empty".to_string()));
        }

        let results: Vec<MemoryEntry> = {
            let memory = self.memory.read().await;
            memory.search_memory(query, top_k)
                .into_iter()
                .cloned()
                .collect()
        };

        if results.is_empty() {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                content: format!("No memory entries found matching query: \"{}\"", query),
                is_error: false,
            });
        }

        let formatted: String = results
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let type_num = entry.entry_type as u8;
                format!("{}. [{}] {}", i + 1, type_num, entry.content)
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(ToolResult {
            tool_call_id: String::new(),
            content: format!("Found {} memory entries:\n{}", results.len(), formatted),
            is_error: false,
        })
    }
}

pub struct ArchiveMemoryTool {
    memory: Arc<RwLock<MemorySystem>>,
}

impl ArchiveMemoryTool {
    pub fn new(memory: Arc<RwLock<MemorySystem>>) -> Self {
        Self { memory }
    }

    fn parse_entry_type(s: &str) -> Result<MemoryType, AppError> {
        match s.to_lowercase().as_str() {
            "character" => Ok(MemoryType::Character),
            "plot" => Ok(MemoryType::Plot),
            "setting" => Ok(MemoryType::Setting),
            "dialogue" => Ok(MemoryType::Dialogue),
            "fact" => Ok(MemoryType::Fact),
            "style" => Ok(MemoryType::Style),
            _ => Err(AppError::bad_request(format!(
                "Invalid entry_type: \"{}\". Must be one of: character, plot, setting, dialogue, fact, style",
                s
            ))),
        }
    }
}

#[async_trait]
impl ToolExecutor for ArchiveMemoryTool {
    fn definition(&self, name: &str) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: "Archive a new memory entry into the archival store for future retrieval. Use this to persist important facts, character details, plot points, or style notes.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "The content of the memory entry to archive"
                    },
                    "entry_type": {
                        "type": "string",
                        "description": "Type of memory entry",
                        "enum": ["character", "plot", "setting", "dialogue", "fact", "style"]
                    },
                    "chapter": {
                        "type": "integer",
                        "description": "Optional chapter number associated with this entry"
                    }
                },
                "required": ["content", "entry_type"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, AppError> {
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::missing_field("content"))?;

        let entry_type_str = args
            .get("entry_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::missing_field("entry_type"))?;

        let entry_type = Self::parse_entry_type(entry_type_str)?;

        let chapter = args.get("chapter").and_then(|v| v.as_u64()).map(|c| c as u32);

        let entry = MemoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            content: content.to_string(),
            entry_type,
            chapter,
            timestamp: chrono::Utc::now().to_rfc3339(),
            tags: Vec::new(),
        };

        {
            let mut memory = self.memory.write().await;
            memory.archive(entry);
        }

        Ok(ToolResult {
            tool_call_id: String::new(),
            content: format!(
                "Memory entry archived successfully. Type: {}, Chapter: {}",
                entry_type_str,
                chapter.map_or("N/A".to_string(), |c| c.to_string())
            ),
            is_error: false,
        })
    }
}
