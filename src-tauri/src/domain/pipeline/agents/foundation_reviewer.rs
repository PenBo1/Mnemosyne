// FoundationReviewer Agent。
//
// 职责：对 Architect 生成的 5-SECTION 基础设定做结构评审，输出 5 维度评分 + 总评。
// 评分标准：80+ 通过 / 60-79 需修改 / <60 方向性错误。
//
// prompt 策略：保留 5 维度评分 + === DIMENSION: N === 输出格式。

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;

use super::architect::ArchitectOutput;

/// 单个评审维度
#[derive(Debug, Clone)]
pub struct ReviewDimension {
    pub name: String,
    pub score: u32,
    pub feedback: String,
}

/// 评审结果
#[derive(Debug, Clone)]
pub struct FoundationReviewResult {
    pub passed: bool,
    pub total_score: u32,
    pub dimensions: Vec<ReviewDimension>,
    pub overall_feedback: String,
}

/// 评审基础设定。
pub async fn review_foundation(
    engine: &AgentEngine,
    foundation: &ArchitectOutput,
    target_chapters: u32,
) -> Result<FoundationReviewResult, AppError> {
    let system_prompt = build_system_prompt(target_chapters);
    let user_message = build_user_message(foundation);

    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    Ok(parse_review_result(&response))
}

// 5 个评审维度
const DIMENSIONS: &[&str] = &[
    "核心冲突",
    "开篇节奏",
    "世界一致性",
    "角色区分度",
    "节奏可行性",
];

fn build_system_prompt(target_chapters: u32) -> String {
    format!(
        r###"<identity>
You are the chief story-architect reviewer for web fiction. Your task is to review the foundation specification produced by the Architect and score it across five dimensions.
</identity>

## Review Dimensions

1. Core conflict: Is the main-line contradiction clear? Is there a foreground/background two-layer story? Is the antagonist forceful?
2. Opening pacing: Can the first 5 chapters grip the reader? Are there explicit hooks?
3. World consistency: Are the world's iron rules self-consistent? Is there a grounded sense of texture?
4. Character differentiation: Do the main characters have contrasting details? Are their speech patterns distinct? Are their arcs clear?
5. Pacing feasibility: Is the volume-outline pacing reasonable across {target_chapters} chapters? Are there climax buildups and aftermaths?

## Scoring Standard

- 80+: Pass — the dimension is of good quality.
- 60-79: Needs revision — clear problems but not fatal.
- <60: Directional error — redo required.

## Output Format (must be followed strictly)

=== DIMENSION: 1 ===
分数：X
意见：Y

=== DIMENSION: 2 ===
分数：X
意见：Y

(emit all five dimensions in order)

=== OVERALL ===
总分：X
通过：是/否
总评：Z"###,
        target_chapters = target_chapters,
    )
}

fn build_user_message(foundation: &ArchitectOutput) -> String {
    format!(
        r###"Review the following foundation specification:

## story_frame
{story_frame}

## volume_map
{volume_map}

## roles
{roles}

## book_rules
{book_rules}

## pending_hooks
{pending_hooks}"###,
        story_frame = foundation.story_frame,
        volume_map = foundation.volume_map,
        roles = foundation.roles,
        book_rules = foundation.book_rules,
        pending_hooks = foundation.pending_hooks,
    )
}

/// 解析评审结果
fn parse_review_result(content: &str) -> FoundationReviewResult {
    let mut dimensions = Vec::new();

    for (idx, name) in DIMENSIONS.iter().enumerate() {
        let marker = format!("=== DIMENSION: {} ===", idx + 1);
        if let Some(start) = content.find(&marker) {
            let content_start = start + marker.len();
            // 找下一个 === 或文本结尾
            let next = content[content_start..]
                .find("=== ")
                .map(|p| content_start + p)
                .unwrap_or(content.len());
            let section = &content[content_start..next];

            let score = extract_score(section);
            let feedback = extract_feedback(section);
            dimensions.push(ReviewDimension {
                name: name.to_string(),
                score,
                feedback,
            });
        }
    }

    // 解析 OVERALL 区块
    let overall_section = extract_overall_section(content);
    let overall_feedback = extract_overall_feedback(&overall_section);

    // 计算总分（平均分）
    let total_score = if dimensions.is_empty() {
        0
    } else {
        dimensions.iter().map(|d| d.score).sum::<u32>() / dimensions.len() as u32
    };

    // 通过条件：总分 >= 80 且所有维度 >= 60 且 5 个维度齐全
    let all_pass_threshold = dimensions.iter().all(|d| d.score >= 60);
    let passed = total_score >= 80 && all_pass_threshold && dimensions.len() == DIMENSIONS.len();

    FoundationReviewResult {
        passed,
        total_score,
        dimensions,
        overall_feedback,
    }
}

