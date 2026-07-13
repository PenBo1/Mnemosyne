// RuntimeState Reducer。
//
// 核心职责：接收 settler agent 输出的 RuntimeStateDelta，应用到当前 RuntimeStateSnapshot，
// 产出新的 snapshot。包含 hookOps 合并、currentStatePatch 应用、chapterSummary 增删。
//
// 简化说明：完整版本的 applyHookOps 依赖 hook-arbiter（evaluateHookAdmission）做重复 family 检测。
// Rust 版暂用直接 upsert + merge 策略（admission 检查需要完整 hook-governance 子系统，
// 后续如需可补；当前 upsert 的语义是：同 hookId 合并，不同 hookId 直接插入）。

use super::super::types::Language;
use super::types::*;
use super::validator::{issues_to_error, validate_runtime_state};

/// 应用 delta 到 snapshot，返回新 snapshot。
pub fn apply_runtime_state_delta(
    snapshot: &RuntimeStateSnapshot,
    delta: &RuntimeStateDelta,
    allow_reapply: bool,
) -> Result<RuntimeStateSnapshot, String> {
    // 章节回退检查
    let goes_backwards = if allow_reapply {
        delta.chapter < snapshot.manifest.last_applied_chapter
    } else {
        delta.chapter <= snapshot.manifest.last_applied_chapter
    };
    if goes_backwards {
        return Err(format!(
            "delta chapter {} goes backwards (last applied: {})",
            delta.chapter, snapshot.manifest.last_applied_chapter
        ));
    }

    // chapterSummary chapter 必须匹配 delta.chapter
    if let Some(summary) = &delta.chapter_summary {
        if summary.chapter != delta.chapter {
            return Err(format!(
                "chapter summary {} does not match delta chapter {}",
                summary.chapter, delta.chapter
            ));
        }
    }

    // 重复 summary row 检查（非 reapply 模式）
    if let Some(summary) = &delta.chapter_summary {
        if !allow_reapply
            && snapshot
                .chapter_summaries
                .rows
                .iter()
                .any(|row| row.chapter == summary.chapter)
        {
            return Err(format!(
                "duplicate summary row for chapter {}",
                summary.chapter
            ));
        }
    }

    let hooks = apply_hook_ops(&snapshot.hooks, delta);
    let current_state =
        apply_current_state_patch(&snapshot.current_state, snapshot.manifest.language, delta);
    let chapter_summaries =
        apply_summary_delta(&snapshot.chapter_summaries, delta, allow_reapply);

    let next = RuntimeStateSnapshot {
        manifest: StateManifest {
            last_applied_chapter: delta.chapter,
            ..snapshot.manifest.clone()
        },
        current_state,
        hooks,
        chapter_summaries,
    };

    let issues = validate_runtime_state(&next);
    if !issues.is_empty() {
        return Err(issues_to_error(&issues));
    }

    Ok(next)
}

// ── Hook 操作 ────────────────────────────────────────────────

fn apply_hook_ops(hooks_state: &HooksState, delta: &RuntimeStateDelta) -> HooksState {
    let mut hooks_by_id: std::collections::HashMap<String, HookRecord> = hooks_state
        .hooks
        .iter()
        .map(|h| (h.hook_id.clone(), h.clone()))
        .collect();

    // upsert
    for hook in &delta.hook_ops.upsert {
        if let Some(existing) = hooks_by_id.get(&hook.hook_id) {
            let merged = merge_hook_record(existing, hook);
            hooks_by_id.insert(merged.hook_id.clone(), merged);
        } else {
            hooks_by_id.insert(hook.hook_id.clone(), hook.clone());
        }
    }

    // resolve
    for hook_id in &delta.hook_ops.resolve {
        if let Some(existing) = hooks_by_id.get(hook_id) {
            let mut updated = existing.clone();
            updated.status = HookStatus::Resolved;
            updated.last_advanced_chapter =
                updated.last_advanced_chapter.max(delta.chapter);
            hooks_by_id.insert(hook_id.clone(), updated);
        }
        // 不存在则跳过（可能已被清理）
    }

    // defer
    for hook_id in &delta.hook_ops.defer {
        if let Some(existing) = hooks_by_id.get(hook_id) {
            let mut updated = existing.clone();
            updated.status = HookStatus::Deferred;
            updated.last_advanced_chapter =
                updated.last_advanced_chapter.max(delta.chapter);
            hooks_by_id.insert(hook_id.clone(), updated);
        }
    }

    let mut hooks: Vec<HookRecord> = hooks_by_id.into_values().collect();
    hooks.sort_by(|a, b| {
        a.start_chapter
            .cmp(&b.start_chapter)
            .then(a.last_advanced_chapter.cmp(&b.last_advanced_chapter))
            .then(a.hook_id.cmp(&b.hook_id))
    });

    HooksState { hooks }
}

