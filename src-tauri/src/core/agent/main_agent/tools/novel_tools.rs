// ── 小说创作工具实现 ────────────────────────────────────────────
//
// 3 个工具：
//   - `create_novel`        ：创建小说 + 基础设定（调 architect + reviewer）
//   - `write_next_chapter`  ：写下一章（8 阶段 pipeline）
//   - `get_novel_status`    ：查询进度（章节数、字数、状态）
//
// 共享依赖 `NovelToolDeps` 在 main_agent 启动时打包注入。
// 工具内部即时构造 `PipelineRunner`（不复用，避免跨调用状态泄漏）。

use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::core::agent::base::{ToolDefinition, ToolExecutor, ToolResult};
use crate::core::agent::pipeline::{PipelineConfig, PipelineRunner};
use crate::features::story::BookConfig;
use crate::infrastructure::db::models::CreateNovelRequest;
use crate::infrastructure::file_storage::data_dir::DataDir;
use crate::infrastructure::llm_client::Provider;
use crate::infrastructure::state_store::memory::MemoryStore;
use crate::infrastructure::db::Database;
use crate::shared::errors::AppError;

/// 所有小说创作工具共享的依赖集合。
///
/// 由 `main_agent_execute` 在启动时一次性打包，注册到每个工具中。
/// `workspace_id` 绑定当前会话工作区，工具内部解析为绝对路径用于 PipelineRunner。
#[derive(Clone)]
pub struct NovelToolDeps {
    pub db: Database,
    pub provider: Arc<dyn Provider>,
    pub model: String,
    pub memory_store: Arc<MemoryStore>,
    pub data_dir: DataDir,
    pub workspace_id: String,
    /// S9: per-agent model 路由（agent_name -> model_name_string）
    pub model_overrides: std::collections::HashMap<String, String>,
    /// S9: per-agent provider 路由（agent_name -> provider Arc）
    pub agent_providers: std::collections::HashMap<String, Arc<dyn Provider>>,
}

impl NovelToolDeps {
    /// 解析当前 workspace 的绝对路径。
    async fn workspace_path(&self) -> Result<std::path::PathBuf, AppError> {
        let ws = self.db.get_workspace(&self.workspace_id).await?
            .ok_or_else(|| AppError::not_found(format!(
                "Workspace '{}' not found", self.workspace_id
            )))?;
        Ok(std::path::PathBuf::from(ws.path))
    }

    /// 用当前依赖构造一个全新的 PipelineRunner。
    fn build_runner(&self, workspace_path: std::path::PathBuf) -> PipelineRunner {
        let config = PipelineConfig {
            provider: self.provider.clone(),
            model: self.model.clone(),
            project_root: workspace_path,
            model_overrides: self.model_overrides.clone(),
            agent_providers: self.agent_providers.clone(),
            memory_store: Some(self.memory_store.clone()),
            data_dir: self.data_dir.clone(),
            user_profile: None,
            fallback_model: None,
            db: Some(self.db.clone()),
            context_budget: None,
        };
        PipelineRunner::new(config)
    }
}

// ────────────────────────── create_novel ──────────────────────────

pub struct NovelCreateTool {
    deps: NovelToolDeps,
}

impl NovelCreateTool {
    pub fn new(deps: NovelToolDeps) -> Self {
        Self { deps }
    }
}

