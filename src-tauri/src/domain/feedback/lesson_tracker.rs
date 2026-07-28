//! ═══════════════════════════════════════════════════════════════════════════
//! 教训追踪器 - Agent 级别约束教训追踪与持久化
//! ═══════════════════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::path::PathBuf;

use tokio::io::AsyncWriteExt;

use crate::infrastructure::fs::data_dir::DataDir;
use crate::shared::error::AppError;

// ── 常量定义 ────────────────────────────────────────────────────────────────

/// MEMORY.md 中 lesson 段落标题前缀
const LESSON_HEADER_PREFIX: &str = "## Lesson: ";
/// ID 行前缀
const ID_PREFIX: &str = "- ID: ";
/// Source 行前缀
const SOURCE_PREFIX: &str = "- Source: ";
/// Created 行前缀
const CREATED_PREFIX: &str = "- Created: ";
/// Times Violated 行前缀
const TIMES_VIOLATED_PREFIX: &str = "- Times Violated: ";

// ── 类型定义 ────────────────────────────────────────────────────────────────

/// 经验教训条目
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Lesson {
    /// 教训 ID
    pub id: String,
    /// 哪个 agent 的教训
    pub agent_role: String,
    /// 约束描述
    pub constraint: String,
    /// 来源
    pub source: String,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 被违反次数
    pub times_violated: u32,
}

/// 经验教训追踪器
pub struct LessonTracker {
    /// key: lesson_id
    lessons: HashMap<String, Lesson>,
    data_dir: DataDir,
}

// ── 公共接口 ────────────────────────────────────────────────────────────────

impl LessonTracker {
    pub fn new(data_dir: DataDir) -> Self {
        Self {
            lessons: HashMap::new(),
            data_dir,
        }
    }

    /// 返回 agent MEMORY.md 的路径
    fn memory_path(&self, agent_role: &str) -> PathBuf {
        self.data_dir.agents_dir().join(agent_role).join("MEMORY.md")
    }

    /// 记录一个约束教训
    pub async fn record_lesson(
        &mut self,
        agent_role: &str,
        constraint: &str,
        source: &str,
    ) -> Result<String, AppError> {
        let id = uuid::Uuid::new_v4().to_string();
        let lesson = Lesson {
            id: id.clone(),
            agent_role: agent_role.to_string(),
            constraint: constraint.to_string(),
            source: source.to_string(),
            created_at: chrono::Utc::now(),
            times_violated: 0,
        };

        self.append_lesson_to_memory(&lesson).await?;
        self.lessons.insert(id.clone(), lesson);

        tracing::info!(
            agent_role = agent_role,
            lesson_id = %id,
            source = source,
            "Recorded constraint lesson"
        );
        Ok(id)
    }

