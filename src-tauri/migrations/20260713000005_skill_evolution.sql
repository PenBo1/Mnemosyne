-- 技能进化系统 —— 追踪 skill 使用频率 + 提取 skill 候选。
--
-- 两张表:
-- 1. skill_usage_stats:已存在 skill 的使用统计(每次工具调用时增量更新)
-- 2. skill_candidate_proposals:从成功任务中提取的潜在 skill(等待人工审核)
--
-- 成熟度模型:
-- - emerging(1-2 次):新 skill,刚开始使用
-- - developing(3-9 次):开发中,持续被使用
-- - mature(10+ 次):成熟,可信赖
-- - deprecated(90 天未用):弃用,可考虑删除
--
-- skill_candidate_proposals 的状态机:
-- - pending:等待人工审核
-- - approved:已批准,转化为正式 skill
-- - rejected:已拒绝(冗余/已有同名 skill)
-- - superseded:已被其他候选取代

CREATE TABLE skill_usage_stats (
    skill_name TEXT PRIMARY KEY,            -- skill 名称(对应 Skill.meta.name)
    used_count INTEGER NOT NULL DEFAULT 0,  -- 总使用次数
    success_count INTEGER NOT NULL DEFAULT 0,  -- 成功次数
    failure_count INTEGER NOT NULL DEFAULT 0,  -- 失败次数
    user_feedback_score REAL NOT NULL DEFAULT 0.0,  -- 用户反馈累计分(可正可负)
    first_used_at TEXT NOT NULL,            -- 首次使用时间
    last_used_at TEXT NOT NULL,             -- 最近使用时间
    last_session_id TEXT                    -- 最近使用的 session
);

CREATE INDEX idx_skill_usage_last_used ON skill_usage_stats(last_used_at);

CREATE TABLE skill_candidate_proposals (
    id TEXT PRIMARY KEY,                    -- ULID/UUID
    candidate_name TEXT NOT NULL,           -- 提议的 skill 名
    candidate_description TEXT NOT NULL,    -- skill 的简短描述(供审核者参考)
    source_session_id TEXT NOT NULL,        -- 来源 session
    source_summary TEXT NOT NULL,          -- 来源 session 的摘要(从 memory_short_term 拷贝)
    candidate_content TEXT NOT NULL,        -- 提议的 skill 内容(markdown 草稿)
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected', 'superseded')),
    reviewer_notes TEXT,                   -- 审核者备注(批准/拒绝原因)
    reviewed_at TEXT,                       -- 审核时间
    created_at TEXT NOT NULL                -- 创建时间
);

CREATE INDEX idx_candidate_status ON skill_candidate_proposals(status);
CREATE INDEX idx_candidate_name ON skill_candidate_proposals(candidate_name);
CREATE INDEX idx_candidate_session ON skill_candidate_proposals(source_session_id);
