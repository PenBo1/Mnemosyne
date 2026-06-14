use super::types::*;
use crate::errors::AppError;
use crate::infra::llm::ToolSpec;

pub struct NovelInfoTool { pub project_root: std::path::PathBuf }
impl ToolExecutor for NovelInfoTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec { name: "novel_info".into(), description: "Get detailed information about the current novel".into(),
            parameters: serde_json::json!({ "type": "object", "properties": {}, "required": [] }) }
    }
    fn execute(&self, _call: &ToolCall, _ctx: &ToolContext) -> Result<ToolOutput, AppError> {
        let config_path = self.project_root.join("book.json");
        if !config_path.exists() { return Ok(ToolOutput::error("No novel found in this workspace".to_string())); }
        let content = std::fs::read_to_string(&config_path).map_err(|e| AppError::internal(format!("Failed to read config: {}", e)))?;
        Ok(ToolOutput::success(content))
    }
}

pub struct ChapterReadTool { pub project_root: std::path::PathBuf }
impl ToolExecutor for ChapterReadTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec { name: "chapter_read".into(), description: "Read a specific chapter's content".into(),
            parameters: serde_json::json!({ "type": "object", "properties": {
                "chapter_number": { "type": "number", "description": "Chapter number" }
            }, "required": ["chapter_number"] }) }
    }
    fn execute(&self, call: &ToolCall, _ctx: &ToolContext) -> Result<ToolOutput, AppError> {
        let chapter_number = call.args["chapter_number"].as_u64().ok_or_else(|| AppError::bad_request("chapter_number is required"))?;
        let chapters_dir = self.project_root.join("chapters");
        if !chapters_dir.exists() { return Ok(ToolOutput::error("No chapters found".to_string())); }
        let prefix = format!("{:04}_", chapter_number);
        let entries: Vec<_> = std::fs::read_dir(&chapters_dir).map_err(|e| AppError::internal(format!("Failed to read chapters: {}", e)))?
            .filter_map(|e| e.ok()).filter(|e| e.file_name().to_string_lossy().starts_with(&prefix)).collect();
        match entries.first() {
            Some(entry) => { let content = std::fs::read_to_string(entry.path()).map_err(|e| AppError::internal(format!("Failed to read chapter: {}", e)))?; Ok(ToolOutput::success(content)) }
            None => Ok(ToolOutput::error(format!("Chapter {} not found", chapter_number))),
        }
    }
}

pub struct ChapterListTool { pub project_root: std::path::PathBuf }
impl ToolExecutor for ChapterListTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec { name: "chapter_list".into(), description: "List all chapters".into(),
            parameters: serde_json::json!({ "type": "object", "properties": {}, "required": [] }) }
    }
    fn execute(&self, _call: &ToolCall, _ctx: &ToolContext) -> Result<ToolOutput, AppError> {
        let chapters_dir = self.project_root.join("chapters");
        if !chapters_dir.exists() { return Ok(ToolOutput::success("No chapters found".to_string())); }
        let mut chapters: Vec<String> = std::fs::read_dir(&chapters_dir).map_err(|e| AppError::internal(format!("Failed to read chapters: {}", e)))?
            .filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.ends_with(".md")).collect();
        chapters.sort();
        Ok(ToolOutput::success(chapters.join("\n")))
    }
}

pub struct NovelListTool { pub project_root: std::path::PathBuf }
impl ToolExecutor for NovelListTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec { name: "novel_list".into(), description: "List the current workspace novel".into(),
            parameters: serde_json::json!({ "type": "object", "properties": {}, "required": [] }) }
    }
    fn execute(&self, _call: &ToolCall, _ctx: &ToolContext) -> Result<ToolOutput, AppError> {
        let config_path = self.project_root.join("book.json");
        if !config_path.exists() { return Ok(ToolOutput::success("No novel in this workspace".to_string())); }
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                let title = config["title"].as_str().unwrap_or("Unknown");
                let id = config["id"].as_str().unwrap_or("unknown");
                return Ok(ToolOutput::success(format!("[{}] {}", id, title)));
            }
        }
        Ok(ToolOutput::success("No novel in this workspace".to_string()))
    }
}
