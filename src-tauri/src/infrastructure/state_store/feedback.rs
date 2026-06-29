use serde::{Deserialize, Serialize};
use crate::infrastructure::sandbox::security::SecretRedactor;

/// pipeline 执行过程中记录的 error event。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub id: String,
    pub agent: String,
    pub error_type: String,
    pub message: String,
    pub chapter: Option<u32>,
    pub book_id: Option<String>,
    pub timestamp: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Warning,
    Critical,
}

/// 从重复 error 模式中推导出的 constraint lesson。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintLesson {
    pub id: String,
    pub rule: String,
    pub reason: String,
    pub source_errors: Vec<String>,
    pub active: bool,
    pub created_at: String,
}

/// 何时从 error 生成 lesson 的配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRules {
    /// 生成 lesson 前的最大 warning 事件数
    pub warning_threshold: usize,
    /// 生成 lesson 前的最大 critical 事件数
    pub critical_threshold: usize,
    /// 注入 prompt 的最大活跃 lesson 数
    pub max_active_lessons: usize,
}

impl Default for FeedbackRules {
    fn default() -> Self {
        Self {
            warning_threshold: 5,
            critical_threshold: 2,
            max_active_lessons: 10,
        }
    }
}

/// 内存中的 feedback store。
pub struct FeedbackStore {
    events: Vec<ErrorEvent>,
    lessons: Vec<ConstraintLesson>,
    rules: FeedbackRules,
}

impl FeedbackStore {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            lessons: Vec::new(),
            rules: FeedbackRules::default(),
        }
    }

    /// 记录一个 error event（带 secret 脱敏）。
    pub fn record_event(&mut self, mut event: ErrorEvent) {
        // 存储前对 message 进行 secret 脱敏
        let redactor = SecretRedactor::new();
        let (redacted_msg, redactions) = redactor.redact(&event.message);
        if redactions > 0 {
            event.message = redacted_msg;
        }
        // 也对其他文本字段进行脱敏
        self.events.push(event);
        self.check_and_generate_lessons();
    }

    /// 检查 error 模式是否需要生成新的 constraint lesson。
    fn check_and_generate_lessons(&mut self) {
        // 按 error_type 分组
        let mut groups: std::collections::HashMap<String, Vec<&ErrorEvent>> = std::collections::HashMap::new();
        for event in &self.events {
            groups.entry(event.error_type.clone()).or_default().push(event);
        }

        for (error_type, events) in &groups {
            let warning_count = events.iter().filter(|e| e.severity == Severity::Warning).count();
            let critical_count = events.iter().filter(|e| e.severity == Severity::Critical).count();

            // 如果该 error 类型已有 lesson 则跳过
            if self.lessons.iter().any(|l| l.rule.contains(error_type)) {
                continue;
            }

            if critical_count >= self.rules.critical_threshold || warning_count >= self.rules.warning_threshold {
                let lesson = ConstraintLesson {
                    id: uuid::Uuid::new_v4().to_string(),
                    rule: format!("Avoid {}", error_type),
                    reason: format!(
                        "Error type '{}' occurred {} times ({} critical, {} warnings)",
                        error_type,
                        events.len(),
                        critical_count,
                        warning_count
                    ),
                    source_errors: events.iter().map(|e| e.id.clone()).collect(),
                    active: true,
                    created_at: chrono::Utc::now().to_rfc3339(),
                };
                tracing::info!(error_type = %error_type, lesson_id = %lesson.id, "Generated constraint lesson");
                self.lessons.push(lesson);
            }
        }
    }

    /// 获取用于 prompt 注入的活跃 lesson。
    pub fn active_lessons(&self) -> Vec<&ConstraintLesson> {
        self.lessons.iter()
            .filter(|l| l.active)
            .take(self.rules.max_active_lessons)
            .collect()
    }

    /// 将 lesson 格式化为 prompt 文本。
    pub fn format_lessons_for_prompt(&self) -> String {
        let lessons = self.active_lessons();
        if lessons.is_empty() {
            return String::new();
        }
        let rules: Vec<String> = lessons.iter()
            .map(|l| format!("- {}", l.rule))
            .collect();
        format!(
            "## Learned Constraints (from past errors)\n{}\n",
            rules.join("\n")
        )
    }

    /// 获取所有 event。
    pub fn events(&self) -> &[ErrorEvent] {
        &self.events
    }

    /// 获取所有 lesson。
    pub fn lessons(&self) -> &[ConstraintLesson] {
        &self.lessons
    }

    /// 停用一个 lesson。
    pub fn deactivate_lesson(&mut self, lesson_id: &str) -> bool {
        if let Some(lesson) = self.lessons.iter_mut().find(|l| l.id == lesson_id) {
            lesson.active = false;
            true
        } else {
            false
        }
    }

    /// 清理旧 event（保留最近 N 条）。
    pub fn prune_events(&mut self, keep: usize) {
        if self.events.len() > keep {
            let drain_count = self.events.len() - keep;
            self.events.drain(..drain_count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_event_and_generate_lesson() {
        let mut store = FeedbackStore::new();
        // 记录 2 个同类型的 critical event
        for i in 0..2 {
            store.record_event(ErrorEvent {
                id: format!("e{}", i),
                agent: "writer".to_string(),
                error_type: "token_limit".to_string(),
                message: "Token limit exceeded".to_string(),
                chapter: Some(1),
                book_id: Some("b1".to_string()),
                timestamp: chrono::Utc::now().to_rfc3339(),
                severity: Severity::Critical,
            });
        }
        assert_eq!(store.lessons().len(), 1);
        assert!(store.active_lessons().len() == 1);
    }

    #[test]
    fn test_format_lessons() {
        let mut store = FeedbackStore::new();
        store.record_event(ErrorEvent {
            id: "e1".into(),
            agent: "writer".into(),
            error_type: "hallucination".into(),
            message: "Found hallucinated fact".into(),
            chapter: None,
            book_id: None,
            timestamp: "".into(),
            severity: Severity::Critical,
        });
        store.record_event(ErrorEvent {
            id: "e2".into(),
            agent: "writer".into(),
            error_type: "hallucination".into(),
            message: "Found hallucinated fact".into(),
            chapter: None,
            book_id: None,
            timestamp: "".into(),
            severity: Severity::Critical,
        });
        let text = store.format_lessons_for_prompt();
        assert!(text.contains("hallucination"));
    }
}
