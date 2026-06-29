//! Narrative control utilities.

use crate::core::agent::governance::{ChapterIntent, ChapterMemo};

/// Build a narrative intent brief from chapter intent.
pub fn build_narrative_intent_brief(chapter_intent: &str, language: &str) -> String {
    if chapter_intent.trim().is_empty() { return String::new(); }
    if language == "en" {
        format!("## Narrative Intent\n\n{}", chapter_intent)
    } else {
        format!("## 叙事意图\n\n{}", chapter_intent)
    }
}

/// Render chapter memo as a narrative block for the writer prompt.
pub fn render_memo_as_narrative_block(memo: &ChapterMemo, intent: Option<&ChapterIntent>, language: &str) -> String {
    let mut sections = Vec::new();
    if language == "en" {
        sections.push("## Chapter Brief".to_string());
        sections.push(format!("Goal: {}", memo.goal));
        if let Some(intent) = intent {
            if !intent.must_keep.is_empty() { sections.push(format!("Must keep: {}", intent.must_keep.join("; "))); }
            if !intent.must_avoid.is_empty() { sections.push(format!("Must avoid: {}", intent.must_avoid.join("; "))); }
        }
    } else {
        sections.push("## 章节简报".to_string());
        sections.push(format!("目标：{}", memo.goal));
        if let Some(intent) = intent {
            if !intent.must_keep.is_empty() { sections.push(format!("必须保持：{}", intent.must_keep.join("；"))); }
            if !intent.must_avoid.is_empty() { sections.push(format!("必须避免：{}", intent.must_avoid.join("；"))); }
        }
    }
    if !memo.body.is_empty() { sections.push(String::new()); sections.push(memo.body.clone()); }
    sections.join("\n")
}

/// Render selected context entries as narrative sections.
pub fn render_narrative_selected_context(entries: &[crate::core::agent::governance::ContextSource], language: &str) -> String {
    if entries.is_empty() { return if language == "en" { "(none)" } else { "(无)" }.to_string(); }
    entries.iter().map(|e| {
        let excerpt = e.excerpt.as_deref().unwrap_or(&e.reason);
        format!("### {} ({})\n\n{}", e.source, e.reason, excerpt)
    }).collect::<Vec<_>>().join("\n\n")
}

/// Sanitize an evidence block for inclusion in prompts.
pub fn sanitize_narrative_evidence_block(block: &str, _language: &str) -> String {
    block.replace("```", "'''")
}
