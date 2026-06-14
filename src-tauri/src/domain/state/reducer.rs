use crate::domain::agents::types::{StoryState, RuntimeStateDelta, HookOpType, HookStatus, HookRecord, TemporalFact};
use uuid::Uuid;
use chrono::Utc;

/// Apply a RuntimeStateDelta to a StoryState immutably.
///
/// This is the core state transition function. It processes hook operations,
/// adds new facts, and updates the chapter summary.
pub fn apply_delta(state: &mut StoryState, delta: &RuntimeStateDelta, chapter: u32) {
    // Process hook operations
    for op in &delta.hook_ops {
        match op.op {
            HookOpType::Upsert => {
                if let Some(existing) = state.hooks.iter_mut().find(|h| h.name == op.name) {
                    // Update existing hook
                    existing.last_advanced_chapter = chapter;
                    existing.updated_at = Utc::now().to_rfc3339();
                    if let Some(status) = &op.status {
                        existing.status = status.clone();
                    }
                    if let Some(desc) = &op.description {
                        if !desc.is_empty() {
                            existing.expected_payoff = desc.clone();
                        }
                    }
                } else {
                    // Create new hook
                    let hook_type = op.hook_type.clone().unwrap_or_else(|| "foreshadowing".to_string());
                    state.hooks.push(HookRecord {
                        hook_id: Uuid::new_v4().to_string(),
                        name: op.name.clone(),
                        hook_type,
                        start_chapter: chapter,
                        status: op.status.clone().unwrap_or_default(),
                        expected_payoff: op.description.clone().unwrap_or_default(),
                        last_advanced_chapter: chapter,
                        core_hook: false,
                        created_at: Utc::now().to_rfc3339(),
                        updated_at: Utc::now().to_rfc3339(),
                    });
                }
            }
            HookOpType::Mention => {
                if let Some(hook) = state.hooks.iter_mut().find(|h| h.name == op.name) {
                    hook.last_advanced_chapter = chapter;
                    hook.updated_at = Utc::now().to_rfc3339();
                }
            }
            HookOpType::Resolve => {
                if let Some(hook) = state.hooks.iter_mut().find(|h| h.name == op.name) {
                    hook.status = HookStatus::Resolved;
                    hook.last_advanced_chapter = chapter;
                    hook.updated_at = Utc::now().to_rfc3339();
                }
            }
            HookOpType::Defer => {
                if let Some(hook) = state.hooks.iter_mut().find(|h| h.name == op.name) {
                    hook.status = HookStatus::Deferred;
                    hook.last_advanced_chapter = chapter;
                    hook.updated_at = Utc::now().to_rfc3339();
                }
            }
        }
    }

    // Add new facts
    for fact in &delta.facts_new {
        state.facts.push(TemporalFact {
            fact_id: Uuid::new_v4().to_string(),
            subject: fact.subject.clone(),
            predicate: fact.predicate.clone(),
            object: fact.object.clone(),
            category: fact.category.clone(),
            valid_from_chapter: chapter,
            valid_until_chapter: None,
            source_chapter: chapter,
            created_at: Utc::now().to_rfc3339(),
        });
    }

    // Update chapter summary
    if let Some(summary) = &delta.summary_new {
        state.summaries.push(summary.clone());
    }

    // Update chapter counter
    state.current_chapter = chapter;
}
