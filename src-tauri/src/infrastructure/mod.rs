// infrastructure —— 基础设施层（系统访问）
//
// 子目录按职责划分：
// - db/            数据库（SQLite + sqlx，各业务域 store）
// - llm_client/    LLM API 客户端（OpenAI/Anthropic/Ollama/Agnes + registry）
// - sandbox/       沙箱与安全（执行隔离 + 注入防御 + 输出验证）
// - file_storage/  文件读写（data_dir/fs_utils/epub/secrets）
// - state_store/   状态存储（memory/feedback/gc）
// - ai_services/   AI 服务（mcp/rag/token_budget/output_validator/proxy_fetch）
// - middleware/    中间件（logging）
// - utils/         工具函数
//
// 依赖规则：只依赖 shared，不依赖任何 features 或 core/agent

pub mod db;
pub mod llm_client;
pub mod sandbox;
pub mod file_storage;
pub mod state_store;
pub mod ai_services;
pub mod middleware;
pub mod utils;