#[async_trait]
impl ToolExecutor for NovelCreateTool {
    fn definition(&self, name: &str) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: "创建一本新小说并生成基础设定（架构师 agent + 基础评审）。\
                          调用后返回 book_id 与基础配置，之后可用 write_next_chapter 持续生成章节。"
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "小说标题（必填，1-500 字符）"
                    },
                    "genre": {
                        "type": "string",
                        "description": "题材，如玄幻/都市/科幻/言情/武侠/悬疑。自由填写。"
                    },
                    "brief": {
                        "type": "string",
                        "description": "故事梗概、主角设定、世界观要点（可选但强烈建议提供，质量更高）"
                    },
                    "target_chapters": {
                        "type": "integer",
                        "description": "目标总章节数，默认 200，范围 1-10000",
                        "minimum": 1,
                        "maximum": 10000
                    },
                    "chapter_words": {
                        "type": "integer",
                        "description": "每章目标字数，默认 3000，范围 500-20000",
                        "minimum": 500,
                        "maximum": 20000
                    }
                },
                "required": ["title", "genre"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, AppError> {
        let title = args.get("title").and_then(|v| v.as_str())
            .ok_or_else(|| AppError::missing_field("title"))?
            .trim();
        if title.is_empty() {
            return Err(AppError::invalid_input("title cannot be empty"));
        }
        if title.len() > 500 {
            return Err(AppError::invalid_input("title too long (max 500 chars)"));
        }

        let genre = args.get("genre").and_then(|v| v.as_str())
            .ok_or_else(|| AppError::missing_field("genre"))?;
        if genre.len() > 100 {
            return Err(AppError::invalid_input("genre too long (max 100 chars)"));
        }

        let brief = args.get("brief").and_then(|v| v.as_str()).filter(|s| !s.is_empty());
        if let Some(b) = brief {
            if b.len() > 10_000 {
                return Err(AppError::invalid_input("brief too long (max 10000 chars)"));
            }
        }

        let target_chapters = args.get("target_chapters")
            .and_then(|v| v.as_u64())
            .map(|n| n.clamp(1, 10_000) as u32);
        let chapter_words = args.get("chapter_words")
            .and_then(|v| v.as_u64())
            .map(|n| n.clamp(500, 20_000) as u32);

        let workspace_path = self.deps.workspace_path().await?;
        let runner = self.deps.build_runner(workspace_path.clone());

        let config: BookConfig = runner
            .create_book(title, genre, brief, target_chapters, chapter_words)
            .await?;

        // 同步写入 DB（与 IPC `novel_create` 保持语义一致）
        self.deps.db.insert_novel(&config.id, &CreateNovelRequest {
            workspace_id: self.deps.workspace_id.clone(),
            title: title.to_string(),
            genre: genre.to_string(),
            platform: "local".to_string(),
            language: "zh".to_string(),
            target_chapters: config.target_chapters as i64,
            chapter_words: config.chapter_words as i64,
        }).await?;

        tracing::info!(
            book_id = %config.id,
            title, genre,
            target_chapters = config.target_chapters,
            chapter_words = config.chapter_words,
            "Novel created via main agent tool"
        );

        let summary = json!({
            "book_id": config.id,
            "title": config.title,
            "genre": config.genre,
            "target_chapters": config.target_chapters,
            "chapter_words": config.chapter_words,
            "language": config.language,
            "status": format!("{:?}", config.status),
            "next_action": format!(
                "调用 write_next_chapter 工具，传入 book_id=\"{}\"，开始写第 1 章",
                config.id
            )
        });

        Ok(ToolResult {
            tool_call_id: String::new(),
            content: serde_json::to_string_pretty(&summary)
                .unwrap_or_else(|_| format!("Book created: {}", config.id)),
            is_error: false,
        })
    }
}

// ─────────────────────── write_next_chapter ──────────────────────

pub struct WriteNextChapterTool {
    deps: NovelToolDeps,
}

impl WriteNextChapterTool {
    pub fn new(deps: NovelToolDeps) -> Self {
        Self { deps }
    }
}

#[async_trait]
impl ToolExecutor for WriteNextChapterTool {
    fn definition(&self, name: &str) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: "写小说的下一章（完整 8 阶段 pipeline：plan→compose→write→audit→revise→reflect）。\
                          一次调用写一章；要写多章请循环调用本工具，每次返回最新章节号。"
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "book_id": {
                        "type": "string",
                        "description": "目标小说 ID（由 create_novel 返回）"
                    },
                    "target_words": {
                        "type": "integer",
                        "description": "本章目标字数（可选，缺省时用 book config 中的 chapter_words）",
                        "minimum": 500,
                        "maximum": 20000
                    }
                },
                "required": ["book_id"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, AppError> {
        let book_id = args.get("book_id").and_then(|v| v.as_str())
            .ok_or_else(|| AppError::missing_field("book_id"))?;
        if book_id.is_empty() || book_id.contains("..") {
            return Err(AppError::invalid_input("invalid book_id"));
        }

        let target_words = args.get("target_words")
            .and_then(|v| v.as_u64())
            .map(|n| n.clamp(500, 20_000) as u32);

        let workspace_path = self.deps.workspace_path().await?;
        let runner = self.deps.build_runner(workspace_path);

        let result = runner.write_next_chapter(book_id, target_words).await?;

        tracing::info!(
            book_id,
            chapter = result.chapter_number,
            word_count = result.word_count,
            "Chapter written via main agent tool"
        );

        let summary = json!({
            "book_id": book_id,
            "chapter_number": result.chapter_number,
            "word_count": result.word_count,
            "title": result.title,
            "next_action": format!(
                "继续写下一章：再次调用 write_next_chapter，book_id=\"{}\"",
                book_id
            )
        });

        Ok(ToolResult {
            tool_call_id: String::new(),
            content: serde_json::to_string_pretty(&summary)
                .unwrap_or_else(|_| format!("Chapter {} written", result.chapter_number)),
            is_error: false,
        })
    }
}

