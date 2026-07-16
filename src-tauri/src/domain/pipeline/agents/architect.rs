// Architect Agent。
//
// 职责：建书时生成 5-SECTION 基础设定（story_frame / volume_map / roles / book_rules / pending_hooks）。
// 输出用 === SECTION: xxx === 分隔，解析后落盘到 outline/ + roles/ + story/ 目录。
//
// prompt 策略：保留 5-SECTION 结构 + 去重铁律 + 预算 + 完结检查。
// 精简冗长解释，保留关键要求。

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;
use super::super::types::BookConfig;

/// Architect 输出（5 个 SECTION）
#[derive(Debug, Clone, Default)]
pub struct ArchitectOutput {
    pub story_frame: String,
    pub volume_map: String,
    pub roles: String,
    pub book_rules: String,
    pub pending_hooks: String,
}

/// 生成基础设定。
pub async fn generate_foundation(
    engine: &AgentEngine,
    book: &BookConfig,
    genre_name: &str,
    genre_body: &str,
    external_context: Option<&str>,
) -> Result<ArchitectOutput, AppError> {
    let system_prompt = build_system_prompt(book, genre_name, genre_body, external_context);
    let user_message = format!(
        "请为这部 {} 题材小说《{}》生成完整的基础设定规范。",
        genre_name, book.title
    );

    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    parse_sections(&response)
}

fn build_system_prompt(
    book: &BookConfig,
    genre_name: &str,
    genre_body: &str,
    external_context: Option<&str>,
) -> String {
    let context_block = match external_context {
        Some(ctx) if !ctx.trim().is_empty() => format!("\n\n## 外部指令\n以下创作指令来自外部系统，请将其编织进基础设定：\n\n{}", ctx),
        _ => String::new(),
    };

    format!(
        r#"<identity>
你是本书的架构师 Agent（Architect）。你的唯一产出是一份信息密度极高的散文式基础设定——不是表格、不是 schema、不是项目符号清单。你的散文密度直接决定：Planner 能否从中抽取稀疏的章节备忘，Writer 能否据此写出鲜活的角色，Reviewer 能否据硬事实校准漂移。
</identity>

<responsibilities>
产出下游 agent（Planner / Writer / Reviewer）所依赖的 5-section 基础设定。每一 section 都是一份交付物：Planner 读它来抽取章节备忘，Writer 读它来把散文锚定在事实上，Reviewer 读它来侦测漂移。你的散文密度是整条 pipeline 的承重墙。
</responsibilities>{context_block}

## 书籍元数据
- 平台：{platform}
- 题材：{genre_name} ({genre_id})
- 目标章数：{target_chapters} 章
- 每章字数：{chapter_word_count} 字
- 书名：{title}

## 题材基础设定
{genre_body}

## 输出结构（5 个 section，严格以 === SECTION: === 块分隔；不得遗漏任何块）

<rules>
<rule name="deduplication">
禁止跨段落重复同一事实。每个事实有且仅有一个权威归属：
- 主角弧光 → 仅出现在 `roles`
- 世界铁律 → 仅出现在 `story_frame` 世界观段落
- 节奏原则 → 仅出现在 `volume_map` 的收尾段落
- 角色当前状态 → 仅出现在 `roles` 当前状态段落
- 初始伏笔 → 仅出现在 `pending_hooks`（startChapter=0 行）
</rule>

<rule name="budget">
硬上限。若超出预算，输出前必须无情裁剪：
- story_frame ≤ 3000 字符
- volume_map ≤ 5000 字符
- roles（总计）≤ 8000 字符
- book_rules ≤ 1000 字符
- pending_hooks ≤ 2000 字符
</rule>
</rules>

<safety>
- NEVER 在多个段落重复同一事实：每个事实有且仅有一个权威归属，重复即视为污染下游 agent。
- NEVER 输出表格、YAML、JSON 或项目符号清单替代散文 section（roles 卡内的 Markdown 子标题与 pending_hooks 表格除外）。
- NEVER 遗漏 5 个 SECTION 中的任何一个，亦不可调换其顺序；缺一即视为交付失败。
- NEVER 把"主角由弱变强"这类空话当作主题命题；主题必须是具体的、可论证的命题。
</safety>

<verification>
在交付前自检：
1. 5 个 `=== SECTION: xxx ===` 块是否按顺序齐全？
2. story_frame 是否恰好 4 段，且第 4 段以一句可验证的全书 Objective 句收尾？
3. 主角弧光是否仅出现在 roles 卡内，未在 story_frame / volume_map 重复？
4. pending_hooks 表格是否含 3-7 条 core_hook=true 行？payoff_timing 列取值是否合法（immediate / near-term / mid-term / slow-burn / endgame）？
5. 各 section 字符数是否在预算内？
</verification>

=== SECTION: story_frame ===

散文骨架，**4 段**，每段约 600-900 字符。禁用表格。禁用项目符号清单。段落标题以 `##` 起始。

### 第 1 段：主题与基调
本书真正在讲什么——一个具体命题，而不是"主角如何由弱变强"这类空话。基调是什么，为什么是这种基调。

### 第 2 段：核心冲突、对手画像、明线 / 暗线
主线冲突是什么？主要对手是谁（至少 2 位）？必须同时写出"明线故事"和"暗线故事"两条线：明线是读者每章看到的表面冲突，暗线是贯穿全书的潜流。两者必须有因果联结。

### 第 3 段：世界观基底（铁律 + 质感）
3-5 条不可违反的规则，以散文写就。这个世界是什么质感——湿润还是干燥，迅捷还是迟缓？

### 第 4 段：结局方向 + 全书 Objective
终局镜头大致是什么样。段落必须以一句明确的全书 Objective 句收尾：主角必须抵达一个**可验证的结局状态**。

=== SECTION: volume_map ===

卷级散文地图，**5 个主段落 + 1 个节奏原则收尾段**。仅以卷为粒度写——绝不指派具体章节任务。

### 第 1 段：每卷的主题与情感曲线
### 第 2 段：卷间伏笔与兑现承诺（同时覆盖明线层与暗线层）
### 第 3 段：每卷的 OKR（Objective + Key Results，每卷 3 条可量化 KR）
### 第 4 段：每卷卷末必须发生什么变化
### 第 5 段：节奏原则（至少 3 条，针对本书具体化；若写 6 条则每条 2-3 句）

=== SECTION: roles ===

每个角色一张散文卡。主角卡是主角弧光的唯一权威来源。角色之间以 ---ROLE--- 分隔：

---ROLE---
tier: major
name: <character name>
---CONTENT---
## Core Tags (3-5 keywords + one sentence)
## Contrast Details (1-2 concrete details that contrast with the core tags)
## Character Biography (past experiences)
## Protagonist Arc (start → end → cost) — required only for the protagonist
## Current Status (Chapter 0 initial state)
## Relationship Network
## Inner Drive
## Growth Arc

至少 3 个主要角色（主角 + 主对手 + 主协作）。3-5 个次要角色；其简化卡仅需 4 个子标题。

=== SECTION: book_rules ===

一份纯 Markdown 规则卡。禁用 YAML、JSON、代码块。
## Protagonist (name + personality lock + behavioral constraints)
## Genre Lock (primary genre + forbidden mix-ins)
## Narrative POV
## Prohibited Elements

=== SECTION: pending_hooks ===

初始伏笔池（Markdown 表格），Phase 7 扩展列：
| hook_id | start_chapter | type | status | last_advanced | expected_payoff | payoff_timing | upstream_dependency | payoff_volume | core | half_life | notes |

- 建书阶段，第 5 列（last_advanced）统一填 0。
- 第 7 列（payoff_timing）取值之一：immediate / near-term / mid-term / slow-burn / endgame。
- 3-7 条 core_hook=true 的主线承重伏笔。

## 硬性完成检查
必须按顺序输出全部 5 个 SECTION 块。只有当 `pending_hooks` 的最后一行写完，交付物才算完成。"#,
        context_block = context_block,
        platform = format!("{:?}", book.platform).to_lowercase(),
        genre_name = genre_name,
        genre_id = book.genre,
        target_chapters = book.target_chapters,
        chapter_word_count = book.chapter_word_count,
        title = book.title,
        genre_body = genre_body,
    )
}

