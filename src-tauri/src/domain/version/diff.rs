//! 行级 diff 算法（LCS + hunk 分组）
//!
//! 实现思路：
//! 1. 用 LCS 动态规划求出两段文本的最长公共子序列（按行）
//! 2. 回溯 LCS 表，生成 added/removed/context 行序列
//! 3. 将连续的差异行聚合成 hunks，context 行作为上下文（默认前后保留 3 行）
//! 4. 统计行数与字符数

use crate::shared::version::types::{DiffHunk, DiffLine, DiffLineType, DiffStats, LineDiffResult};

/// 上下文行数（hunk 前后保留的未变更行数）
const CONTEXT_LINES: usize = 3;

/// 计算两段文本的行级 diff
pub fn compute_line_diff(old: &str, new: &str) -> LineDiffResult {
    let old_lines: Vec<&str> = old.split('\n').collect();
    let new_lines: Vec<&str> = new.split('\n').collect();

    let ops = lcs_diff_ops(&old_lines, &new_lines);
    let hunks = build_hunks(&ops, &old_lines, &new_lines);
    let stats = build_stats(&ops);

    LineDiffResult { hunks, stats }
}

/// LCS 回溯产生的操作序列
#[derive(Debug, Clone)]
enum DiffOp {
    Equal { old_idx: usize, new_idx: usize },
    Added { new_idx: usize },
    Removed { old_idx: usize },
}

/// 用 LCS 动态规划求 diff 操作序列
fn lcs_diff_ops(old: &[&str], new: &[&str]) -> Vec<DiffOp> {
    let n = old.len();
    let m = new.len();
    // dp[i][j] = old[0..i] 与 new[0..j] 的 LCS 长度
    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in 1..=n {
        for j in 1..=m {
            if old[i - 1] == new[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }
    // 回溯生成操作序列（从尾部开始，最后反转）
    let mut ops: Vec<DiffOp> = Vec::with_capacity(n + m);
    let (mut i, mut j) = (n, m);
    while i > 0 && j > 0 {
        if old[i - 1] == new[j - 1] {
            ops.push(DiffOp::Equal { old_idx: i - 1, new_idx: j - 1 });
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] >= dp[i][j - 1] {
            ops.push(DiffOp::Removed { old_idx: i - 1 });
            i -= 1;
        } else {
            ops.push(DiffOp::Added { new_idx: j - 1 });
            j -= 1;
        }
    }
    while i > 0 {
        ops.push(DiffOp::Removed { old_idx: i - 1 });
        i -= 1;
    }
    while j > 0 {
        ops.push(DiffOp::Added { new_idx: j - 1 });
        j -= 1;
    }
    ops.reverse();
    ops
}

/// 将操作序列聚合成 hunks（带上下文）
fn build_hunks(ops: &[DiffOp], old_lines: &[&str], new_lines: &[&str]) -> Vec<DiffHunk> {
    if ops.is_empty() {
        return Vec::new();
    }

    // 找出所有差异位置（非 Equal 的操作索引）
    let diff_indices: Vec<usize> = ops
        .iter()
        .enumerate()
        .filter_map(|(i, op)| match op {
            DiffOp::Equal { .. } => None,
            _ => Some(i),
        })
        .collect();

    if diff_indices.is_empty() {
        return Vec::new();
    }

    // 将差异位置分组：相邻差异（考虑上下文重叠）合并为同一 hunk
    let mut groups: Vec<(usize, usize)> = Vec::new();
    let mut current_start = diff_indices[0].saturating_sub(CONTEXT_LINES);
    let mut current_end = (diff_indices[0] + 1).min(ops.len()) + CONTEXT_LINES;

    for &idx in &diff_indices[1..] {
        let gap_start = idx.saturating_sub(CONTEXT_LINES);
        if gap_start <= current_end {
            // 合并
            current_end = (idx + 1).min(ops.len()) + CONTEXT_LINES;
        } else {
            groups.push((current_start, current_end.min(ops.len())));
            current_start = gap_start;
            current_end = (idx + 1).min(ops.len()) + CONTEXT_LINES;
        }
    }
    groups.push((current_start, current_end.min(ops.len())));

    // 为每个分组生成 hunk
    groups
        .iter()
        .filter_map(|&(start, end)| {
            if start >= ops.len() {
                return None;
            }
            let end = end.min(ops.len());
            let mut lines: Vec<DiffLine> = Vec::new();
            let mut old_start: Option<u32> = None;
            let mut new_start: Option<u32> = None;
            let mut old_count = 0u32;
            let mut new_count = 0u32;

            for k in start..end {
                match &ops[k] {
                    DiffOp::Equal { old_idx, new_idx } => {
                        let old_no = (*old_idx as u32) + 1;
                        let new_no = (*new_idx as u32) + 1;
                        if old_start.is_none() {
                            old_start = Some(old_no);
                        }
                        if new_start.is_none() {
                            new_start = Some(new_no);
                        }
                        lines.push(DiffLine {
                            line_type: DiffLineType::Context,
                            content: old_lines[*old_idx].to_string(),
                            old_number: Some(old_no),
                            new_number: Some(new_no),
                        });
                        old_count += 1;
                        new_count += 1;
                    }
                    DiffOp::Removed { old_idx } => {
                        let old_no = (*old_idx as u32) + 1;
                        if old_start.is_none() {
                            old_start = Some(old_no);
                        }
                        lines.push(DiffLine {
                            line_type: DiffLineType::Removed,
                            content: old_lines[*old_idx].to_string(),
                            old_number: Some(old_no),
                            new_number: None,
                        });
                        old_count += 1;
                    }
                    DiffOp::Added { new_idx } => {
                        let new_no = (*new_idx as u32) + 1;
                        if new_start.is_none() {
                            new_start = Some(new_no);
                        }
                        lines.push(DiffLine {
                            line_type: DiffLineType::Added,
                            content: new_lines[*new_idx].to_string(),
                            old_number: None,
                            new_number: Some(new_no),
                        });
                        new_count += 1;
                    }
                }
            }

            // 跳过纯上下文的空 hunk（理论上不会出现，但防御）
            if old_count == 0 && new_count == 0 {
                return None;
            }

            Some(DiffHunk {
                old_start: old_start.unwrap_or(1),
                old_lines: old_count,
                new_start: new_start.unwrap_or(1),
                new_lines: new_count,
                lines,
            })
        })
        .collect()
}

/// 统计行数和字符数
fn build_stats(ops: &[DiffOp]) -> DiffStats {
    let mut stats = DiffStats::default();
    for op in ops {
        match op {
            DiffOp::Equal { .. } => {}
            DiffOp::Removed { old_idx: _ } => {
                stats.lines_removed += 1;
            }
            DiffOp::Added { new_idx: _ } => {
                stats.lines_added += 1;
            }
        }
    }
    // lines_modified 近似为 added 和 removed 的最小值（成对出现的视为修改）
    stats.lines_modified = stats.lines_added.min(stats.lines_removed);
    stats
}
