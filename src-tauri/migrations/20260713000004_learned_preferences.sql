-- 学习到的用户偏好 —— 从交互中自动提取的偏好。
--
-- 与 user_profile 表的区别:
-- - user_profile 是用户手动配置的静态偏好(写作风格/语言/阅读类型)
-- - learned_preferences 是从交互中自动学习的偏好(代码风格/工作时段/常用工具)
--
-- 置信度模型:
-- - occurrence_count < 3 → confidence = 0.2(探索期)
-- - occurrence_count >= 3 → confidence = 0.5(确认期)
-- - occurrence_count >= 7 → confidence = 0.8(稳定期)
-- - last_seen_at 超过 30 天 → confidence *= 0.5(衰减)
--
-- 高置信度偏好(>=0.7)会自动合并到 UserProfile,低置信度只做记录不应用

CREATE TABLE learned_preferences (
    id TEXT PRIMARY KEY,                    -- ULID/UUID
    preference_key TEXT NOT NULL,          -- 如 "code_style" / "work_hours" / "favorite_tools"
    preference_value TEXT NOT NULL,         -- 如 "concise" / "09-18" / "git/cargo"
    confidence REAL NOT NULL DEFAULT 0.2,   -- 0.0-1.0,越高越可信
    occurrence_count INTEGER NOT NULL DEFAULT 1, -- 出现次数(每次相同偏好被识别 +1)
    learned_from TEXT,                      -- 来源标识,如 "session:abc123" 或 "manual"
    last_seen_at TEXT NOT NULL,             -- ISO8601,最近一次被识别的时间
    created_at TEXT NOT NULL,               -- ISO8601,首次被识别的时间
    UNIQUE(preference_key, preference_value) -- 同一 key+value 只保留一条
);

CREATE INDEX idx_learned_pref_key ON learned_preferences(preference_key);
CREATE INDEX idx_learned_pref_confidence ON learned_preferences(confidence DESC);
CREATE INDEX idx_learned_pref_last_seen ON learned_preferences(last_seen_at);
