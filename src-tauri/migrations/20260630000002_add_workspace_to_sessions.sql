-- 给 sessions 表添加 workspace_id 字段，建立 session ↔ workspace 关联。
--
-- 背景：此前 session 只通过 novel_id 间接关联 workspace，导致 chat agent
-- 无法访问用户在 UI 中选择的工作区内容。新增 workspace_id 让 agent 能按
-- session 找到对应的工作区，从而组装工作区上下文。
--
-- workspace_id 可空（兼容历史 session），无外键约束以避免与 workspace 删除
-- 的级联冲突（workspace 删除时 session 保留，workspace_id 置 NULL 由应用层处理）。

ALTER TABLE sessions ADD COLUMN workspace_id TEXT;
CREATE INDEX idx_sessions_workspace ON sessions(workspace_id);
