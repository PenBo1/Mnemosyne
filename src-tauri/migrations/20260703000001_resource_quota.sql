CREATE TABLE IF NOT EXISTS resource_quotas (
    workspace_id TEXT PRIMARY KEY NOT NULL,
    cpu_percent REAL,
    memory_mb INTEGER,
    disk_mb INTEGER,
    network_mb INTEGER,
    token_limit INTEGER,
    cost_limit REAL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS resource_usage (
    workspace_id TEXT PRIMARY KEY NOT NULL,
    cpu_usage REAL NOT NULL DEFAULT 0,
    memory_usage INTEGER NOT NULL DEFAULT 0,
    disk_usage INTEGER NOT NULL DEFAULT 0,
    network_usage INTEGER NOT NULL DEFAULT 0,
    token_usage INTEGER NOT NULL DEFAULT 0,
    cost_usage REAL NOT NULL DEFAULT 0,
    last_updated TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_resource_quotas_created_at ON resource_quotas(created_at);
CREATE INDEX IF NOT EXISTS idx_resource_usage_last_updated ON resource_usage(last_updated);