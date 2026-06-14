use crate::errors::AppError;
use crate::infra::db::Database;
use super::types::*;

pub struct FeedbackLoop;

impl FeedbackLoop {
    pub fn should_generate_lesson<'a>(
        error_type: &str,
        occurrence_count: u32,
        rules: &'a [FeedbackRule],
    ) -> Option<&'a FeedbackRule> {
        rules.iter().find(|r| {
            r.trigger.error_type == error_type && occurrence_count >= r.trigger.min_occurrences
        })
    }

    pub fn generate_lesson_text(
        _error_type: &str,
        descriptions: &[String],
        count: u32,
        rule: &FeedbackRule,
    ) -> String {
        let summary = if descriptions.len() >= 3 {
            descriptions.iter().take(3).cloned().collect::<Vec<_>>().join("; ")
        } else {
            descriptions.join("; ")
        };
        format!(
            "{} (基于 {} 条错误: {}) [已出现 {} 次，务必遵守]",
            rule.constraint, descriptions.len(), summary, count
        )
    }

    pub fn create_lesson(
        novel_id: &str,
        chapter_number: u32,
        error_type: &str,
        descriptions: &[String],
        count: u32,
        rule: &FeedbackRule,
    ) -> ConstraintLesson {
        let constraint_text = Self::generate_lesson_text(error_type, descriptions, count, rule);
        ConstraintLesson {
            id: uuid::Uuid::new_v4().to_string(),
            novel_id: novel_id.to_string(),
            chapter_number,
            error_type: error_type.to_string(),
            description: format!("基于 {} 条同类错误生成的约束", descriptions.len()),
            constraint_added: constraint_text,
            active: true,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn record_error(
        novel_id: &str,
        chapter_number: u32,
        agent_role: &str,
        error_type: &str,
        dimension: Option<&str>,
        severity: &str,
        description: &str,
        _suggestion: Option<&str>,
        rules: &[FeedbackRule],
        db: &Database,
    ) -> Result<Option<ConstraintLesson>, AppError> {
        tracing::info!(
            novel_id,
            chapter = chapter_number,
            agent_role,
            error_type,
            severity,
            "Recording error for feedback loop"
        );
        let event_id = uuid::Uuid::new_v4().to_string();
        let event = ErrorEvent {
            id: event_id,
            novel_id: novel_id.to_string(),
            chapter_number,
            agent_role: agent_role.to_string(),
            error_type: error_type.to_string(),
            dimension: dimension.map(|s| s.to_string()),
            severity: severity.to_string(),
            description: description.to_string(),
            suggestion: None,
            lesson_id: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        db.insert_error_event(&event)?;

        let count = db.count_error_events(novel_id, error_type)?;

        if let Some(rule) = Self::should_generate_lesson(error_type, count, rules) {
            if let Some(_existing) = db.find_active_lesson(novel_id, error_type)? {
                tracing::debug!(
                    novel_id,
                    error_type,
                    count,
                    "Updating existing lesson stats"
                );
                db.update_lesson_stats(&_existing.id, count, chapter_number)?;
                return Ok(None);
            }

            let events = db.list_error_events_by_type(novel_id, error_type)?;
            let descriptions: Vec<String> = events.iter().map(|e| e.description.clone()).collect();

            tracing::info!(
                novel_id,
                error_type,
                count,
                error_count = descriptions.len(),
                "Generating new constraint lesson"
            );

            let lesson = Self::create_lesson(novel_id, chapter_number, error_type, &descriptions, count, rule);
            db.insert_lesson(&lesson)?;

            for evt in &events {
                db.link_event_to_lesson(&evt.id, &lesson.id)?;
            }

            return Ok(Some(lesson));
        }

        Ok(None)
    }

    pub fn get_active_lessons(novel_id: &str, db: &Database) -> Result<Vec<ConstraintLesson>, AppError> {
        db.list_active_lessons(novel_id)
    }

    pub fn format_lessons_for_prompt(lessons: &[ConstraintLesson]) -> String {
        if lessons.is_empty() { return String::new(); }
        let mut section = String::from("\n## 约束教训（必须遵守）\n\n");
        for (i, lesson) in lessons.iter().enumerate() {
            section.push_str(&format!("{}. {}\n", i + 1, lesson.constraint_added));
        }
        section
    }

    pub fn suspend_lesson(lesson_id: &str, db: &Database) -> Result<(), AppError> {
        db.update_lesson_state(lesson_id, "suppressed")
    }

    pub fn resume_lesson(lesson_id: &str, db: &Database) -> Result<(), AppError> {
        db.update_lesson_state(lesson_id, "active")
    }

    pub fn archive_lesson(lesson_id: &str, db: &Database) -> Result<(), AppError> {
        db.archive_lesson(lesson_id)
    }

    pub fn auto_archive_lessons(
        novel_id: &str,
        current_chapter: u32,
        silence_threshold: u32,
        db: &Database,
    ) -> Result<u32, AppError> {
        let lessons = db.list_active_lessons(novel_id)?;
        let mut archived_count = 0;
        for lesson in lessons {
            let recent_errors = db.count_error_events_after(
                novel_id, &lesson.error_type, lesson.chapter_number, current_chapter,
            )?;
            if recent_errors == 0 && (current_chapter - lesson.chapter_number) >= silence_threshold {
                db.archive_lesson(&lesson.id)?;
                archived_count += 1;
            }
        }
        Ok(archived_count)
    }
}
