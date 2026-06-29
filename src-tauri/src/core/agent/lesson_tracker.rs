//! Lesson Tracker — Agent self-evolution via error-correction learning.
//!
//! Aligned with Hermes Agent's feedback loop:
//! 1. Audit finds issues → record as error events
//! 2. Error type reaches threshold → generate constraint lesson
//! 3. Active lessons injected into agent prompts via MEMORY.md
//!
//! Each agent accumulates lessons across pipeline runs. Lessons are written
//! to `agents/<role>/MEMORY.md` and read back via `AgentIdentity::load()`.

use serde::{Serialize, Deserialize};
use crate::shared::errors::AppError;
use crate::infrastructure::file_storage::data_dir::DataDir;
use crate::features::story::AuditResult;

/// A single constraint lesson derived from repeated audit issues.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintLesson {
    /// The agent role this lesson applies to (e.g. "writer", "planner")
    pub agent_role: String,
    /// The audit dimension the issue belongs to (e.g. "OOC", "timeline")
    pub dimension: String,
    /// The lesson text — what the agent should avoid/do
    pub lesson: String,
    /// How many times this issue type was observed
    pub occurrence_count: u32,
    /// ISO 8601 timestamp of when the lesson was generated
    pub generated_at: String,
    /// Whether this lesson is currently active (injected into prompts)
    pub active: bool,
    /// ISO 8601 timestamp of when this lesson was last triggered
    #[serde(default)]
    pub last_triggered: String,
}

/// Tracks audit issues across chapters and generates constraint lessons
/// when an error type exceeds its threshold.
pub struct LessonTracker {
    /// Lessons by agent role
    lessons: Vec<ConstraintLesson>,
    /// Threshold: how many occurrences before a lesson is generated
    threshold: u32,
    /// Maximum active lessons per agent (prevent prompt bloat)
    max_active_per_agent: u32,
}

impl LessonTracker {
    pub fn new(threshold: u32, max_active_per_agent: u32) -> Self {
        Self {
            lessons: Vec::new(),
            threshold,
            max_active_per_agent,
        }
    }

    pub fn default_config() -> Self {
        Self::new(3, 10)
    }

    /// Record audit issues from a chapter and generate lessons if threshold is met.
    ///
    /// `agent_role` is the agent that produced the content (e.g. "writer").
    /// `audit` is the audit result for that chapter.
    pub fn record_audit(
        &mut self,
        agent_role: &str,
        audit: &AuditResult,
    ) -> Vec<ConstraintLesson> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut new_lessons = Vec::new();

        for issue in &audit.issues {
            let dimension = issue.category.clone();
            let description = issue.description.clone();
            let suggestion = issue.suggestion.clone();

            // Check if we already have a lesson for this dimension
            let existing = self.lessons.iter_mut().find(|l| {
                l.agent_role == agent_role && l.dimension == dimension
            });

            if let Some(lesson) = existing {
                lesson.occurrence_count += 1;
                lesson.last_triggered = now.clone();
                if lesson.occurrence_count >= self.threshold && !lesson.active {
                    lesson.active = true;
                    lesson.lesson = if suggestion.is_empty() { description } else { suggestion };
                    new_lessons.push(lesson.clone());
                }
            } else {
                let lesson_text = if suggestion.is_empty() { description } else { suggestion };
                let mut new_lesson = ConstraintLesson {
                    agent_role: agent_role.to_string(),
                    dimension: dimension.clone(),
                    lesson: lesson_text,
                    occurrence_count: 1,
                    generated_at: now.clone(),
                    active: false,
                    last_triggered: now.clone(),
                };
                // If threshold is 1, activate immediately
                if self.threshold <= 1 {
                    new_lesson.active = true;
                    new_lessons.push(new_lesson.clone());
                }
                self.lessons.push(new_lesson);
            }
        }

