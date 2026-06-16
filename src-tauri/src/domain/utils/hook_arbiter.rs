//! Hook arbiter for resolving conflicts in runtime state deltas.

use crate::domain::agents::governance::ChapterIntent;

/// Resolve conflicts between multiple hook operations
pub fn arbitrate_hook_ops(
    ops: Vec<HookOperation>,
    intent: &ChapterIntent,
) -> Vec<HookOperation> {
    let mut resolved = Vec::new();
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    for op in ops {
        if seen_names.contains(&op.name) {
            // Skip duplicate operations on the same hook
            continue;
        }
        seen_names.insert(op.name.clone());

        // Check if operation conflicts with chapter intent
        if matches!(op.operation, HookOperationType::Resolve)
            && intent.must_keep.iter().any(|k| k.contains(&op.name)) {
            continue;
        }

        resolved.push(op);
    }

    resolved
}

pub struct HookOperation {
    pub name: String,
    pub operation: HookOperationType,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HookOperationType {
    Upsert,
    Mention,
    Resolve,
    Defer,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arbitrate_dedup() {
        let ops = vec![
            HookOperation { name: "H1".into(), operation: HookOperationType::Resolve, description: None },
            HookOperation { name: "H1".into(), operation: HookOperationType::Mention, description: None },
        ];
        let intent = ChapterIntent {
            chapter: 1, goal: "test".into(), outline_node: None, arc_context: None,
            must_keep: vec![], must_avoid: vec![], style_emphasis: vec![],
        };
        let result = arbitrate_hook_ops(ops, &intent);
        assert_eq!(result.len(), 1);
    }
}