    /// 加载某个 agent 的所有 lessons
    pub async fn load_lessons(&self, agent_role: &str) -> Result<Vec<Lesson>, AppError> {
        let path = self.memory_path(agent_role);
        let content = match tokio::fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Vec::new());
            }
            Err(e) => {
                return Err(AppError::file_read_error(format!(
                    "MEMORY.md for agent '{}': {}",
                    agent_role, e
                )))
            }
        };
        Ok(parse_lessons_from_memory(&content, agent_role))
    }

    /// 获取所有 lessons
    pub fn get_all_lessons(&self) -> Vec<&Lesson> {
        self.lessons.values().collect()
    }

    /// 标记违反
    pub async fn record_violation(&mut self, lesson_id: &str) -> Result<(), AppError> {
        let snapshot = {
            let lesson = self.lessons.get_mut(lesson_id).ok_or_else(|| {
                AppError::not_found(format!("Lesson '{}' not found in tracker", lesson_id))
            })?;
            lesson.times_violated = lesson.times_violated.saturating_add(1);
            lesson.clone()
        };
        self.rewrite_memory_for_lesson(&snapshot).await?;
        Ok(())
    }

    /// 将一个 lesson 段落 append 到 agent 的 MEMORY.md
    async fn append_lesson_to_memory(&self, lesson: &Lesson) -> Result<(), AppError> {
        let path = self.memory_path(&lesson.agent_role);

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                AppError::internal(format!(
                    "Failed to create agent dir for '{}': {}",
                    lesson.agent_role, e
                ))
            })?;
        }

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .map_err(|e| {
                AppError::file_write_error(format!(
                    "MEMORY.md (agent '{}'): {}",
                    lesson.agent_role, e
                ))
            })?;

        let block = format_lesson_block(lesson);
        let to_write = format!("\n{}\n", block);
        file.write_all(to_write.as_bytes()).await.map_err(|e| {
            AppError::file_write_error(format!(
                "MEMORY.md write (agent '{}'): {}",
                lesson.agent_role, e
            ))
        })?;
        file.flush().await.map_err(|e| {
            AppError::file_write_error(format!(
                "MEMORY.md flush (agent '{}'): {}",
                lesson.agent_role, e
            ))
        })?;
        Ok(())
    }

    /// 重写 MEMORY.md 中指定 lesson 的 Times Violated 行
    async fn rewrite_memory_for_lesson(&self, lesson: &Lesson) -> Result<(), AppError> {
        let path = self.memory_path(&lesson.agent_role);
        let content = match tokio::fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(AppError::file_not_found(format!(
                    "MEMORY.md for agent '{}' (cannot persist violation)",
                    lesson.agent_role
                )));
            }
            Err(e) => {
                return Err(AppError::file_read_error(format!(
                    "MEMORY.md (agent '{}'): {}",
                    lesson.agent_role, e
                )))
            }
        };

        let updated = update_lesson_violation_in_content(&content, lesson);
        tokio::fs::write(&path, updated).await.map_err(|e| {
            AppError::file_write_error(format!(
                "MEMORY.md (agent '{}'): {}",
                lesson.agent_role, e
            ))
        })?;
        Ok(())
    }
}

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/// 格式化 lesson 为 MEMORY.md 段落
fn format_lesson_block(lesson: &Lesson) -> String {
    format!(
        "{header}{constraint}\n{id_line}{id}\n{source_line}{source}\n{created_line}{ts}\n{tv_line}{tv}",
        header = LESSON_HEADER_PREFIX,
        constraint = lesson.constraint,
        id_line = ID_PREFIX,
        id = lesson.id,
        source_line = SOURCE_PREFIX,
        source = lesson.source,
        created_line = CREATED_PREFIX,
        ts = lesson.created_at.to_rfc3339(),
        tv_line = TIMES_VIOLATED_PREFIX,
        tv = lesson.times_violated,
    )
}

