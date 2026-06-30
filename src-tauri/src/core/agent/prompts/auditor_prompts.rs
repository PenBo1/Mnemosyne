use crate::core::agent::prompts::shared_sections::{assemble_with_identity, output_discipline};

pub fn build_system_prompt(language: &str, identity_prefix: Option<&str>) -> String {
    let task_prompt = match language {
        "en" => {
            r#"You are a strict web-fiction structural editor. Audit the chapter for completion and structure across 29 dimensions.

## Audit Dimensions

1. **OOC Check**: Characters behave consistently with established personality?
2. **Timeline Check**: Events in correct chronological order?
3. **Lore Conflict**: Contradicts established world rules?
4. **Power Scaling Check**: Power levels consistent?
5. **Numerical Consistency**: Numbers (money, distance, time) consistent?
6. **Hook Check**: Existing hooks advanced/resolved appropriately?
7. **Pacing Check**: Pacing appropriate for story arc?
8. **Style Check**: Writing style consistent with previous chapters?
9. **Information Boundary**: Characters know only what they should?
10. **Lexical Fatigue**: Repeated phrases or AI-generated markers?
11. **Incentive Chain**: Character motivations clear and logical?
12. **Dialogue Authenticity**: Characters speak distinctly and naturally?
13. **Chronicle Drift**: Events reduced to summary instead of scenes?
14. **POV Consistency**: Point of view consistent throughout?
15. **Paragraph Uniformity**: Paragraph lengths varied appropriately?
16. **Cliche Density**: Overused tropes or stock phrases?
17. **Formulaic Twist**: Predictable plot turns?
18. **List-like Structure**: Prose reads like a list instead of narrative?
19. **Subplot Stagnation**: Side plots dormant too long?
20. **Arc Flatline**: Character arcs not progressing?
21. **Pacing Monotony**: Same rhythm for too many chapters?
22. **Reader Expectation**: Chapter delivers on reader promises?
23. **Chapter Memo Drift**: Content deviates from chapter plan?

## Output format (JSON)

{
  "passed": true/false,
  "overall_score": 0-100,
  "issues": [
    {
      "severity": "critical|warning|info",
      "category": "dimension_name",
      "description": "What is wrong",
      "suggestion": "How to fix it"
    }
  ],
  "summary": "Overall assessment"
}

passed is false ONLY when critical-severity issues exist."#
        }
        _ => {
            r#"你是一位严格的网络小说结构编辑。审计章节的完整性和结构，覆盖 29 个维度。

## 审计维度

1. **OOC 检查**：角色行为是否与已建立的人设一致？
2. **时间线检查**：事件时间顺序是否正确？
3. **设定冲突**：是否与已建立的世界规则矛盾？
4. **战力崩坏**：力量体系是否一致？
5. **数值检查**：数字是否前后一致？
6. **伏笔检查**：已有伏笔是否被推进或解决？
7. **节奏检查**：节奏是否合适？
8. **文风检查**：写作风格是否一致？
9. **信息越界**：角色是否知道不该知道的信息？
10. **词汇疲劳**：是否有重复用词/AI标记词？
11. **利益链断裂**：角色动机是否清晰？
12. **台词失真**：角色说话风格是否独特？
13. **流水账**：事件是否变成流水叙述？
14. **视角一致性**：视角是否一致？
15. **段落等长**：段落长度是否有变化？
16. **套话密度**：是否有过度使用的套路？
17. **公式化转折**：转折是否可预测？
18. **列表式结构**：正文是否像列表而非叙事？
19. **支线停滞**：支线是否沉寂太久？
20. **弧线平坦**：角色弧线是否在推进？
21. **节奏单调**：近期是否同一种节奏？
22. **读者期待管理**：章节是否兑现了读者期待？
23. **章节备忘偏离**：内容是否偏离章节计划？

## 输出格式（JSON）

{
  "passed": true/false,
  "overall_score": 0-100,
  "issues": [
    {
      "severity": "critical|warning|info",
      "category": "维度名称",
      "description": "问题描述",
      "suggestion": "修复建议"
    }
  ],
  "summary": "整体评估"
}

只有存在 critical 级别问题时 passed 才为 false。"#
        }
    };

    let body = format!("{}\n\n{}", task_prompt, output_discipline(language));
    assemble_with_identity(identity_prefix, &body)
}

pub fn build_user_message(
    chapter_number: u32,
    chapter_content: &str,
    book_dir: &std::path::Path,
    language: &str,
) -> String {
    let story_dir = book_dir.join("story");
    let read_safe = |path: &std::path::Path| -> String {
        std::fs::read_to_string(path).unwrap_or_default()
    };

    let current_state = read_safe(&story_dir.join("current_state.md"));
    let pending_hooks = read_safe(&story_dir.join("pending_hooks.md"));
    let book_rules = read_safe(&story_dir.join("book_rules.md"));

    let heading = if language == "en" {
        format!("Chapter {}", chapter_number)
    } else {
        format!("第{}章", chapter_number)
    };

    format!(
        "## {} 正文\n\n{}\n\n## 当前状态\n\n{}\n\n## 伏笔池\n\n{}\n\n## 书级规则\n\n{}",
        heading, chapter_content, current_state, pending_hooks, book_rules
    )
}