        new_lessons
    }

    /// Get active lessons for a specific agent role.
    pub fn active_lessons_for(&self, agent_role: &str) -> Vec<&ConstraintLesson> {
        self.lessons.iter()
            .filter(|l| l.agent_role == agent_role && l.active)
            .take(self.max_active_per_agent as usize)
            .collect()
    }

    /// Format active lessons as a prompt-injectable block.
    ///
    /// This is meant to be appended to the agent's MEMORY.md content
    /// or injected directly into the system prompt.
    pub fn format_lessons_block(&self, agent_role: &str) -> String {
        let lessons = self.active_lessons_for(agent_role);
        if lessons.is_empty() {
            return String::new();
        }

        let mut block = String::from("## Constraint Lessons (auto-generated from audit feedback)\n\n");
        block.push_str("The following issues have been repeatedly flagged. Avoid them:\n\n");
        for lesson in &lessons {
            block.push_str(&format!(
                "- **[{}]** ({} occurrences) {}\n",
                lesson.dimension, lesson.occurrence_count, lesson.lesson
            ));
        }
        block.push('\n');
        block
    }

    /// Deactivate lessons that haven't been triggered within `stale_days`.
    ///
    /// Returns the number of lessons deactivated.
    pub fn deactivate_stale_lessons(&mut self, stale_days: u64) -> usize {
        let now = chrono::Utc::now();
        let mut count = 0;
        for lesson in &mut self.lessons {
            if !lesson.active {
                continue;
            }
            let ts = if lesson.last_triggered.is_empty() {
                &lesson.generated_at
            } else {
                &lesson.last_triggered
            };
            if let Ok(last) = chrono::DateTime::parse_from_rfc3339(ts) {
                let elapsed = now.signed_duration_since(last);
                if elapsed.num_days() > stale_days as i64 {
                    lesson.active = false;
                    count += 1;
                }
            }
        }
        count
    }

    /// Get all lessons (for persistence/serialization).
    pub fn all_lessons(&self) -> &[ConstraintLesson] {
        &self.lessons
    }

    /// Load lessons from serialized data.
    pub fn load_lessons(&mut self, lessons: Vec<ConstraintLesson>) {
        self.lessons = lessons;
    }
}

/// Update an agent's MEMORY.md with new lessons.
///
/// This reads the existing MEMORY.md, appends new lessons, and writes it back.
/// Existing content is preserved — lessons are appended at the end.
pub fn append_lessons_to_memory(
    data_dir: &DataDir,
    agent_role: &str,
    new_lessons: &[ConstraintLesson],
) -> Result<(), AppError> {
    if new_lessons.is_empty() {
        return Ok(());
    }

    let memory_path = data_dir.agent_memory_path(agent_role);
    let existing = std::fs::read_to_string(&memory_path).unwrap_or_default();

    // Remove old constraint lessons section if present
    let cleaned = remove_lessons_section(&existing);

    // Build new lessons section
    let mut lessons_section = String::from("\n\n## Constraint Lessons (auto-generated)\n\n");
    for lesson in new_lessons {
        lessons_section.push_str(&format!(
            "- **[{}]** ({}x) {}\n",
            lesson.dimension, lesson.occurrence_count, lesson.lesson
        ));
    }

    let updated = format!("{}{}", cleaned.trim(), lessons_section);
    std::fs::write(&memory_path, updated)
        .map_err(|e| AppError::internal(format!("Failed to write MEMORY.md for {}: {}", agent_role, e)))?;

    Ok(())
}

/// Remove the auto-generated lessons section from MEMORY.md content.
fn remove_lessons_section(content: &str) -> String {
    let marker_start = "## Constraint Lessons (auto-generated)";
    if let Some(pos) = content.find(marker_start) {
        content[..pos].trim().to_string()
    } else {
        content.to_string()
    }
}

