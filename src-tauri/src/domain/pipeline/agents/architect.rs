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
        "请为标题为\"{}\"的{}小说生成完整基础设定。",
        book.title, genre_name
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
        Some(ctx) if !ctx.trim().is_empty() => format!("\n\n## 外部指令\n以下是来自外部系统的创作指令，请将其融入设定中：\n\n{}", ctx),
        _ => String::new(),
    };

    format!(
        r#"你是这本书的总架构师。你的唯一输出是**散文密度的基础设定**——不是表格、不是 schema、不是条目化 bullet。你的散文密度决定了后面 planner 能不能读出"稀疏 memo"，writer 能不能写出活人，reviewer 能不能校准硬伤。{context_block}

## 书籍元信息
- 平台：{platform}
- 题材：{genre_name}（{genre_id}）
- 目标章数：{target_chapters}章
- 每章字数：{chapter_word_count}字
- 标题：{title}

## 题材底色
{genre_body}

## 输出结构（5 个 SECTION，严格按 === SECTION: === 分块，不要漏任何一块）

## 去重铁律
禁止在多段里重复同一事实。主角弧线只写在 roles；世界铁律只写在 story_frame.世界观底色；节奏原则只写在 volume_map 最后一段；角色当前现状只写在 roles.当前现状；初始钩子只写在 pending_hooks（startChapter=0 行）。

## 预算（超预算必删）
- story_frame ≤ 3000 chars
- volume_map ≤ 5000 chars
- roles 总 ≤ 8000 chars
- book_rules ≤ 1000 chars
- pending_hooks ≤ 2000 chars

=== SECTION: story_frame ===

散文骨架，**4 段**，每段约 600-900 字，不要写表格，不要写 bullet list。段落标题用 ## 开头。

### 段 1：主题与基调
这本书讲的是什么——具体的命题，不是"主角如何从弱到强"的空话。基调是什么？为什么？

### 段 2：核心冲突、对手定性、前台/后台双层故事
主要矛盾是什么？主要对手是谁（至少 2 个）？必须显式写出"前台故事/后台故事"两条线：前台是每章看到的表层冲突，后台是贯穿全书的暗线。两者必须有因果关联。

### 段 3：世界观底色（铁律 + 质感）
3-5 条不可违反的铁律，以 prose 写出。世界质感是什么——湿的还是干的、快的还是慢的？

### 段 4：终局方向 + 全书 Objective
最后一个镜头大致长什么样。末尾必须明确写出全书 Objective 一句话：主角必须达成一个**可验证的终局状态**。

=== SECTION: volume_map ===

分卷散文地图，**5 段主体 + 1 段节奏原则尾段**。只写到卷级 prose，禁止指定具体章号任务。

### 段 1：各卷主题与情绪曲线
### 段 2：卷间钩子与回收承诺（前台/后台双层都要覆盖）
### 段 3：各卷 OKR（Objective + Key Results，每卷 3 条可量化 KR）
### 段 4：卷尾必须发生的改变
### 段 5：节奏原则（至少 3 条具体化到本书，6 条各写 2-3 句）

=== SECTION: roles ===

一人一卡 prose。主角卡是角色弧线的唯一权威来源。用 ---ROLE--- 分隔：

---ROLE---
tier: major
name: <角色名>
---CONTENT---
## 核心标签（3-5 个关键词 + 一句话）
## 反差细节（1-2 个与核心标签反差的具体细节）
## 人物小传（过往经历）
## 主角弧线（起点 → 终点 → 代价）——只有主角必须写
## 当前现状（第 0 章初始状态）
## 关系网络
## 内在驱动
## 成长弧光

主要角色至少 3 个（主角 + 主要对手 + 主要协作者）。次要角色 3-5 个，简化版只需 4 个小标题。

=== SECTION: book_rules ===

普通 Markdown 规则卡，不要 YAML/JSON/代码块。
## 主角（名字 + 性格锁 + 行为约束）
## 题材锁（主类型 + 禁止混入）
## 叙事人称
## 禁止事项

=== SECTION: pending_hooks ===

初始伏笔池（Markdown 表格），Phase 7 扩展列：
| hook_id | 起始章节 | 类型 | 状态 | 最近推进 | 预期回收 | 回收节奏 | 上游依赖 | 回收卷 | 核心 | 半衰期 | 备注 |

- 建书阶段第 5 列统一填 0
- 第 7 列必须填写：立即 / 近期 / 中程 / 慢烧 / 终局 之一
- core_hook=true 的主线承重伏笔 3-7 条

## 硬性完结检查
必须依次输出全部 5 个 SECTION 块。只有写完 pending_hooks 最后一行才算交付。"#,
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

/// 将 ArchitectOutput 落盘到书籍目录
pub fn persist_output(book_dir: &std::path::Path, output: &ArchitectOutput) -> Result<(), AppError> {
    let story_dir = book_dir.join("story");
    let outline_dir = story_dir.join("outline");
    std::fs::create_dir_all(&outline_dir)?;

    std::fs::write(outline_dir.join("story_frame.md"), &output.story_frame)?;
    std::fs::write(outline_dir.join("volume_map.md"), &output.volume_map)?;
    std::fs::write(story_dir.join("book_rules.md"), &output.book_rules)?;
    std::fs::write(story_dir.join("pending_hooks.md"), &output.pending_hooks)?;

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