/// 从区块中提取分数（支持 "分数：X" 和 "分数: X"）
fn extract_score(section: &str) -> u32 {
    let line = section.lines().find(|l| {
        let t = l.trim();
        t.starts_with("分数：") || t.starts_with("分数:")
    });
    match line {
        Some(l) => {
            let after = l
                .trim()
                .trim_start_matches("分数：")
                .trim_start_matches("分数:")
                .trim();
            after
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0)
        }
        None => 0,
    }
}

/// 从区块中提取意见
fn extract_feedback(section: &str) -> String {
    let line = section.lines().find(|l| {
        let t = l.trim();
        t.starts_with("意见：") || t.starts_with("意见:")
    });
    match line {
        Some(l) => l
            .trim()
            .trim_start_matches("意见：")
            .trim_start_matches("意见:")
            .trim()
            .to_string(),
        None => String::new(),
    }
}

/// 提取 OVERALL 区块内容
fn extract_overall_section(content: &str) -> String {
    let marker = "=== OVERALL ===";
    match content.find(marker) {
        Some(start) => content[start + marker.len()..].trim().to_string(),
        None => String::new(),
    }
}

/// 从 OVERALL 区块提取总评
fn extract_overall_feedback(section: &str) -> String {
    let line = section.lines().find(|l| {
        let t = l.trim();
        t.starts_with("总评：") || t.starts_with("总评:")
    });
    match line {
        Some(l) => l
            .trim()
            .trim_start_matches("总评：")
            .trim_start_matches("总评:")
            .trim()
            .to_string(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_five_dimensions() {
        let content = r###"=== DIMENSION: 1 ===
分数：85
意见：核心冲突清晰

=== DIMENSION: 2 ===
分数：80
意见：开篇节奏良好

=== DIMENSION: 3 ===
分数：90
意见：世界一致

=== DIMENSION: 4 ===
分数：82
意见：角色区分度高

=== DIMENSION: 5 ===
分数：78
意见：节奏可行

=== OVERALL ===
总分：83
通过：是
总评：整体良好"###;
        let result = parse_review_result(content);
        assert_eq!(result.dimensions.len(), 5);
        assert_eq!(result.dimensions[0].name, "核心冲突");
        assert_eq!(result.dimensions[0].score, 85);
        assert_eq!(result.dimensions[0].feedback, "核心冲突清晰");
        // (85+80+90+82+78)/5 = 415/5 = 83
        assert_eq!(result.total_score, 83);
        assert!(result.passed);
        assert_eq!(result.overall_feedback, "整体良好");
    }

    #[test]
    fn rejects_below_threshold() {
        let content = r###"=== DIMENSION: 1 ===
分数：85
意见：良好

=== DIMENSION: 2 ===
分数：50
意见：开篇太慢

=== DIMENSION: 3 ===
分数：90
意见：一致

=== DIMENSION: 4 ===
分数：82
意见：良好

=== DIMENSION: 5 ===
分数：78
意见：可行

=== OVERALL ===
总分：77
通过：否
总评：需修改"###;
        let result = parse_review_result(content);
        // 维度 2 只有 50 分，低于 60 阈值
        assert!(!result.passed);
        assert_eq!(result.dimensions[1].score, 50);
        assert_eq!(result.total_score, 77);
    }

    #[test]
    fn handles_colon_space_format() {
        let content = r###"=== DIMENSION: 1 ===
分数: 85
意见: 核心冲突清晰

=== DIMENSION: 2 ===
分数: 80
意见: 良好

=== DIMENSION: 3 ===
分数: 90
意见: 一致

=== DIMENSION: 4 ===
分数: 82
意见: 良好

=== DIMENSION: 5 ===
分数: 88
意见: 可行

=== OVERALL ===
总分: 85
通过: 是
总评: 通过"###;
        let result = parse_review_result(content);
        assert_eq!(result.dimensions.len(), 5);
        assert_eq!(result.dimensions[0].score, 85);
        assert!(result.passed);
    }

    #[test]
    fn handles_missing_dimensions() {
        let content = r###"=== DIMENSION: 1 ===
分数：85
意见：良好

=== OVERALL ===
总分：85
通过：是
总评：通过"###;
        let result = parse_review_result(content);
        // 只有 1 个维度，不齐全 → 不通过
        assert!(!result.passed);
        assert_eq!(result.dimensions.len(), 1);
    }
}
