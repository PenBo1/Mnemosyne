-- 工作区增加"最近打开时间"字段，用于恢复上次活动工作区
-- 启动时按 last_opened_at DESC 取首个作为 activeWorkspaceId（对标 Cursor/Notion）
ALTER TABLE workspaces ADD COLUMN last_opened_at TEXT;
CREATE INDEX idx_workspaces_last_opened ON workspaces(last_opened_at DESC);