/// Load lessons from an agent's MEMORY.md file.
///
/// Parses the `## Constraint Lessons` section back into `ConstraintLesson` structs.
pub fn load_lessons_from_memory(
    data_dir: &DataDir,
    agent_role: &str,
) -> Vec<ConstraintLesson> {
    let memory_path = data_dir.agent_memory_path(agent_role);
    let content = std::fs::read_to_string(&memory_path).unwrap_or_default();

    let marker_start = "## Constraint Lessons (auto-generated)";
    let marker_end = "\n## "; // Next section starts

    let Some(start) = content.find(marker_start) else {
        return Vec::new();
    };

    let section = if let Some(end) = content[start + marker_start.len()..].find(marker_end) {
        &content[start..start + marker_start.len() + end]
    } else {
        &content[start..]
    };

    let mut lessons = Vec::new();
    for line in section.lines() {
        if line.starts_with("- **[") {
            // Parse: "- **[dimension]** (Nx) lesson text"
            if let Some(dim_end) = line.find("]**") {
                let dimension = &line[5..dim_end];
                let rest = &line[dim_end + 3..].trim();
                // Skip the " (Nx) " part
                if let Some(count_end) = rest.find(") ") {
                    let count_str = &rest[2..count_end];
                    let count: u32 = count_str.parse().unwrap_or(1);
                    let lesson_text = &rest[count_end + 2..];
                    lessons.push(ConstraintLesson {
                        agent_role: agent_role.to_string(),
                        dimension: dimension.to_string(),
                        lesson: lesson_text.to_string(),
                        occurrence_count: count,
                        generated_at: String::new(),
                        active: true,
                        last_triggered: String::new(),
                    });
                }
            }
        }
    }

    lessons
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_audit(dimension: &str, description: &str, suggestion: &str) -> AuditResult {
        AuditResult {
            passed: false,
            score: 50.0,
            issues: vec![crate::features::story::AuditIssue {
                category: dimension.to_string(),
                severity: crate::features::story::AuditSeverity::Critical,
                description: description.to_string(),
                suggestion: suggestion.to_string(),
            }],
            summary: "Test audit".to_string(),
        }
    }

    #[test]
    fn test_lesson_tracker_threshold() {
        let mut tracker = LessonTracker::new(2, 10);
        let audit = make_audit("OOC", "Character acts out of character", "Stay consistent with character profile");

        // First occurrence — no lesson generated
        let new = tracker.record_audit("writer", &audit);
        assert!(new.is_empty());
        assert_eq!(tracker.active_lessons_for("writer").len(), 0);

        // Second occurrence — threshold met, lesson generated
        let new = tracker.record_audit("writer", &audit);
        assert_eq!(new.len(), 1);
        assert_eq!(new[0].dimension, "OOC");
        assert!(new[0].active);
        assert_eq!(tracker.active_lessons_for("writer").len(), 1);
    }

    #[test]
    fn test_lessons_block_formatting() {
        let mut tracker = LessonTracker::new(1, 10);
        let audit = make_audit("timeline", "Timeline inconsistency", "Track time passage explicitly");

        tracker.record_audit("writer", &audit);
        let block = tracker.format_lessons_block("writer");
        assert!(block.contains("timeline"));
        assert!(block.contains("Track time passage"));
    }

    #[test]
    fn test_remove_lessons_section() {
        let content = "# Agent Memory\n\nSome notes here.\n\n## Constraint Lessons (auto-generated)\n\n- **[OOC]** (3x) Stay consistent\n";
        let cleaned = remove_lessons_section(content);
        assert!(!cleaned.contains("Constraint Lessons"));
        assert!(cleaned.contains("Some notes here"));
    }

    #[test]
    fn test_deactivate_stale_lessons() {
        let mut tracker = LessonTracker::new(1, 10);
        let audit = make_audit("OOC", "Character acts out of character", "Stay consistent");

        tracker.record_audit("writer", &audit);
        assert_eq!(tracker.active_lessons_for("writer").len(), 1);

        // With 0 stale_days, the lesson (just created) should NOT be deactivated
        // because 0 days have passed.
        let deactivated = tracker.deactivate_stale_lessons(0);
        assert_eq!(deactivated, 0);
        assert_eq!(tracker.active_lessons_for("writer").len(), 1);
    }
}
