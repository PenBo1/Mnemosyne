use serde::{Deserialize, Serialize};

/// Runtime state delta produced by the Settler/Reflector.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeStateDelta {
    pub chapter: u32,
    pub hook_ops: Vec<HookOp>,
    pub facts_new: Vec<NewFact>,
    pub chapter_summary: Option<ChapterSummaryDelta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookOp {
    pub op: HookOpType,
    pub name: String,
    pub hook_type: Option<String>,
    pub status: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookOpType {
    Upsert,
    Mention,
    Resolve,
    Defer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewFact {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterSummaryDelta {
    pub chapter: u32,
    pub title: String,
    pub characters: Vec<String>,
    pub events: Vec<String>,
    pub state_changes: Vec<String>,
    pub hook_activity: Vec<String>,
    pub mood: String,
    pub chapter_type: String,
}

/// Runtime state snapshot — a point-in-time copy of all story state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStateSnapshot {
    pub chapter: u32,
    pub timestamp: String,
    pub current_state: String,
    pub pending_hooks: String,
    pub chapter_summaries: String,
    pub hooks_state: HooksState,
    pub facts_state: FactsState,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HooksState {
    pub hooks: Vec<HooksStateHook>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HooksStateHook {
    pub hook_id: String,
    pub name: String,
    pub status: String,
    pub start_chapter: u32,
    pub last_advanced_chapter: u32,
    pub expected_payoff: String,
    pub core_hook: bool,
    pub promoted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactsState {
    pub facts: Vec<FactEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactEntry {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub category: String,
    pub valid_from_chapter: u32,
}

/// Apply a delta to a snapshot, producing a new snapshot.
pub fn apply_delta_to_snapshot(
    snapshot: &RuntimeStateSnapshot,
    delta: &RuntimeStateDelta,
) -> RuntimeStateSnapshot {
    let mut new_snapshot = snapshot.clone();
    new_snapshot.chapter = delta.chapter;
    new_snapshot.timestamp = chrono::Utc::now().to_rfc3339();

    // Apply hook operations
    for op in &delta.hook_ops {
        match op.op {
            HookOpType::Upsert => {
                if let Some(existing) = new_snapshot.hooks_state.hooks.iter_mut()
                    .find(|h| h.name == op.name)
                {
                    existing.last_advanced_chapter = delta.chapter;
                    if let Some(status) = &op.status {
                        existing.status = status.clone();
                    }
                    if let Some(desc) = &op.description {
                        if !desc.is_empty() {
                            existing.expected_payoff = desc.clone();
                        }
                    }
                } else {
                    let _hook_type = op.hook_type.clone().unwrap_or_else(|| "foreshadowing".to_string());
                    new_snapshot.hooks_state.hooks.push(HooksStateHook {
                        hook_id: uuid::Uuid::new_v4().to_string(),
                        name: op.name.clone(),
                        status: op.status.clone().unwrap_or_else(|| "open".to_string()),
                        start_chapter: delta.chapter,
                        last_advanced_chapter: delta.chapter,
                        expected_payoff: op.description.clone().unwrap_or_default(),
                        core_hook: false,
                        promoted: false,
                    });
                }
            }
            HookOpType::Mention => {
                if let Some(hook) = new_snapshot.hooks_state.hooks.iter_mut()
                    .find(|h| h.name == op.name)
                {
                    hook.last_advanced_chapter = delta.chapter;
                }
            }
            HookOpType::Resolve => {
                if let Some(hook) = new_snapshot.hooks_state.hooks.iter_mut()
                    .find(|h| h.name == op.name)
                {
                    hook.status = "resolved".to_string();
                    hook.last_advanced_chapter = delta.chapter;
                }
            }
            HookOpType::Defer => {
                if let Some(hook) = new_snapshot.hooks_state.hooks.iter_mut()
                    .find(|h| h.name == op.name)
                {
                    hook.status = "deferred".to_string();
                    hook.last_advanced_chapter = delta.chapter;
                }
            }
        }
    }

    // Add new facts
    for fact in &delta.facts_new {
        new_snapshot.facts_state.facts.push(FactEntry {
            subject: fact.subject.clone(),
            predicate: fact.predicate.clone(),
            object: fact.object.clone(),
            category: fact.category.clone(),
            valid_from_chapter: delta.chapter,
        });
    }

    new_snapshot
}
