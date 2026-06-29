//! Chapter cadence analysis.

pub const DEFAULT_CHAPTER_CADENCE_WINDOW: u32 = 5;

pub fn analyze_chapter_cadence(chapter_summaries: &str, current_chapter: u32, window: u32) -> CadenceAnalysis {
    let rows = parse_summary_rows(chapter_summaries);
    let recent: Vec<&SummaryRow> = rows.iter()
        .filter(|r| r.chapter < current_chapter && r.chapter >= current_chapter.saturating_sub(window))
        .collect();
    if recent.is_empty() { return CadenceAnalysis { dominant_type: "unknown".into(), consecutive_same_type: 0, type_distribution: std::collections::HashMap::new() }; }
    let mut type_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for row in &recent { *type_counts.entry(row.chapter_type.clone()).or_insert(0) += 1; }
    let dominant_type = type_counts.iter().max_by_key(|(_, c)| *c).map(|(t, _)| t.clone()).unwrap_or_default();
    let mut consecutive = 0u32;
    for row in recent.iter().rev() { if row.chapter_type == dominant_type { consecutive += 1; } else { break; } }
    CadenceAnalysis { dominant_type, consecutive_same_type: consecutive, type_distribution: type_counts }
}

pub struct CadenceAnalysis { pub dominant_type: String, pub consecutive_same_type: u32, pub type_distribution: std::collections::HashMap<String, u32> }

struct SummaryRow { chapter: u32, chapter_type: String }

fn parse_summary_rows(markdown: &str) -> Vec<SummaryRow> {
    let mut rows = Vec::new();
    for line in markdown.lines() {
        if !line.starts_with('|') || line.contains("---") { continue; }
        let cells: Vec<&str> = line.split('|').skip(1).collect();
        if cells.len() < 8 { continue; }
        if let Ok(chapter) = cells[0].trim().parse::<u32>() {
            let ct = cells[7].trim().to_string();
            if !ct.is_empty() { rows.push(SummaryRow { chapter, chapter_type: ct }); }
        }
    }
    rows
}
