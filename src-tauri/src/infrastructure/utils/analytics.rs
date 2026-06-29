//! Analytics utilities for token usage and performance tracking.

/// Compute analytics from pipeline runs
pub fn compute_analytics(runs: &[PipelineRun]) -> Analytics {
    let total_runs = runs.len() as u32;
    let total_tokens: u64 = runs.iter().map(|r| r.total_tokens as u64).sum();
    let avg_tokens = if total_runs > 0 { total_tokens / total_runs as u64 } else { 0 };
    let total_words: u32 = runs.iter().map(|r| r.word_count).sum();

    let avg_score: f64 = if runs.is_empty() {
        0.0
    } else {
        runs.iter().map(|r| r.audit_score).sum::<f64>() / runs.len() as f64
    };

    Analytics {
        total_runs,
        total_tokens,
        avg_tokens_per_run: avg_tokens,
        total_words,
        avg_audit_score: avg_score,
    }
}

pub struct PipelineRun {
    pub total_tokens: u32,
    pub word_count: u32,
    pub audit_score: f64,
}

pub struct Analytics {
    pub total_runs: u32,
    pub total_tokens: u64,
    pub avg_tokens_per_run: u64,
    pub total_words: u32,
    pub avg_audit_score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_analytics() {
        let runs = vec![
            PipelineRun { total_tokens: 1000, word_count: 3000, audit_score: 85.0 },
            PipelineRun { total_tokens: 1500, word_count: 3500, audit_score: 90.0 },
        ];
        let analytics = compute_analytics(&runs);
        assert_eq!(analytics.total_runs, 2);
        assert_eq!(analytics.avg_tokens_per_run, 1250);
        assert_eq!(analytics.total_words, 6500);
    }
}
