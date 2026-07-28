//! ═══════════════════════════════════════════════════════════════════════════
//! Summary - 会话摘要与用户偏好提取
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 会话摘要与用户偏好提取。
//!
//! - `summarize_session`：压缩会话最近消息为短摘要 + 关键 topics
//! - `analyze_user_preferences`：从用户消息中挖掘偏好键值对
//!
//! 两者均驱动单次 `prompt_once` 调用，结果持久化到专用 DB 表。
//! 从 engine.rs 拆分，隔离 JSON 解析辅助函数及其测试。

use crate::shared::error::AppError;

use super::AgentEngine;

impl AgentEngine {
    /// 为 session 生成短期记忆摘要并 upsert 到 memory_short_term 表。
    ///
    /// 触发时机:
    /// - session commit(用户手动 / 自动保存)
    /// - session 关闭
    /// - 用户主动请求"重新生成摘要"
    ///
    /// 流程:
    /// 1. 拉取 session 的最近 N 条消息(默认 20)
    /// 2. 拼装成 LLM 输入(角色 + 内容摘要)
    /// 3. 调用 LLM 生成 2-3 句话摘要 + 关键 topics(JSON 数组)
    /// 4. upsert 到 memory_short_term(UNIQUE session_id+date → 当天覆盖)
    ///
    /// 失败策略(对齐 "no silent fallback"):
    /// - 拉取消息失败:返回 Err
    /// - LLM 调用失败:返回 Err(不写空摘要)
    /// - JSON 解析失败:用整个响应作为 summary,key_topics 留空数组
    pub async fn summarize_session(
        &self,
        session_id: &str,
        book_id: Option<&str>,
        agent_role: Option<&str>,
    ) -> Result<(), AppError> {
        use crate::infrastructure::db::stores::short_term_memory::ShortTermMemoryRow;

        // 1. 拉取消息(全量后取最近 20 条)
        let mut messages = self.db.list_messages(session_id)?;
        if messages.is_empty() {
            tracing::debug!(session_id, "skip summarize: no messages");
            return Ok(());
        }
        let total = messages.len();
        if total > 20 {
            messages = messages.split_off(total - 20);
        }
        let message_count = messages.len() as u32;
        let total_tokens: u64 = messages.iter().map(|m| m.token_count.unwrap_or(0) as u64).sum();

        // 2. 拼装 LLM 输入
        let transcript = messages
            .iter()
            .map(|m| format!("[{}] {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        let system_prompt = "你是会话摘要助手。请把下面的对话压缩成 2-3 句话摘要,并提取 1-5 个关键主题。输出 JSON 格式:{\"summary\":\"...\",\"key_topics\":[\"...\"]}";
        let user_message = format!(
            "Session ID: {}\n消息数: {}\nToken 总量: {}\n\n对话内容:\n{}",
            session_id, message_count, total_tokens, transcript
        );

        // 3. 调用 LLM
        let raw = self.prompt_once(system_prompt, &user_message).await?;

        // 4. 解析 JSON(失败则用原始响应作为 summary)
        let (summary, key_topics) = parse_summary_response(&raw);

        // 5. upsert 到 DB
        let now = chrono::Utc::now();
        let row = ShortTermMemoryRow {
            id: format!("stm-{}-{}", session_id, now.timestamp_millis()),
            session_id: session_id.to_string(),
            book_id: book_id.map(|s| s.to_string()),
            entry_date: now.format("%Y-%m-%d").to_string(),
            summary,
            key_topics,
            agent_role: agent_role.map(|s| s.to_string()),
            token_count: total_tokens,
            message_count,
            created_at: now.to_rfc3339(),
        };
        self.db.upsert_short_term_memory(&row)?;
        tracing::info!(
            session_id,
            entry_date = %row.entry_date,
            "Session summary persisted"
        );
        Ok(())
    }

    /// 从 session 对话中提取并记录用户偏好到 learned_preferences 表。
    ///
    /// 触发时机:
    /// - 每次 send_message 完成后(异步,不阻塞响应)
    /// - 用户主动请求"分析我的偏好"
    ///
    /// 流程:
    /// 1. 拉取 session 最近 10 条用户消息(role=user)
    /// 2. 调用 LLM 分析:提取偏好键值对(code_style/tone/work_hours/...)
    /// 3. 对每个提取到的偏好调用 upsert_learned_preference
    /// 4. 返回提取的偏好数量
    ///
    /// 失败策略:
    /// - 拉取消息失败:返回 Err
    /// - LLM 调用失败:返回 Err(不静默跳过)
    /// - JSON 解析失败:返回 0(可能 LLM 没识别到偏好,不算错误)
    pub async fn analyze_user_preferences(
        &self,
        session_id: &str,
    ) -> Result<usize, AppError> {
        // 1. 拉取最近 10 条用户消息
        // 注：先 `rev().take(10)` 取最近 10 条（按时间倒序），
        // 然后 `reverse()` 原地翻转为正序，避免在 transcript 阶段再次 `rev()`。
        let messages = self.db.list_messages(session_id)?;
        let mut user_msgs: Vec<_> = messages
            .iter()
            .filter(|m| m.role == "user")
            .rev()
            .take(10)
            .collect::<Vec<_>>();
        if user_msgs.is_empty() {
            tracing::debug!(session_id, "skip preference analysis: no user messages");
            return Ok(0);
        }
        user_msgs.reverse(); // 倒序 → 正序，单次原地翻转

        let transcript = user_msgs
            .iter()
            .map(|m| m.content.clone())
            .collect::<Vec<_>>()
            .join("\n---\n");

        // 2. 调用 LLM 提取偏好
        let system_prompt = "你是用户偏好分析助手。从下面的用户消息中提取可识别的偏好。输出 JSON 数组,每个元素形如 {\"key\":\"code_style\",\"value\":\"concise\"}。如果没有识别到偏好,输出空数组 []。常见 key:code_style, tone, work_hours, favorite_tools, response_length, language, framework.";
        let user_message = format!("用户消息:\n{}", transcript);

        let raw = self.prompt_once(system_prompt, &user_message).await?;

        // 3. 解析 JSON 数组
        let prefs = parse_preferences_response(&raw);
        if prefs.is_empty() {
            tracing::debug!(session_id, "no preferences extracted from session");
            return Ok(0);
        }

        // 4. upsert 每个偏好
        let source = format!("session:{}", session_id);
        for (key, value) in &prefs {
            self.db
                .upsert_learned_preference(key, value, Some(&source))?;
        }
        tracing::info!(
            session_id,
            extracted = prefs.len(),
            "User preferences recorded"
        );
        Ok(prefs.len())
    }
}

/// 解析 LLM 摘要响应(容错:JSON 失败则退化为 raw text summary + 空 topics)
///
/// 支持三种 LLM 输出:
/// 1. 纯 JSON:`{"summary":"...","key_topics":["a","b"]}`
/// 2. JSON in code block:```json\n{...}\n```
/// 3. 混杂文本 + JSON(提取第一个 { 到最后 )
fn parse_summary_response(raw: &str) -> (String, String) {
    if let Some(json_str) = extract_json_block(raw) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json_str) {
            let summary = v
                .get("summary")
                .and_then(|s| s.as_str())
                .unwrap_or(raw)
                .to_string();
            let topics_arr = v
                .get("key_topics")
                .and_then(|t| t.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            return (
                summary,
                serde_json::to_string(&topics_arr).unwrap_or_else(|_| "[]".to_string()),
            );
        }
    }
    // 解析失败:用 raw 作为 summary,topics 留空
    (raw.to_string(), "[]".to_string())
}

/// 解析 LLM 偏好提取响应,返回 (key, value) 元组列表。
///
/// 期望格式:`[{"key":"code_style","value":"concise"}, ...]`
/// 容错:
/// - JSON 解析失败 → 返回空 Vec
/// - 数组中元素缺 key 或 value → 跳过该元素
fn parse_preferences_response(raw: &str) -> Vec<(String, String)> {
    let json_str = match extract_json_array(raw) {
        Some(s) => s,
        None => return Vec::new(),
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&json_str) else {
        return Vec::new();
    };
    let Some(arr) = v.as_array() else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|item| {
            let key = item.get("key")?.as_str()?.to_string();
            let value = item.get("value")?.as_str()?.to_string();
            if key.is_empty() || value.is_empty() {
                None
            } else {
                Some((key, value))
            }
        })
        .collect()
}

/// 从可能含 code block 的字符串中提取 JSON 数组部分
fn extract_json_array(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.starts_with('[') {
        return Some(trimmed.to_string());
    }
    if let Some(start) = trimmed.find("```json") {
        let after = &trimmed[start + 7..];
        if let Some(end) = after.find("```") {
            return Some(after[..end].trim().to_string());
        }
    }
    if let Some(start) = trimmed.find("```") {
        let after = &trimmed[start + 3..];
        if let Some(end) = after.find("```") {
            return Some(after[..end].trim().to_string());
        }
    }
    if let (Some(start), Some(end)) = (trimmed.find('['), trimmed.rfind(']')) {
        if end > start {
            return Some(trimmed[start..=end].to_string());
        }
    }
    None
}

/// 从可能含 code block 的字符串中提取 JSON 部分
fn extract_json_block(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.starts_with('{') {
        return Some(trimmed.to_string());
    }
    if let Some(start) = trimmed.find("```json") {
        let after = &trimmed[start + 7..];
        if let Some(end) = after.find("```") {
            return Some(after[..end].trim().to_string());
        }
    }
    if let Some(start) = trimmed.find("```") {
        let after = &trimmed[start + 3..];
        if let Some(end) = after.find("```") {
            return Some(after[..end].trim().to_string());
        }
    }
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if end > start {
            return Some(trimmed[start..=end].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_summary_pure_json() {
        let raw = r#"{"summary":"讨论了章节 3 的剧情","key_topics":["chapter-3","plot"]}"#;
        let (s, t) = parse_summary_response(raw);
        assert_eq!(s, "讨论了章节 3 的剧情");
        assert!(t.contains("chapter-3"));
        assert!(t.contains("plot"));
    }

    #[test]
    fn parse_summary_code_block() {
        let raw = "这是摘要:\n```json\n{\"summary\":\"很好\",\"key_topics\":[\"a\"]}\n```\n";
        let (s, t) = parse_summary_response(raw);
        assert_eq!(s, "很好");
        assert!(t.contains("\"a\""));
    }

    #[test]
    fn parse_summary_plain_code_block() {
        let raw = "```\n{\"summary\":\"纯文本块\",\"key_topics\":[\"x\",\"y\"]}\n```";
        let (s, t) = parse_summary_response(raw);
        assert_eq!(s, "纯文本块");
        assert!(t.contains("\"x\""));
        assert!(t.contains("\"y\""));
    }

    #[test]
    fn parse_summary_invalid_falls_back_to_raw() {
        let raw = "这不是 JSON,只是普通文本";
        let (s, t) = parse_summary_response(raw);
        assert_eq!(s, raw);
        assert_eq!(t, "[]");
    }

    #[test]
    fn parse_summary_embedded_json() {
        let raw = "好的,我来总结:\n{\"summary\":\"嵌入 JSON\",\"key_topics\":[\"z\"]}\n以上是总结";
        let (s, t) = parse_summary_response(raw);
        assert_eq!(s, "嵌入 JSON");
        assert!(t.contains("\"z\""));
    }

    #[test]
    fn extract_json_block_handles_no_brace() {
        assert_eq!(extract_json_block("no json here"), None);
    }

    #[test]
    fn parse_preferences_pure_array() {
        let raw = r#"[{"key":"code_style","value":"concise"},{"key":"tone","value":"friendly"}]"#;
        let prefs = parse_preferences_response(raw);
        assert_eq!(prefs.len(), 2);
        assert_eq!(prefs[0], ("code_style".to_string(), "concise".to_string()));
        assert_eq!(prefs[1], ("tone".to_string(), "friendly".to_string()));
    }

    #[test]
    fn parse_preferences_code_block_array() {
        let raw = "好的,我识别到:\n```json\n[{\"key\":\"work_hours\",\"value\":\"09-18\"}]\n```";
        let prefs = parse_preferences_response(raw);
        assert_eq!(prefs.len(), 1);
        assert_eq!(prefs[0], ("work_hours".to_string(), "09-18".to_string()));
    }

    #[test]
    fn parse_preferences_empty_array() {
        let raw = "[]";
        let prefs = parse_preferences_response(raw);
        assert_eq!(prefs.len(), 0);
    }

    #[test]
    fn parse_preferences_skips_invalid_items() {
        // 第二项缺 value,第三项缺 key,应当被跳过
        let raw = r#"[
            {"key":"ok","value":"yes"},
            {"key":"bad"},
            {"value":"bad"}
        ]"#;
        let prefs = parse_preferences_response(raw);
        assert_eq!(prefs.len(), 1);
        assert_eq!(prefs[0], ("ok".to_string(), "yes".to_string()));
    }

    #[test]
    fn parse_preferences_empty_keys_filtered() {
        let raw = r#"[{"key":"","value":"bad"},{"key":"ok","value":"good"}]"#;
        let prefs = parse_preferences_response(raw);
        assert_eq!(prefs.len(), 1);
    }

    #[test]
    fn parse_preferences_invalid_json_returns_empty() {
        let prefs = parse_preferences_response("no json here");
        assert_eq!(prefs.len(), 0);
    }

    #[test]
    fn extract_json_array_handles_no_bracket() {
        assert_eq!(extract_json_array("no array"), None);
    }
}
