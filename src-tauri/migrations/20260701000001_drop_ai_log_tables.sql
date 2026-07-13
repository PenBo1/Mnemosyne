-- 删除 AI 日志死表
--
-- AI 引擎已迁移至前端 (Vercel AI SDK)，Rust 侧不再写入以下 5 张 AI 日志表：
-- - llm_calls          (LLM 调用记录)
-- - tool_executions    (工具执行记录)
-- - agent_thinking     (Agent 思考过程)
-- - sandbox_violations (沙箱违规记录)
-- - memory_operations  (记忆操作记录)
--
-- 对应的 ai_log_store.rs 与 ai_logs IPC 命令已删除，前端查询永远返回空。
-- 删除死表释放空间并保持 schema 整洁。

DROP TABLE IF EXISTS memory_operations;
DROP TABLE IF EXISTS sandbox_violations;
DROP TABLE IF EXISTS agent_thinking;
DROP TABLE IF EXISTS tool_executions;
DROP TABLE IF EXISTS llm_calls;
