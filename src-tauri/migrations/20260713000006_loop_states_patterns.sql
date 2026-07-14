-- Loop-Engineering 扩展迁移 —— 为已存在的 loop_states / loop_patterns / loop_runs 表补充列与索引。
--
-- 注意:loop_states / loop_patterns / loop_run_logs 表已在 20260628000001_init_state.sql 中创建,
--       loop_runs 表已在 20260713000002_loop_runs.sql 中创建。
--       本迁移仅做增量扩展(ALTER TABLE + ADD INDEX),不重建已有表。
--
-- 扩展内容:
-- 1. loop_patterns 添加 is_builtin 列(标记 builtin pattern,builtin 不可删除)
-- 2. loop_runs 添加扩展列(loop_state_id / findings_json / actions_json / escalations_json /
--    phase_results_json / error_message),支持前端 LoopRunLog 抽象
-- 3. 新增索引(loop_states.status / loop_patterns.is_builtin / loop_patterns.is_active /
--    loop_runs.loop_state_id)

-- ── loop_patterns: 添加 is_builtin 列 ──────────────────
ALTER TABLE loop_patterns ADD COLUMN is_builtin INTEGER NOT NULL DEFAULT 0;

CREATE INDEX idx_loop_patterns_builtin ON loop_patterns(is_builtin);
CREATE INDEX idx_loop_patterns_active ON loop_patterns(is_active);

-- ── loop_runs: 添加扩展列 ──────────────────────────────
-- 这些列支持前端 LoopRunLog 抽象(findings/actions/escalations 详情 + phase_results + error_message)
-- 与现有计数列(items_found / actions_taken / escalations)并存:
--   计数列供 budget 系统快速聚合,JSON 列供 dashboard 展示详情
ALTER TABLE loop_runs ADD COLUMN loop_state_id TEXT;
ALTER TABLE loop_runs ADD COLUMN findings_json TEXT;
ALTER TABLE loop_runs ADD COLUMN actions_json TEXT;
ALTER TABLE loop_runs ADD COLUMN escalations_json TEXT;
ALTER TABLE loop_runs ADD COLUMN phase_results_json TEXT;
ALTER TABLE loop_runs ADD COLUMN error_message TEXT;

CREATE INDEX idx_loop_runs_state ON loop_runs(loop_state_id);

-- ── loop_states: 添加 status 索引 ──────────────────────
-- idx_loop_states_novel 和 idx_loop_states_pattern 已在 init_state.sql 中创建,不重建
CREATE INDEX idx_loop_states_status ON loop_states(status);
