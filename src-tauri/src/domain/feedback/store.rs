
use super::types::{ErrorEvent, Severity, ConstraintLesson, FeedbackRules};

pub struct SecretRedactor { patterns: Vec<&'static str> }

impl SecretRedactor {
    pub fn new() -> Self { Self { patterns: vec!["api_key", "secret", "password", "token", "credential"] } }
    pub fn redact(&self, text: &str) -> (String, usize) {
        let mut redacted = text.to_string();
        let mut count = 0;
        for pattern in &self.patterns { if redacted.contains(pattern) { redacted = redacted.replace(pattern, "[REDACTED]"); count += 1; } }
        (redacted, count)
    }
}

pub struct FeedbackStore {
    events: Vec<ErrorEvent>,
    lessons: Vec<ConstraintLesson>,
    rules: FeedbackRules,
}

impl FeedbackStore {
    pub fn new() -> Self { Self { events: Vec::new(), lessons: Vec::new(), rules: FeedbackRules::default() } }

    pub fn record_event(&mut self, mut event: ErrorEvent) {
        let redactor = SecretRedactor::new();
        let (redacted_msg, redactions) = redactor.redact(&event.message);
        if redactions > 0 { event.message = redacted_msg; }
        self.events.push(event);
        self.check_and_generate_lessons();
    }

    fn check_and_generate_lessons(&mut self) {
        let mut groups: std::collections::HashMap<String, Vec<&ErrorEvent>> = std::collections::HashMap::new();
        for event in &self.events { groups.entry(event.error_type.clone()).or_default().push(event); }
        for (error_type, events) in &groups {
            let warning_count = events.iter().filter(|e| e.severity == Severity::Warning).count();
            let critical_count = events.iter().filter(|e| e.severity == Severity::Critical).count();
            if self.lessons.iter().any(|l| l.rule.contains(error_type)) { continue; }
            if critical_count >= self.rules.critical_threshold || warning_count >= self.rules.warning_threshold {
                let lesson = ConstraintLesson {
                    id: uuid::Uuid::new_v4().to_string(),
                    rule: format!("Avoid {}", error_type),
                    reason: format!("Error type '{}' occurred {} times ({} critical, {} warnings)", error_type, events.len(), critical_count, warning_count),
                    source_errors: events.iter().map(|e| e.id.clone()).collect(),
                    active: true,
                    created_at: chrono::Utc::now().to_rfc3339(),
                };
                tracing::info!(error_type = %error_type, lesson_id = %lesson.id, "Generated constraint lesson");
                self.lessons.push(lesson);
            }
        }
    }

    pub fn active_lessons(&self) -> Vec<&ConstraintLesson> { self.lessons.iter().filter(|l| l.active).take(self.rules.max_active_lessons).collect() }

    pub fn format_lessons_for_prompt(&self) -> String {
        let lessons = self.active_lessons();
        if lessons.is_empty() { return String::new(); }
        let rules: Vec<String> = lessons.iter().map(|l| format!("- {}", l.rule)).collect();
        format!("## Learned Constraints (from past errors)\n{}\n", rules.join("\n"))
    }

    pub fn events(&self) -> &[ErrorEvent] { &self.events }
    pub fn lessons(&self) -> &[ConstraintLesson] { &self.lessons }

    pub fn deactivate_lesson(&mut self, lesson_id: &str) -> bool {
        if let Some(lesson) = self.lessons.iter_mut().find(|l| l.id == lesson_id) { lesson.active = false; true } else { false }
    }

    pub fn prune_events(&mut self, keep: usize) { if self.events.len() > keep { let drain_count = self.events.len() - keep; self.events.drain(..drain_count); } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_event_and_generate_lesson() {
        let mut store = FeedbackStore::new();
        for i in 0..2 {
            store.record_event(ErrorEvent {
                id: format!("e{}", i), agent: "writer".to_string(), error_type: "token_limit".to_string(),
                message: "Token limit exceeded".to_string(), chapter: Some(1), book_id: Some("b1".to_string()),
                timestamp: chrono::Utc::now().to_rfc3339(), severity: Severity::Critical,
            });
        }
        assert_eq!(store.lessons().len(), 1);
        assert!(store.active_lessons().len() == 1);
    }

    #[test]
    fn test_format_lessons() {
        let mut store = FeedbackStore::new();
        store.record_event(ErrorEvent { id: "e1".into(), agent: "writer".into(), error_type: "hallucination".into(), message: "Found hallucinated fact".into(), chapter: None, book_id: None, timestamp: "".into(), severity: Severity::Critical });
        store.record_event(ErrorEvent { id: "e2".into(), agent: "writer".into(), error_type: "hallucination".into(), message: "Found hallucinated fact".into(), chapter: None, book_id: None, timestamp: "".into(), severity: Severity::Critical });
        let text = store.format_lessons_for_prompt();
        assert!(text.contains("hallucination"));
    }
}