//! Long span fatigue detection.

use crate::features::story::{AuditIssue, AuditSeverity};

pub struct FatigueParams {
    pub language: String,
    pub chapter_number: u32,
    pub recent_chapters: Vec<String>,
}

/// Analyze long-span fatigue across recent chapters.
pub fn analyze_long_span_fatigue(params: FatigueParams) -> Vec<AuditIssue> {
    let is_en = params.language == "en";
    let mut issues = Vec::new();
    if params.recent_chapters.len() < 3 { return issues; }

    let openings: Vec<&str> = params.recent_chapters.iter().filter_map(|ch| ch.lines().next()).collect();
    if openings.len() >= 3 {
        let unique: std::collections::HashSet<&str> = openings.iter().copied().collect();
        if unique.len() <= 1 && openings.len() >= 3 {
            issues.push(AuditIssue {
                severity: AuditSeverity::Warning,
                category: if is_en { "Pacing Monotony" } else { "节奏单调" }.to_string(),
                description: if is_en { "Recent chapters have identical opening patterns.".into() } else { "近几章的开头模式完全相同。".into() },
                suggestion: if is_en { "Vary the opening technique.".into() } else { "变化开头手法。".into() },
                repair_scope: None,
            });
        }
    }

    let all_paragraphs: Vec<usize> = params.recent_chapters.iter()
        .flat_map(|ch| ch.split("\n\n").map(|p| p.len())).collect();
    if all_paragraphs.len() > 20 {
        let avg = all_paragraphs.iter().sum::<usize>() as f64 / all_paragraphs.len() as f64;
        let uniform = all_paragraphs.iter().filter(|&&len| (len as f64 - avg).abs() < avg * 0.1).count();
        if uniform as f64 / all_paragraphs.len() as f64 > 0.8 {
            issues.push(AuditIssue {
                severity: AuditSeverity::Warning,
                category: if is_en { "Paragraph Uniformity" } else { "段落等长" }.to_string(),
                description: if is_en { "Paragraph lengths are too uniform.".into() } else { "段落长度过于均匀。".into() },
                suggestion: if is_en { "Vary paragraph lengths.".into() } else { "变化段落长度。".into() },
                repair_scope: None,
            });
        }
    }
    issues
}

/// Build a brief for English variance.
pub fn build_english_variance_brief(book_dir: &std::path::Path, chapter_number: u32) -> Option<String> {
    let recent = load_recent_chapters(book_dir, chapter_number, 5);
    if recent.len() < 2 { return None; }
    let all_sentences: Vec<usize> = recent.iter()
        .flat_map(|ch| ch.split(['.', '!', '?']).map(|s| s.len()))
        .filter(|&l| l > 10).collect();
    if all_sentences.len() < 10 { return None; }
    let avg = all_sentences.iter().sum::<usize>() as f64 / all_sentences.len() as f64;
    let variance = all_sentences.iter().map(|&l| (l as f64 - avg).powi(2)).sum::<f64>() / all_sentences.len() as f64;
    let cv = variance.sqrt() / avg;
    if cv < 0.3 { Some("Vary sentence lengths more.".to_string()) } else { None }
}

fn load_recent_chapters(book_dir: &std::path::Path, _current_chapter: u32, count: usize) -> Vec<String> {
    let chapters_dir = book_dir.join("chapters");
    if !chapters_dir.exists() { return vec![]; }
    let mut files: Vec<String> = std::fs::read_dir(&chapters_dir).into_iter().flatten()
        .filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|f| f.ends_with(".md")).collect();
    files.sort();
    let start = if files.len() > count + 1 { files.len() - count - 1 } else { 0 };
    let recent_files = &files[start..files.len().saturating_sub(1)];
    recent_files.iter().filter_map(|f| std::fs::read_to_string(chapters_dir.join(f)).ok()).collect()
}
