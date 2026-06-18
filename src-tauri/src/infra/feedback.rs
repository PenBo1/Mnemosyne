use serde::{Deserialize, Serialize};
use crate::infra::security::SecretRedactor;

/// A recorded error event from pipeline execution.
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

/// A constraint lesson derived from repeated error patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintLesson {
    pub id: String,
    pub rule: String,
    pub reason: String,
    pub source_errors: Vec<String>,
    pub active: bool,
    pub created_at: String,
}

/// Configuration for when to generate lessons from errors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRules {
    /// Max warning events before generating a lesson
    pub warning_threshold: usize,
    /// Max critical events before generating a lesson
    pub critical_threshold: usize,
    /// Max active lessons to inject into prompts
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

/// In-memory feedback store.
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

    /// Record an error event (with secret redaction).
    pub fn record_event(&mut self, mut event: ErrorEvent) {
        // Redact secrets from message before storage
        let redactor = SecretRedactor::new();
        let (redacted_msg, redactions) = redactor.redact(&event.message);
        if redactions > 0 {
            event.message = redacted_msg;
        }
        // Also redact any other text fields
        self.events.push(event);
        self.check_and_generate_lessons();
    }

    /// Check if error patterns warrant new constraint lessons.
    fn check_and_generate_lessons(&mut self) {
        // Group events by error_type
        let mut groups: std::collections::HashMap<String, Vec<&ErrorEvent>> = std::collections::HashMap::new();
        for event in &self.events {
            groups.entry(event.error_type.clone()).or_default().push(event);
        }

        for (error_type, events) in &groups {
            let warning_count = events.iter().filter(|e| e.severity == Severity::Warning).count();
            let critical_count = events.iter().filter(|e| e.severity == Severity::Critical).count();

            // Skip if lesson already exists for this error type
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

    /// Get active lessons for prompt injection.
    pub fn active_lessons(&self) -> Vec<&ConstraintLesson> {
        self.lessons.iter()
            .filter(|l| l.active)
            .take(self.rules.max_active_lessons)
            .collect()
    }

    /// Format lessons as prompt text.
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

    /// Get all events.
    pub fn events(&self) -> &[ErrorEvent] {
        &self.events
    }

    /// Get all lessons.
    pub fn lessons(&self) -> &[ConstraintLesson] {
        &self.lessons
    }

    /// Deactivate a lesson.
    pub fn deactivate_lesson(&mut self, lesson_id: &str) -> bool {
        if let Some(lesson) = self.lessons.iter_mut().find(|l| l.id == lesson_id) {
            lesson.active = false;
            true
        } else {
            false
        }
    }

    /// Clear old events (keep last N).
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
        // Record 2 critical events of same type
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