/// 合并同 hookId 的记录
fn merge_hook_record(existing: &HookRecord, incoming: &HookRecord) -> HookRecord {
    let expected_payoff = prefer_richer_text(&existing.expected_payoff, &incoming.expected_payoff);
    let notes = prefer_richer_text(&existing.notes, &incoming.notes);
    let advanced = existing.last_advanced_chapter.max(incoming.last_advanced_chapter);
    let progressed = advanced > existing.last_advanced_chapter;

    HookRecord {
        hook_id: existing.hook_id.clone(),
        start_chapter: existing.start_chapter.min(incoming.start_chapter),
        r#type: prefer_richer_text(&existing.r#type, &incoming.r#type),
        status: merge_hook_status(existing.status, incoming.status, progressed),
        last_advanced_chapter: advanced,
        expected_payoff,
        payoff_timing: incoming.payoff_timing.or(existing.payoff_timing),
        notes,
        depends_on: incoming.depends_on.clone().or_else(|| existing.depends_on.clone()),
        pays_off_in_arc: incoming.pays_off_in_arc.clone().or_else(|| existing.pays_off_in_arc.clone()),
        core_hook: incoming.core_hook.or(existing.core_hook),
        half_life_chapters: incoming.half_life_chapters.or(existing.half_life_chapters),
        advanced_count: incoming.advanced_count.or(existing.advanced_count),
        promoted: incoming.promoted.or(existing.promoted),
    }
}

fn merge_hook_status(existing: HookStatus, incoming: HookStatus, progressed: bool) -> HookStatus {
    if existing == HookStatus::Resolved || incoming == HookStatus::Resolved {
        return HookStatus::Resolved;
    }
    if progressed || existing == HookStatus::Progressing || incoming == HookStatus::Progressing {
        return HookStatus::Progressing;
    }
    existing
}

/// 选择更丰富的文本（非空优先，同等取更长）
fn prefer_richer_text(primary: &str, fallback: &str) -> String {
    let left = primary.trim();
    let right = fallback.trim();
    if left.is_empty() {
        return right.to_string();
    }
    if right.is_empty() {
        return left.to_string();
    }
    if left == right {
        return left.to_string();
    }
    if right.len() > left.len() {
        right.to_string()
    } else {
        left.to_string()
    }
}

// ── CurrentState 补丁 ────────────────────────────────────────

/// current_state_patch 各字段的 predicate 别名表
fn patch_aliases(language: Language) -> [(&'static str, [&'static str; 2]); 6] {
    match language {
        Language::En => [
            ("current_location", ["Current Location", "当前位置"]),
            ("protagonist_state", ["Protagonist State", "主角状态"]),
            ("current_goal", ["Current Goal", "当前目标"]),
            ("current_constraint", ["Current Constraint", "当前限制"]),
            ("current_alliances", ["Current Alliances", "当前敌我"]),
            ("current_conflict", ["Current Conflict", "当前冲突"]),
        ],
        Language::Zh => [
            ("current_location", ["当前位置", "Current Location"]),
            ("protagonist_state", ["主角状态", "Protagonist State"]),
            ("current_goal", ["当前目标", "Current Goal"]),
            ("current_constraint", ["当前限制", "Current Constraint"]),
            ("current_alliances", ["当前敌我", "Current Alliances"]),
            ("current_conflict", ["当前冲突", "Current Conflict"]),
        ],
    }
}