/// 从 MEMORY.md 内容解析所有 lesson 段落
fn parse_lessons_from_memory(content: &str, agent_role: &str) -> Vec<Lesson> {
    let mut lessons: Vec<Lesson> = Vec::new();
    let mut current: Option<Lesson> = None;

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix(LESSON_HEADER_PREFIX) {
            if let Some(l) = current.take() {
                lessons.push(l);
            }
            current = Some(Lesson {
                id: String::new(),
                agent_role: agent_role.to_string(),
                constraint: rest.to_string(),
                source: String::new(),
                created_at: chrono::Utc::now(),
                times_violated: 0,
            });
            continue;
        }

        if let Some(lesson) = current.as_mut() {
            if line.starts_with("# ") || line.starts_with("## ") {
                lessons.push(current.take().unwrap());
                continue;
            }
            if let Some(rest) = line.strip_prefix(ID_PREFIX) {
                lesson.id = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix(SOURCE_PREFIX) {
                lesson.source = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix(CREATED_PREFIX) {
                if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(rest.trim()) {
                    lesson.created_at = ts.with_timezone(&chrono::Utc);
                }
            } else if let Some(rest) = line.strip_prefix(TIMES_VIOLATED_PREFIX) {
                if let Ok(n) = rest.trim().parse::<u32>() {
                    lesson.times_violated = n;
                }
            }
        }
    }
    if let Some(l) = current {
        lessons.push(l);
    }
    lessons
}

/// 在 MEMORY.md 内容中更新指定 lesson 的 Times Violated 行
fn update_lesson_violation_in_content(content: &str, lesson: &Lesson) -> String {
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let target_header = format!("{}{}", LESSON_HEADER_PREFIX, lesson.constraint);

    let mut updated = false;
    let mut i = 0;
    while i < lines.len() {
        if lines[i] == target_header {
            let end = (i + 6).min(lines.len());
            let mut id_found = false;
            let mut tv_idx: Option<usize> = None;
            for (j, item) in lines.iter().enumerate().take(end).skip(i + 1) {
                if item.strip_prefix(ID_PREFIX).map(|s| s.trim()) == Some(lesson.id.as_str()) {
                    id_found = true;
                }
                if item.starts_with(TIMES_VIOLATED_PREFIX) {
                    tv_idx = Some(j);
                }
                if item.starts_with("# ") || item.starts_with("## ") {
                    break;
                }
            }
            if id_found {
                if let Some(j) = tv_idx {
                    lines[j] = format!("{}{}", TIMES_VIOLATED_PREFIX, lesson.times_violated);
                    updated = true;
                    break;
                }
            }
        }
        i += 1;
    }

    if !updated {
        tracing::warn!(
            lesson_id = %lesson.id,
            agent_role = %lesson.agent_role,
            "Lesson block not found in MEMORY.md, violation count not persisted to disk"
        );
    }

    let trailing_newline = content.ends_with('\n');
    let mut result = lines.join("\n");
    if trailing_newline {
        result.push('\n');
    }
    result
}

// ── 测试 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_tracker() -> (LessonTracker, TempDir) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        (LessonTracker::new(data_dir), tmp)
    }

    #[tokio::test]
    async fn test_record_and_load_lesson() {
        let (mut tracker, _tmp) = temp_tracker();
        let id = tracker
            .record_lesson("writer", "不要写超过 3000 字", "continuity_audit")
            .await
            .expect("record_lesson should succeed");
        assert!(!id.is_empty());

        let all = tracker.get_all_lessons();
        assert_eq!(all.len(), 1, "tracker should hold 1 lesson in memory");
        assert_eq!(all[0].id, id);
        assert_eq!(all[0].agent_role, "writer");
        assert_eq!(all[0].constraint, "不要写超过 3000 字");
        assert_eq!(all[0].source, "continuity_audit");
        assert_eq!(all[0].times_violated, 0);

        let loaded = tracker.load_lessons("writer").await.expect("load_lessons should succeed");
        assert_eq!(loaded.len(), 1, "MEMORY.md should contain 1 lesson");
        assert_eq!(loaded[0].id, id);
        assert_eq!(loaded[0].constraint, "不要写超过 3000 字");
        assert_eq!(loaded[0].source, "continuity_audit");
        assert_eq!(loaded[0].times_violated, 0);
    }

    #[tokio::test]
    async fn test_record_violation() {
        let (mut tracker, _tmp) = temp_tracker();
        let id = tracker
            .record_lesson("writer", "约束 X", "audit")
            .await
            .expect("record_lesson should succeed");

        tracker.record_violation(&id).await.expect("first violation");
        tracker.record_violation(&id).await.expect("second violation");

        let all = tracker.get_all_lessons();
        assert_eq!(all[0].times_violated, 2, "in-memory times_violated should be 2");

        let loaded = tracker.load_lessons("writer").await.expect("load_lessons should succeed");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, id);
        assert_eq!(
            loaded[0].times_violated, 2,
            "persisted times_violated should be 2 after two record_violation calls"
        );
    }

    #[tokio::test]
    async fn test_load_nonexistent() {
        let (tracker, _tmp) = temp_tracker();
        let loaded = tracker
            .load_lessons("nonexistent_role")
            .await
            .expect("load_lessons on missing file should return empty, not error");
        assert!(loaded.is_empty(), "missing MEMORY.md should yield empty Vec");
    }
}