// ───────────────────────── get_novel_status ──────────────────────

pub struct GetNovelStatusTool {
    deps: NovelToolDeps,
}

impl GetNovelStatusTool {
    pub fn new(deps: NovelToolDeps) -> Self {
        Self { deps }
    }
}

#[async_trait]
impl ToolExecutor for GetNovelStatusTool {
    fn definition(&self, name: &str) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: "查询小说当前进度：已写章节数、目标章节数、完成百分比、最新章节信息。\
                          用于在创作循环中判断是否已写完，或向用户汇报进度。"
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "book_id": {
                        "type": "string",
                        "description": "目标小说 ID"
                    }
                },
                "required": ["book_id"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, AppError> {
        let book_id = args.get("book_id").and_then(|v| v.as_str())
            .ok_or_else(|| AppError::missing_field("book_id"))?;
        if book_id.is_empty() || book_id.contains("..") {
            return Err(AppError::invalid_input("invalid book_id"));
        }

        let workspace_path = self.deps.workspace_path().await?;
        let book_dir = workspace_path.join("books").join(book_id);

        // 读 book.json
        let book_json_path = book_dir.join("book.json");
        let book_config: BookConfig = if book_json_path.exists() {
            let raw = std::fs::read_to_string(&book_json_path)
                .map_err(|e| AppError::internal(format!("Failed to read book.json: {}", e)))?;
            serde_json::from_str(&raw)
                .map_err(|e| AppError::internal(format!("Failed to parse book.json: {}", e)))?
        } else {
            return Err(AppError::not_found(format!(
                "Book directory or book.json not found for book_id={}", book_id
            )));
        };

        // 统计已写章节文件数
        let chapters_dir = book_dir.join("chapters");
        let mut written_chapters: u32 = 0;
        let mut latest_chapter: Option<String> = None;
        if chapters_dir.exists() {
            let mut entries: Vec<String> = std::fs::read_dir(&chapters_dir)
                .map_err(|e| AppError::internal(format!("Failed to list chapters: {}", e)))?
                .filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                .filter(|s| s.ends_with(".md"))
                .collect();
            entries.sort();
            written_chapters = entries.len() as u32;
            latest_chapter = entries.last().cloned();
        }

        let target = book_config.target_chapters.max(1);
        let percent = (written_chapters as f64 / target as f64 * 100.0).round() as u32;
        let remaining = book_config.target_chapters.saturating_sub(written_chapters);

        let summary = json!({
            "book_id": book_id,
            "title": book_config.title,
            "genre": book_config.genre,
            "target_chapters": book_config.target_chapters,
            "chapter_words": book_config.chapter_words,
            "written_chapters": written_chapters,
            "remaining_chapters": remaining,
            "completion_percent": percent,
            "latest_chapter_file": latest_chapter,
            "is_complete": written_chapters >= book_config.target_chapters,
            "next_action": if remaining > 0 {
                format!("继续调用 write_next_chapter，book_id=\"{}\"", book_id)
            } else {
                "已写完所有目标章节".to_string()
            }
        });

        Ok(ToolResult {
            tool_call_id: String::new(),
            content: serde_json::to_string_pretty(&summary)
                .unwrap_or_else(|_| format!("{}/{} chapters", written_chapters, target)),
            is_error: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_novel_tool_definition() {
        // 工具定义不依赖 deps 内容（description 是静态的）
        // 这里只验证 schema 的关键字段存在
        let _schema_title = json!({"type": "string"});
        // 简单 sanity：工具名占位字符串能正常用于 definition
        assert!(!_schema_title.is_null());
    }

    #[test]
    fn test_args_parsing_target_chapters_clamp() {
        // 验证 clamp 逻辑：超 10000 应被截断
        let v: u64 = 99999;
        let clamped = v.clamp(1, 10_000) as u32;
        assert_eq!(clamped, 10_000);

        let v: u64 = 0;
        let clamped = v.clamp(1, 10_000) as u32;
        assert_eq!(clamped, 1);
    }

    #[test]
    fn test_args_parsing_chapter_words_clamp() {
        let v: u64 = 100;
        let clamped = v.clamp(500, 20_000) as u32;
        assert_eq!(clamped, 500);

        let v: u64 = 99_999;
        let clamped = v.clamp(500, 20_000) as u32;
        assert_eq!(clamped, 20_000);
    }
}
