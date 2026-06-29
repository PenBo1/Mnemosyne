//! Governed context — memory evidence blocks from ContextPackage.

use crate::core::agent::governance::ContextPackage;

pub fn build_governed_memory_evidence_blocks(context_package: &ContextPackage, language: &str) -> MemoryEvidenceBlocks {
    let hook_entries: Vec<&str> = context_package.selected_context.iter()
        .filter(|e| e.source.starts_with("story/pending_hooks.md")).map(|e| e.source.as_str()).collect();
    let summary_entries: Vec<&str> = context_package.selected_context.iter()
        .filter(|e| e.source.starts_with("story/chapter_summaries.md")).map(|e| e.source.as_str()).collect();
    let canon_entries: Vec<&str> = context_package.selected_context.iter()
        .filter(|e| e.source == "story/parent_canon.md" || e.source == "story/fanfic_canon.md").map(|e| e.source.as_str()).collect();

    MemoryEvidenceBlocks {
        hooks_block: if !hook_entries.is_empty() {
            Some(render_evidence_block(if language == "en" { "Selected Hook Evidence" } else { "已选伏笔证据" }, &context_package.selected_context, "pending_hooks"))
        } else { None },
        summaries_block: if !summary_entries.is_empty() {
            Some(render_evidence_block(if language == "en" { "Selected Chapter Summary Evidence" } else { "已选章节摘要证据" }, &context_package.selected_context, "chapter_summaries"))
        } else { None },
        canon_block: if !canon_entries.is_empty() {
            Some(render_evidence_block(if language == "en" { "Canon Evidence" } else { "正典约束证据" }, &context_package.selected_context, "canon"))
        } else { None },
    }
}

pub struct MemoryEvidenceBlocks { pub hooks_block: Option<String>, pub summaries_block: Option<String>, pub canon_block: Option<String> }

fn render_evidence_block(heading: &str, entries: &[crate::core::agent::governance::ContextSource], filter_prefix: &str) -> String {
    let lines: Vec<String> = entries.iter().filter(|e| e.source.contains(filter_prefix))
        .map(|e| { let excerpt = e.excerpt.as_deref().unwrap_or(&e.reason); format!("- {}: {}", e.source, excerpt) }).collect();
    format!("\n## {}\n{}\n", heading, lines.join("\n"))
}