/// 解析 === SECTION: xxx === 分隔的输出
pub fn parse_sections(content: &str) -> Result<ArchitectOutput, AppError> {
    let mut output = ArchitectOutput::default();
    let sections = ["story_frame", "volume_map", "roles", "book_rules", "pending_hooks"];

    let mut found_sections = Vec::new();
    for section_name in &sections {
        let marker = format!("=== SECTION: {} ===", section_name);
        if let Some(start) = content.find(&marker) {
            let content_start = start + marker.len();
            // 找下一个 === SECTION: === 或文本结尾
            let section_end = sections
                .iter()
                .filter_map(|s| {
                    let m = format!("=== SECTION: {} ===", s);
                    content[content_start..].find(&m).map(|pos| content_start + pos)
                })
                .min()
                .unwrap_or(content.len());
            let section_content = content[content_start..section_end].trim();
            match *section_name {
                "story_frame" => output.story_frame = section_content.to_string(),
                "volume_map" => output.volume_map = section_content.to_string(),
                "roles" => output.roles = section_content.to_string(),
                "book_rules" => output.book_rules = section_content.to_string(),
                "pending_hooks" => output.pending_hooks = section_content.to_string(),
                _ => {}
            }
            found_sections.push(*section_name);
        }
    }

    let missing: Vec<&str> = sections
        .iter()
        .copied()
        .filter(|s| !found_sections.contains(s))
        .collect();
    if !missing.is_empty() {
        return Err(AppError::invalid_format(format!(
            "Architect output missing sections: {}",
            missing.join(", ")
        )));
    }

    Ok(output)
}