fn apply_current_state_patch(
    current_state: &CurrentStateState,
    language: Language,
    delta: &RuntimeStateDelta,
) -> CurrentStateState {
    let patch = match &delta.current_state_patch {
        None => {
            return CurrentStateState {
                chapter: delta.chapter,
                facts: current_state.facts.clone(),
            }
        }
        Some(p) => p,
    };

    let mut next_facts = current_state.facts.clone();
    let aliases = patch_aliases(language);

    let patch_values: [(&str, Option<&String>); 6] = [
        ("current_location", patch.current_location.as_ref()),
        ("protagonist_state", patch.protagonist_state.as_ref()),
        ("current_goal", patch.current_goal.as_ref()),
        ("current_constraint", patch.current_constraint.as_ref()),
        ("current_alliances", patch.current_alliances.as_ref()),
        ("current_conflict", patch.current_conflict.as_ref()),
    ];

    for (i, (_field_key, value_opt)) in patch_values.iter().enumerate() {
        let value = match value_opt {
            None => continue,
            Some(v) => v.as_str(),
        };
        let (_, alias_pair) = &aliases[i];
        let primary_alias = alias_pair[0];

        // 删除匹配 alias 的旧 fact（倒序删除）
        for index in (0..next_facts.len()).rev() {
            let predicate = &next_facts[index].predicate;
            let matches = alias_pair
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(predicate));
            if matches {
                next_facts.remove(index);
            }
        }

        next_facts.push(CurrentStateFact {
            subject: "protagonist".into(),
            predicate: primary_alias.to_string(),
            object: value.to_string(),
            valid_from_chapter: delta.chapter,
            valid_until_chapter: None,
            source_chapter: delta.chapter,
        });
    }

    next_facts.sort_by(|a, b| {
        a.predicate
            .cmp(&b.predicate)
            .then(a.object.cmp(&b.object))
    });

    CurrentStateState {
        chapter: delta.chapter,
        facts: next_facts,
    }
}

// ── ChapterSummary 增删 ─────────────────────────────────────

fn apply_summary_delta(
    state: &ChapterSummariesState,
    delta: &RuntimeStateDelta,
    allow_reapply: bool,
) -> ChapterSummariesState {
    let mut rows = if let Some(summary) = &delta.chapter_summary {
        if allow_reapply {
            // reapply 模式：先过滤掉同 chapter 的旧 row，再 push 新的
            state
                .rows
                .iter()
                .filter(|row| row.chapter != summary.chapter)
                .cloned()
                .collect::<Vec<_>>()
        } else {
            state.rows.clone()
        }
    } else {
        state.rows.clone()
    };

    if let Some(summary) = &delta.chapter_summary {
        rows.push(summary.clone());
    }

    rows.sort_by(|a, b| a.chapter.cmp(&b.chapter));
    ChapterSummariesState { rows }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_delta(chapter: u32) -> RuntimeStateDelta {
        RuntimeStateDelta {
            chapter,
            current_state_patch: None,
            hook_ops: HookOps::default(),
            new_hook_candidates: Vec::new(),
            chapter_summary: None,
            subplot_ops: Vec::new(),
            emotional_arc_ops: Vec::new(),
            character_matrix_ops: Vec::new(),
            notes: Vec::new(),
        }
    }

    #[test]
    fn applies_empty_delta_updates_chapter() {
        let snapshot = RuntimeStateSnapshot::empty(Language::Zh);
        let delta = make_delta(1);
        let next = apply_runtime_state_delta(&snapshot, &delta, false).unwrap();
        assert_eq!(next.manifest.last_applied_chapter, 1);
        assert_eq!(next.current_state.chapter, 1);
    }

    #[test]
    fn rejects_backwards_chapter() {
        let mut snapshot = RuntimeStateSnapshot::empty(Language::Zh);
        snapshot.manifest.last_applied_chapter = 5;
        let delta = make_delta(3);
        assert!(apply_runtime_state_delta(&snapshot, &delta, false).is_err());
    }

    #[test]
    fn applies_hook_upsert_and_resolve() {
        let snapshot = RuntimeStateSnapshot::empty(Language::Zh);
        let mut delta = make_delta(1);
        delta.hook_ops.upsert.push(HookRecord {
            hook_id: "h1".into(),
            start_chapter: 1,
            r#type: "mystery".into(),
            status: HookStatus::Open,
            last_advanced_chapter: 1,
            expected_payoff: String::new(),
            payoff_timing: None,
            notes: String::new(),
            depends_on: None,
            pays_off_in_arc: None,
            core_hook: None,
            half_life_chapters: None,
            advanced_count: None,
            promoted: None,
        });

        let next = apply_runtime_state_delta(&snapshot, &delta, false).unwrap();
        assert_eq!(next.hooks.hooks.len(), 1);
        assert_eq!(next.hooks.hooks[0].hook_id, "h1");

        // 在 chapter 2 resolve
        let mut delta2 = make_delta(2);
        delta2.hook_ops.resolve.push("h1".into());
        let next2 = apply_runtime_state_delta(&next, &delta2, false).unwrap();
        assert_eq!(next2.hooks.hooks[0].status, HookStatus::Resolved);
        assert_eq!(next2.hooks.hooks[0].last_advanced_chapter, 2);
    }
}
