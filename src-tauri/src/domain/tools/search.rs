use std::fs;
use std::path::Path;
use super::types::*;
use crate::errors::AppError;
use crate::infra::llm::ToolSpec;

pub struct GrepTool;

impl ToolExecutor for GrepTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "grep".into(),
            description: "Search file contents by regex pattern".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Regex pattern to search for" },
                    "path": { "type": "string", "description": "Directory or file to search in (default: working directory)" },
                    "include": { "type": "string", "description": "File pattern to include (e.g. '*.ts')" }
                },
                "required": ["pattern"]
            }),
        }
    }

    fn execute(&self, call: &ToolCall, ctx: &ToolContext) -> Result<ToolOutput, AppError> {
        let pattern = call.args["pattern"].as_str()
            .ok_or_else(|| AppError::missing_field("pattern"))?;
        let search_path = call.args["path"].as_str().unwrap_or(".");
        let include = call.args["include"].as_str();
        let resolved = if Path::new(search_path).is_absolute() {
            search_path.to_string()
        } else { Path::new(&ctx.work_dir).join(search_path).to_string_lossy().to_string() };
        tracing::debug!(pattern = %pattern, path = %resolved, include = ?include, "grep");
        let re = regex::Regex::new(pattern)
            .map_err(|e| {
                tracing::warn!(pattern = %pattern, error = %e, "Invalid regex");
                AppError::invalid_format(format!("Invalid regex: {}", e))
            })?;
        let mut results = Vec::new();
        search_dir(&resolved, &re, include, &ctx.work_dir, &mut results, 0)?;
        tracing::debug!(matches = results.len(), "grep completed");
        if results.is_empty() { Ok(ToolOutput::success("No matches found".to_string())) }
        else { Ok(ToolOutput::success(results.join("\n"))) }
    }
}

fn search_dir(dir: &str, re: &regex::Regex, include: Option<&str>, work_dir: &str, results: &mut Vec<String>, depth: usize) -> Result<(), AppError> {
    if depth > 10 { return Ok(()); }
    let entries = fs::read_dir(dir).map_err(|e| {
        tracing::error!(error = %e, dir = %dir, "Failed to read dir for grep");
        AppError::directory_not_found(dir)
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| AppError::internal(e.to_string()))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == "node_modules" || name == "target" { continue; }
        if path.is_dir() {
            search_dir(&path.to_string_lossy(), re, include, work_dir, results, depth + 1)?;
        } else if path.is_file() {
            if let Some(inc) = include { if !matches_glob(inc, &name) { continue; } }
            if let Ok(content) = fs::read_to_string(&path) {
                for (line_num, line) in content.lines().enumerate() {
                    if re.is_match(line) {
                        let rel = path.strip_prefix(work_dir).unwrap_or(&path).to_string_lossy();
                        results.push(format!("{}:{}: {}", rel, line_num + 1, line));
                        if results.len() >= 100 { return Ok(()); }
                    }
                }
            }
        }
    }
    Ok(())
}

fn matches_glob(pattern: &str, name: &str) -> bool {
    if pattern.starts_with("*.") { let ext = &pattern[1..]; name.ends_with(ext) }
    else { name.contains(pattern) }
}

pub struct GlobTool;

impl ToolExecutor for GlobTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "glob".into(),
            description: "Find files matching a glob pattern".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Glob pattern (e.g. '**/*.ts')" },
                    "path": { "type": "string", "description": "Root directory (default: working directory)" }
                },
                "required": ["pattern"]
            }),
        }
    }

    fn execute(&self, call: &ToolCall, ctx: &ToolContext) -> Result<ToolOutput, AppError> {
        let pattern = call.args["pattern"].as_str()
            .ok_or_else(|| AppError::missing_field("pattern"))?;
        let root = call.args["path"].as_str().unwrap_or(&ctx.work_dir);
        let full_pattern = if Path::new(pattern).is_absolute() { pattern.to_string() }
        else { Path::new(root).join(pattern).to_string_lossy().to_string() };
        tracing::debug!(pattern = %pattern, root = %root, "glob");
        let mut glob_results = glob::glob(&full_pattern)
            .map_err(|e| {
                tracing::warn!(pattern = %full_pattern, error = %e, "Invalid glob pattern");
                AppError::invalid_format(format!("Invalid glob pattern: {}", e))
            })?;
        let mut files = Vec::new();
        while let Some(entry) = glob_results.next() {
            if let Ok(path) = entry {
                let rel = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().to_string();
                files.push(rel);
                if files.len() >= 200 { break; }
            }
        }
        files.sort();
        tracing::debug!(count = files.len(), "glob completed");
        Ok(ToolOutput::success(files.join("\n")))
    }
}