/// 将 ArchitectOutput 落盘到书籍目录。
///
/// 采用「先全部写 .tmp 再 rename」的两阶段提交：
/// 1. 第一阶段：所有目标文件写到 `<name>.md.tmp`，任意失败立即清理已写 tmp 文件并返回错误；
/// 2. 第二阶段：依次 rename 覆盖目标文件（同分区 rename 原子）。
/// 注意：roles 子目录文件较多，单独走 persist_roles（仍非事务性，但仅作 append-only 落盘）。
pub fn persist_output(book_dir: &std::path::Path, output: &ArchitectOutput) -> Result<(), AppError> {
    let story_dir = book_dir.join("story");
    let outline_dir = story_dir.join("outline");
    std::fs::create_dir_all(&outline_dir)?;

    // 第一阶段：写所有 .tmp
    let targets: [(std::path::PathBuf, &str); 4] = [
        (outline_dir.join("story_frame.md"), &output.story_frame),
        (outline_dir.join("volume_map.md"), &output.volume_map),
        (story_dir.join("book_rules.md"), &output.book_rules),
        (story_dir.join("pending_hooks.md"), &output.pending_hooks),
    ];
    let mut written_tmps: Vec<std::path::PathBuf> = Vec::with_capacity(targets.len());
    for (final_path, content) in &targets {
        let tmp_path = final_path.with_extension("md.tmp");
        if let Err(e) = std::fs::write(&tmp_path, content) {
            // 清理已写 tmp 文件
            for t in &written_tmps {
                let _ = std::fs::remove_file(t);
            }
            return Err(AppError::file_write_error(format!(
                "{}: {}",
                tmp_path.display(),
                e
            )));
        }
        written_tmps.push(tmp_path);
    }

    // 第二阶段：依次 rename（同分区原子）
    for (final_path, _) in &targets {
        let tmp_path = final_path.with_extension("md.tmp");
        if let Err(e) = std::fs::rename(&tmp_path, final_path) {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(AppError::file_write_error(format!(
                "{}: {}",
                final_path.display(),
                e
            )));
        }
    }

    // roles 需要按 ---ROLE--- 分隔拆分为一人一文件
    persist_roles(&story_dir, &output.roles)?;

    Ok(())
}

/// 解析 roles SECTION 并按角色拆分文件
fn persist_roles(story_dir: &std::path::Path, roles_content: &str) -> Result<(), AppError> {
    let roles_major = story_dir.join("roles").join("主要角色");
    let roles_minor = story_dir.join("roles").join("次要角色");
    std::fs::create_dir_all(&roles_major)?;
    std::fs::create_dir_all(&roles_minor)?;

    // 按 ---ROLE--- 分割
    let parts: Vec<&str> = roles_content.split("---ROLE---").collect();
    for part in parts.iter().skip(1) {
        // 解析 tier 和 name
        let (tier, name, content) = parse_role_block(part)?;
        if name.is_empty() {
            continue;
        }
        let dir = if tier == "major" { &roles_major } else { &roles_minor };
        let file_path = dir.join(format!("{}.md", sanitize_filename(&name)));
        std::fs::write(file_path, content)?;
    }
    Ok(())
}

fn parse_role_block(block: &str) -> Result<(String, String, String), AppError> {
    let mut tier = String::new();
    let mut name = String::new();
    let mut content_start = 0;

    for (i, line) in block.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("tier:") {
            tier = trimmed[5..].trim().to_string();
        } else if trimmed.starts_with("name:") {
            name = trimmed[5..].trim().to_string();
        } else if trimmed.starts_with("---CONTENT---") {
            content_start = block.find("---CONTENT---").unwrap_or(0) + "---CONTENT---".len();
            break;
        }
        if i > 10 {
            break;
        }
    }

    let content = if content_start > 0 {
        block[content_start..].trim().to_string()
    } else {
        block.trim().to_string()
    };

    Ok((tier, name, content))
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_five_sections() {
        let content = r#"
=== SECTION: story_frame ===
故事框架内容

=== SECTION: volume_map ===
卷地图内容

=== SECTION: roles ===
---ROLE---
tier: major
name: 主角
---CONTENT---
角色卡

=== SECTION: book_rules ===
规则卡

=== SECTION: pending_hooks ===
伏笔池
"#;
        let output = parse_sections(content).unwrap();
        assert_eq!(output.story_frame, "故事框架内容");
        assert_eq!(output.volume_map, "卷地图内容");
        assert!(output.roles.contains("主角"));
        assert_eq!(output.book_rules, "规则卡");
        assert_eq!(output.pending_hooks, "伏笔池");
    }

    #[test]
    fn rejects_missing_sections() {
        let content = "=== SECTION: story_frame ===\n只有一段";
        assert!(parse_sections(content).is_err());
    }
}
