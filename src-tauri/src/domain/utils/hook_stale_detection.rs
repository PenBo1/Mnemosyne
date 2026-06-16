//! Stale / blocked hook detection.
//!
//! Tags hooks the reviewer and planner should pay attention to:
//! - stale: planted long ago and still not resolved
//! - blocked: depends_on references upstream hooks that are still unresolved

use crate::domain::utils::story_markdown::ParsedHook;

pub struct HookDiagnostics {
    pub stale: bool,
    pub blocked: bool,
    pub missing_upstream: Vec<String>,
    pub distance: u32,
    pub half_life: u32,
    pub blocked_distance: u32,
}

/// Compute diagnostics for all hooks.
pub fn compute_hook_diagnostics(
    hooks: &[ParsedHook],
    current_chapter: u32,
) -> std::collections::HashMap<String, HookDiagnostics> {
    let _by_id: std::collections::HashMap<&str, &ParsedHook> = hooks.iter()
        .map(|h| (h.hook_id.as_str(), h))
        .collect();

    let mut result = std::collections::HashMap::new();

    for hook in hooks {
        let half_life = 30u32; // default mid-arc
        let planted_chapter = hook.start_chapter;
        let distance = current_chapter.saturating_sub(planted_chapter);

        let stale = !is_resolved(&hook.status)
            && planted_chapter > 0
            && distance > half_life;

        let missing_upstream = Vec::new();
        let upstream_reference_chapters = Vec::new();

        // Check depends_on (simplified - we don't store depends_on in ParsedHook yet)
        // For now, just check staleness

        let blocked = !missing_upstream.is_empty() && !is_resolved(&hook.status);

        let blocked_distance = if blocked && !upstream_reference_chapters.is_empty() {
            let earliest = upstream_reference_chapters.iter().min().copied().unwrap_or(0);
            current_chapter.saturating_sub(earliest)
        } else {
            0
        };

        result.insert(hook.hook_id.clone(), HookDiagnostics {
            stale,
            blocked,
            missing_upstream,
            distance,
            half_life,
            blocked_distance,
        });
    }

    result
}

/// Render diagnostic flags as a compact marker string.
pub fn render_hook_diagnostic_marker(diagnostics: &HookDiagnostics, language: &str) -> String {
    let mut tokens = Vec::new();

    if diagnostics.stale {
        if language == "en" {
            tokens.push(format!("stale (d={}/half={})", diagnostics.distance, diagnostics.half_life));
        } else {
            tokens.push(format!("过期 (距={}/半衰={})", diagnostics.distance, diagnostics.half_life));
        }
    }
    if diagnostics.blocked {
        let missing = diagnostics.missing_upstream.join(", ");
        let distance_token = if diagnostics.blocked_distance > 0 {
            if language == "en" {
                format!(" (blocked {} chapters)", diagnostics.blocked_distance)
            } else {
                format!(" (已阻 {} �?", diagnostics.blocked_distance)
            }
        } else {
            String::new()
        };
        if language == "en" {
            tokens.push(format!("blocked on {}{}", missing, distance_token));
        } else {
            tokens.push(format!("受阻�?{}{}", missing, distance_token));
        }
    }

    tokens.join("; ")
}

fn is_resolved(status: &str) -> bool {
    let lower = status.trim().to_lowercase();
    matches!(lower.as_str(),
        "resolved" | "closed" | "done" | "已回收" | "已解决"
    )
}
