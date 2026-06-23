use sha2::{Sha256, Digest};
use similar::{ChangeTag, TextDiff};
use super::models::*;

/// Diff engine for computing line-level differences
pub struct DiffEngine;

impl DiffEngine {
    /// Compute SHA256 hash of content
    pub fn compute_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Compute line-level diff between two content strings
    pub fn compute_line_diff(old_content: &str, new_content: &str) -> LineDiffResult {
        let diff = TextDiff::from_lines(old_content, new_content);
        
        let mut hunks: Vec<DiffHunk> = Vec::new();
        let mut current_hunk: Option<HunkBuilder> = None;
        let mut old_line = 0u32;
        let mut new_line = 0u32;
        
        for change in diff.iter_all_changes() {
            let tag = change.tag();
            let value = change.value();
            
            match tag {
                ChangeTag::Equal => {
                    // If we have a pending hunk, finalize it
                    if let Some(builder) = current_hunk.take() {
                        hunks.push(builder.build());
                    }
                    old_line += 1;
                    new_line += 1;
                }
                ChangeTag::Delete => {
                    // Start a new hunk if needed
                    if current_hunk.is_none() {
                        current_hunk = Some(HunkBuilder::new(old_line, new_line));
                    }
                    current_hunk.as_mut().unwrap().add_removed(value, old_line);
                    old_line += 1;
                }
                ChangeTag::Insert => {
                    // Start a new hunk if needed
                    if current_hunk.is_none() {
                        current_hunk = Some(HunkBuilder::new(old_line, new_line));
                    }
                    current_hunk.as_mut().unwrap().add_added(value, new_line);
                    new_line += 1;
                }
            }
        }
        
        // Finalize any pending hunk
        if let Some(builder) = current_hunk {
            hunks.push(builder.build());
        }
        
        // Compute statistics
        let stats = compute_stats(&hunks);
        
        LineDiffResult { hunks, stats }
    }
}

/// Helper struct for building hunks
struct HunkBuilder {
    old_start: u32,
    new_start: u32,
    lines: Vec<DiffLine>,
    old_count: u32,
    new_count: u32,
}

impl HunkBuilder {
    fn new(old_start: u32, new_start: u32) -> Self {
        Self {
            old_start,
            new_start,
            lines: Vec::new(),
            old_count: 0,
            new_count: 0,
        }
    }
    
    fn add_removed(&mut self, content: &str, old_number: u32) {
        self.lines.push(DiffLine {
            line_type: DiffLineType::Removed,
            content: content.to_string(),
            old_number: Some(old_number),
            new_number: None,
        });
        self.old_count += 1;
    }
    
    fn add_added(&mut self, content: &str, new_number: u32) {
        self.lines.push(DiffLine {
            line_type: DiffLineType::Added,
            content: content.to_string(),
            old_number: None,
            new_number: Some(new_number),
        });
        self.new_count += 1;
    }
    
    fn build(self) -> DiffHunk {
        DiffHunk {
            old_start: self.old_start + 1, // 1-indexed
            old_lines: self.old_count,
            new_start: self.new_start + 1, // 1-indexed
            new_lines: self.new_count,
            lines: self.lines,
        }
    }
}

/// Compute statistics from hunks
fn compute_stats(hunks: &[DiffHunk]) -> DiffStats {
    let mut stats = DiffStats::default();
    
    for hunk in hunks {
        for line in &hunk.lines {
            match line.line_type {
                DiffLineType::Added => {
                    stats.lines_added += 1;
                    stats.chars_added += line.content.len() as u32;
                }
                DiffLineType::Removed => {
                    stats.lines_removed += 1;
                    stats.chars_removed += line.content.len() as u32;
                }
                DiffLineType::Context => {}
            }
        }
    }
    
    // Approximate modified lines as min of added/removed
    stats.lines_modified = std::cmp::min(stats.lines_added, stats.lines_removed);
    
    stats
}