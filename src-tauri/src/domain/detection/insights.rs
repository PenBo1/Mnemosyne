// 检测历史聚合统计。
//
// 按 chapter 分组,排序后取首条作为 original_score,末条作为 final_score,
// rewrite 次数为该章 action=="rewrite" 的条目数。
// pass_rate = final_score <= original_score 的章节占比。

use std::collections::BTreeMap;

use super::types::{ChapterDetectionRow, DetectionStats};

/// 分析检测历史,产出聚合统计。
pub fn analyze_detection_insights(history: &[super::types::DetectionHistoryEntry]) -> DetectionStats {
    if history.is_empty() {
        return DetectionStats {
            total_detections: 0,
            total_rewrites: 0,
            avg_original_score: 0.0,
            avg_final_score: 0.0,
            pass_rate: 0.0,
            chapter_breakdown: Vec::new(),
        };
    }

    let total_detections = history.iter().filter(|h| h.action == "detect").count() as u32;
    let total_rewrites = history.iter().filter(|h| h.action == "rewrite").count() as u32;

    // 按 chapter_number 分组(BTreeMap 保证按章号有序)
    let mut chapter_map: BTreeMap<u32, Vec<&super::types::DetectionHistoryEntry>> = BTreeMap::new();
    for entry in history {
        chapter_map.entry(entry.chapter_number).or_default().push(entry);
    }

    let mut chapter_breakdown = Vec::with_capacity(chapter_map.len());
    let mut total_original = 0.0_f64;
    let mut total_final = 0.0_f64;

    for (&chapter_number, entries) in &chapter_map {
        let mut sorted: Vec<&&super::types::DetectionHistoryEntry> = entries.iter().collect();
        sorted.sort_by_key(|e| e.attempt);
        let original_score = sorted.first().map(|e| e.score).unwrap_or(0.0);
        let final_score = sorted.last().map(|e| e.score).unwrap_or(original_score);
        let rewrite_attempts = entries.iter().filter(|e| e.action == "rewrite").count() as u32;
        chapter_breakdown.push(ChapterDetectionRow {
            chapter_number,
            original_score,
            final_score,
            rewrite_attempts,
        });
        total_original += original_score;
        total_final += final_score;
    }

    let chapter_count = chapter_breakdown.len();
    let avg_original_score = if chapter_count > 0 {
        total_original / chapter_count as f64
    } else {
        0.0
    };
    let avg_final_score = if chapter_count > 0 {
        total_final / chapter_count as f64
    } else {
        0.0
    };
    let passed = chapter_breakdown
        .iter()
        .filter(|c| c.final_score <= c.original_score)
        .count();
    let pass_rate = if chapter_count > 0 {
        passed as f64 / chapter_count as f64
    } else {
        0.0
    };

    DetectionStats {
        total_detections,
        total_rewrites,
        avg_original_score: round3(avg_original_score),
        avg_final_score: round3(avg_final_score),
        pass_rate: round2(pass_rate),
        chapter_breakdown,
    }
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::DetectionHistoryEntry;

    fn entry(ch: u32, action: &str, attempt: u32, score: f64) -> DetectionHistoryEntry {
        DetectionHistoryEntry {
            book_id: "b1".into(),
            chapter_number: ch,
            action: action.into(),
            attempt,
            score,
            provider: "gptzero".into(),
            detected_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn empty_history_returns_zeros() {
        let stats = analyze_detection_insights(&[]);
        assert_eq!(stats.total_detections, 0);
        assert_eq!(stats.chapter_breakdown.len(), 0);
    }

    #[test]
    fn single_detect_counts_as_passed() {
        let history = vec![entry(1, "detect", 1, 0.9)];
        let stats = analyze_detection_insights(&history);
        assert_eq!(stats.total_detections, 1);
        assert_eq!(stats.chapter_breakdown.len(), 1);
        assert_eq!(stats.chapter_breakdown[0].original_score, 0.9);
        assert_eq!(stats.chapter_breakdown[0].final_score, 0.9);
        assert_eq!(stats.pass_rate, 1.0);
    }

    #[test]
    fn rewrite_lowers_final_score() {
        let history = vec![
            entry(2, "detect", 1, 0.95),
            entry(2, "rewrite", 2, 0.0),
            entry(2, "detect", 3, 0.4),
        ];
        let stats = analyze_detection_insights(&history);
        assert_eq!(stats.total_detections, 2);
        assert_eq!(stats.total_rewrites, 1);
        assert_eq!(stats.chapter_breakdown[0].original_score, 0.95);
        assert_eq!(stats.chapter_breakdown[0].final_score, 0.4);
        assert_eq!(stats.chapter_breakdown[0].rewrite_attempts, 1);
        assert_eq!(stats.pass_rate, 1.0);
    }
}
