// Consolidator Agent。
//
// 职责：将已完成卷的逐章摘要压缩为卷级叙事摘要，降低长篇创作中的 token 占用。
// 仅处理已完成卷（end_ch <= 当前最新章节），当前进行中卷的详细摘要保留原状。
//
// 流程：
// 1. 重新执行 hook 晋升（rerun_promotion_pass）：基于 chapter_summaries.md
//    统计 advancedCount，达到阈值的 hook 翻转 promoted=true 并写回 pending_hooks.md
// 2. 读取 volume_map.md + chapter_summaries.md
// 3. 解析卷边界 + 摘要表
// 4. 对每个已完成卷，LLM 压缩为叙事段落
// 5. 归档已完成卷的详细摘要，仅保留当前卷的行

use crate::core::agent::engine::AgentEngine;
use crate::domain::pipeline::types::Language;
use crate::domain::pipeline::utils::hook_promotion::rerun_promotion_pass;
use crate::domain::pipeline::utils::story_markdown::{
    parse_pending_hooks_markdown, render_hooks_markdown,
};
use crate::shared::error::AppError;

/// Consolidator 输出
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConsolidationResult {
    pub volume_summaries: String,
    pub archived_volumes: u32,
    pub retained_chapters: u32,
    /// 本次运行中被晋升（promoted 由 false/None → true）的 hook 数量。
    /// pending_hooks.md 不存在或无 hook 越过阈值时为 0。
    pub promoted_hook_count: u32,
}

/// 卷边界（从 volume_map.md 解析）
struct VolumeBoundary {
    name: String,
    start_ch: u32,
    end_ch: u32,
}

/// 章节摘要行（从 chapter_summaries.md 解析）
struct SummaryRow {
    chapter: u32,
    raw: String,
}

/// 压缩已完成卷的章节摘要为卷级叙事摘要。
///
/// 流程：
/// 1. 重新执行 hook 晋升（rerun_promotion_pass）
/// 2. 读取 volume_map.md + chapter_summaries.md
/// 3. 解析卷边界 + 摘要表
/// 4. 对每个已完成卷，LLM 压缩为叙事段落
/// 5. 归档已完成卷的详细摘要，仅保留当前卷的行
pub async fn consolidate(
    engine: &AgentEngine,
    book_dir: &std::path::Path,
) -> Result<ConsolidationResult, AppError> {
    let story_dir = book_dir.join("story");
    let summaries_path = story_dir.join("chapter_summaries.md");
    let volume_summaries_path = story_dir.join("volume_summaries.md");

    // 读取文件（同步，缺失返回空字符串）
    let summaries_raw = std::fs::read_to_string(&summaries_path).unwrap_or_default();
    let outline_raw = std::fs::read_to_string(story_dir.join("outline").join("volume_map.md"))
        .or_else(|_| std::fs::read_to_string(story_dir.join("volume_outline.md")))
        .unwrap_or_default();

    // Phase 7 hotfix 2：归档前的 hook 晋升重跑。独立于摘要压缩执行，
    // 即使是尚无已完成卷的新书，也会在 seed 的 advanced_count 越过阈值时
    // 翻转 promoted 标志。
    let promoted_hook_count = rerun_advanced_count_promotion(&story_dir, &summaries_raw)?;

    // 任一为空则提前返回
    if summaries_raw.is_empty() || outline_raw.is_empty() {
        return Ok(ConsolidationResult {
            volume_summaries: String::new(),
            archived_volumes: 0,
            retained_chapters: 0,
            promoted_hook_count,
        });
    }

    let volumes = parse_volume_boundaries(&outline_raw);
    let (header, rows) = parse_summary_table(&summaries_raw);

    // 无卷边界或无摘要行 → 无需压缩
    if volumes.is_empty() || rows.is_empty() {
        return Ok(ConsolidationResult {
            volume_summaries: String::new(),
            archived_volumes: 0,
            retained_chapters: rows.len() as u32,
            promoted_hook_count,
        });
    }

    // 已完成卷 = end_ch <= 最新章节号
    let last_chapter = rows.last().map(|r| r.chapter).unwrap_or(0);
    let completed_volumes: Vec<&VolumeBoundary> = volumes
        .iter()
        .filter(|v| v.end_ch <= last_chapter)
        .collect();

    if completed_volumes.is_empty() {
        return Ok(ConsolidationResult {
            volume_summaries: String::new(),
            archived_volumes: 0,
            retained_chapters: rows.len() as u32,
            promoted_hook_count,
        });
    }

    // 对每个已完成卷做 LLM 压缩
    let mut volume_summaries: Vec<String> = Vec::new();
    for vol in &completed_volumes {
        let vol_rows: Vec<&SummaryRow> = rows
            .iter()
            .filter(|r| r.chapter >= vol.start_ch && r.chapter <= vol.end_ch)
            .collect();
        if vol_rows.is_empty() {
            continue;
        }
        let rows_text: String = vol_rows
            .iter()
            .map(|r| r.raw.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let summary = consolidate_volume(engine, vol, &header, &rows_text).await?;
        volume_summaries.push(format!(
            "## {}（第{}-{}章）\n\n{}",
            vol.name, vol.start_ch, vol.end_ch, summary
        ));
    }

    let new_summaries = volume_summaries.join("\n\n");

    // 写入 volume_summaries.md
    std::fs::write(&volume_summaries_path, &new_summaries)?;

    // 归档已完成卷的详细摘要
    let archive_dir = story_dir.join("summaries_archive");
    std::fs::create_dir_all(&archive_dir)?;
    for vol in &completed_volumes {
        let vol_rows_raw: String = rows
            .iter()
            .filter(|r| r.chapter >= vol.start_ch && r.chapter <= vol.end_ch)
            .map(|r| r.raw.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let archive_path = archive_dir.join(format!("vol_{}-{}.md", vol.start_ch, vol.end_ch));
        std::fs::write(
            archive_path,
            format!("# {}\n\n{}\n{}", vol.name, header, vol_rows_raw),
        )?;
    }

    // 重写 chapter_summaries.md，仅保留不属于任何已完成卷的行（当前卷 + 尾部）
    let retained_rows: Vec<&SummaryRow> = rows
        .iter()
        .filter(|r| {
            !completed_volumes
                .iter()
                .any(|v| r.chapter >= v.start_ch && r.chapter <= v.end_ch)
        })
        .collect();
    let retained_content = if retained_rows.is_empty() {
        format!("{}\n", header)
    } else {
        format!(
            "{}\n{}\n",
            header,
            retained_rows
                .iter()
                .map(|r| r.raw.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    std::fs::write(&summaries_path, &retained_content)?;

    Ok(ConsolidationResult {
        volume_summaries: new_summaries,
        archived_volumes: completed_volumes.len() as u32,
        retained_chapters: retained_rows.len() as u32,
        promoted_hook_count,
    })
}

/// Phase 7 hotfix 2 — 重跑 advancedCount 晋升。
///
/// 当 pending_hooks.md 中的 seed hook 在历史章节中累计被推进 ≥ 2 次时，
/// 将其 `promoted` 标志翻转为 true 并写回。返回本次翻转的 hook 数量。
///
/// 算法委托给纯函数 `rerun_promotion_pass`；本函数负责文件 I/O 与语言检测。
///
/// - `story_dir`：`<book>/story` 目录
/// - `summaries_raw`：已读取的 chapter_summaries.md 内容（避免重复读盘）
fn rerun_advanced_count_promotion(story_dir: &std::path::Path, summaries_raw: &str) -> Result<u32, AppError> {
    let ledger_path = story_dir.join("pending_hooks.md");
    let raw = match std::fs::read_to_string(&ledger_path) {
        Ok(s) if !s.trim().is_empty() => s,
        // 文件缺失或空 → 无需晋升，返回 0
        _ => return Ok(0),
    };

    // 语言检测：含 CJK 字符视为 zh，否则 en
    let language = if raw.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)) {
        Language::Zh
    } else {
        Language::En
    };

    let hooks = parse_pending_hooks_markdown(&raw, language);
    if hooks.is_empty() {
        return Ok(0);
    }

    let result = rerun_promotion_pass(&hooks, summaries_raw);
    if !result.updated {
        return Ok(0);
    }

    let rendered = render_hooks_markdown(&result.hooks, language);
    std::fs::write(&ledger_path, rendered)
        .map_err(|e| AppError::file_write_error(format!("{}: {}", ledger_path.display(), e)))?;
    Ok(result.flipped_count)
}

/// Compress a single volume via one LLM pass; return the narrative paragraph.
async fn consolidate_volume(
    engine: &AgentEngine,
    vol: &VolumeBoundary,
    header: &str,
    rows: &str,
) -> Result<String, AppError> {
    let system_prompt = r###"<identity>
你是一名叙事摘要专家。将逐章摘要压缩为一段连贯的叙事段落（不超过 500 字），保留关键事件、角色发展与情节推进。保留具体人名、地名与情节点。使用与输入相同的语言撰写。
</identity>

<safety>
- 绝不（NEVER）丢弃关键事件、角色姓名或重要情节点：摘要可以精简，但不可丢失叙事骨架。
- 绝不（NEVER）编造正文章节摘要中未出现的事件、对话或人物关系。
- 绝不（NEVER）改变原作语种：输入为中文则输出中文，输入为英文则输出英文。
</safety>

<examples>
正确：
- 将 30 章逐章摘要压缩为一段约 400 字的叙事段落，按时间顺序串联主线事件，保留主角姓名、关键反派、核心冲突与转折点。

错误：
- 仅罗列"第 1 章发生 X，第 2 章发生 Y……"的流水账，未融合为连贯叙事。
- 为凑字数凭空补充原文未提及的感情线或支线。
- 输入为中文却用英文撰写摘要。
</examples>

<verification>
完成后自检：
1. 摘要是否为单段连贯叙事（非逐章罗列），且不超过 500 字。
2. 关键人名、地名、核心事件、情节转折是否均被保留。
3. 是否与输入同语种。
4. 是否未引入原文未出现的内容。
</verification>"###;
    let user_message = format!(
        r###"卷：{name}（第 {start_ch}-{end_ch} 章）

章节摘要：
{header}
{rows}"###,
        name = vol.name,
        start_ch = vol.start_ch,
        end_ch = vol.end_ch,
        header = header,
        rows = rows,
    );
    let response = engine.prompt_once(system_prompt, &user_message).await?;
    Ok(response.trim().to_string())
}

// ── 解析：卷边界 ────────────────────────────────────────────

/// 解析 volume_map.md，提取卷边界。
/// 仅识别以 `#` 开头且包含 `第X卷` 或 `Volume N` 的标题行，
/// 并从同一行提取章节范围（如 `第1-30章`、`Chapters 1-30`、`(1-30)`）。
fn parse_volume_boundaries(outline: &str) -> Vec<VolumeBoundary> {
    let mut result = Vec::new();
    for line in outline.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('#') {
            continue;
        }
        // 去掉前导 # 与空白
        let header = trimmed.trim_start_matches('#').trim();
        if !is_volume_header(header) {
            continue;
        }
        if let Some((name, start_ch, end_ch)) = extract_volume_header(header) {
            if !name.is_empty() {
                result.push(VolumeBoundary {
                    name,
                    start_ch,
                    end_ch,
                });
            }
        }
    }
    result
}

/// 判断是否为卷标题：`第X卷`（中文）或 `Volume N`（英文）
fn is_volume_header(header: &str) -> bool {
    if header.starts_with('第') && header.contains('卷') {
        return true;
    }
    if header.starts_with("Volume") {
        return true;
    }
    false
}

/// 从卷标题行提取名称与章节范围。
/// 返回 (name, start_ch, end_ch)；无范围时返回 None。
fn extract_volume_header(header: &str) -> Option<(String, u32, u32)> {
    let (range_byte_pos, start_ch, end_ch) = find_range(header)?;
    let before_range = &header[..range_byte_pos];
    // 优先按开括号切分：`第一卷 起源（第1-30章）` → `第一卷 起源`
    let name = if let Some(bpos) = before_range.rfind(['（', '(']) {
        before_range[..bpos].trim().to_string()
    } else {
        // 无括号时剥离尾部范围前缀词（第 / Chapters / Ch. 等）
        strip_range_prefix(before_range)
    };
    Some((name, start_ch, end_ch))
}

/// 在字符串中查找首个 `数字 [-–~] 数字` 模式。
/// 破折号兼容：`-`(U+002D) / `–`(U+2013 en-dash) / `~`(U+007E)。
/// 返回 (范围起始数字的字节位置, start, end)。
fn find_range(line: &str) -> Option<(usize, u32, u32)> {
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].1.is_ascii_digit() {
            let byte_start = chars[i].0;
            let mut j = i;
            while j < chars.len() && chars[j].1.is_ascii_digit() {
                j += 1;
            }
            let start_num: u32 = chars[i..j]
                .iter()
                .map(|(_, c)| *c)
                .collect::<String>()
                .parse()
                .ok()?;
            // 跳过空格
            let mut k = j;
            while k < chars.len() && chars[k].1 == ' ' {
                k += 1;
            }
            // 破折号：`-` / `–` / `~`
            if k < chars.len() && (chars[k].1 == '-' || chars[k].1 == '–' || chars[k].1 == '~') {
                let mut m = k + 1;
                while m < chars.len() && chars[m].1 == ' ' {
                    m += 1;
                }
                if m < chars.len() && chars[m].1.is_ascii_digit() {
                    let mut n = m;
                    while n < chars.len() && chars[n].1.is_ascii_digit() {
                        n += 1;
                    }
                    let end_num: u32 = chars[m..n]
                        .iter()
                        .map(|(_, c)| *c)
                        .collect::<String>()
                        .parse()
                        .ok()?;
                    return Some((byte_start, start_num, end_num));
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    None
}

/// 剥离尾部的范围前缀词（`第` / `Chapters` / `Chapter` / `Ch.` / `Ch`）与开括号、空白。
fn strip_range_prefix(name: &str) -> String {
    let mut chars: Vec<char> = name.chars().collect();
    loop {
        // 剥离尾部空白
        while matches!(chars.last(), Some(c) if c.is_whitespace()) {
            chars.pop();
        }
        // 剥离尾部开括号
        if matches!(chars.last(), Some(c) if *c == '（' || *c == '(') {
            chars.pop();
            continue;
        }
        // 剥离尾部范围前缀词
        let stripped = pop_suffix_if_match(&mut chars, &['C', 'h', 'a', 'p', 't', 'e', 'r', 's'])
            || pop_suffix_if_match(&mut chars, &['C', 'h', 'a', 'p', 't', 'e', 'r'])
            || pop_suffix_if_match(&mut chars, &['C', 'h', '.'])
            || pop_suffix_if_match(&mut chars, &['C', 'h'])
            || pop_suffix_if_match(&mut chars, &['第']);
        if !stripped {
            break;
        }
    }
    chars.iter().collect()
}

/// 若 `chars` 尾部匹配 `suffix` 则弹出并返回 true。
fn pop_suffix_if_match(chars: &mut Vec<char>, suffix: &[char]) -> bool {
    let n = suffix.len();
    if chars.len() >= n && &chars[chars.len() - n..] == suffix {
        chars.truncate(chars.len() - n);
        true
    } else {
        false
    }
}

// ── 解析：摘要表 ────────────────────────────────────────────

/// 解析 chapter_summaries.md。
/// 返回 (表头字符串, 数据行列表)。表头 = 以 `|` 开头且含 `章节`/`Chapter`/`---` 的行。
fn parse_summary_table(raw: &str) -> (String, Vec<SummaryRow>) {
    if raw.trim().is_empty() {
        return (String::new(), Vec::new());
    }
    let mut header_lines: Vec<String> = Vec::new();
    let mut rows: Vec<SummaryRow> = Vec::new();
    for line in raw.lines() {
        if !line.starts_with('|') {
            continue;
        }
        if is_header_line(line) {
            header_lines.push(line.to_string());
        } else if let Some(chapter) = parse_first_cell_chapter(line) {
            rows.push(SummaryRow {
                chapter,
                raw: line.to_string(),
            });
        }
    }
    (header_lines.join("\n"), rows)
}

/// 判断是否为表头行：含 `章节` / `Chapter` / `---`
fn is_header_line(line: &str) -> bool {
    line.contains("章节") || line.contains("Chapter") || line.contains("---")
}

/// 解析表格行首个单元格中的章节号。
fn parse_first_cell_chapter(line: &str) -> Option<u32> {
    let after_pipe = line.trim_start_matches('|');
    let first_cell = after_pipe.split('|').next().unwrap_or("").trim();
    let num_str: String = first_cell.chars().filter(|c| c.is_ascii_digit()).collect();
    if num_str.is_empty() {
        return None;
    }
    num_str.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_volume_boundaries_chinese() {
        let outline = "## 第一卷 起源（第1-30章）\n内容...";
        let vols = parse_volume_boundaries(outline);
        assert_eq!(vols.len(), 1);
        assert_eq!(vols[0].name, "第一卷 起源");
        assert_eq!(vols[0].start_ch, 1);
        assert_eq!(vols[0].end_ch, 30);
    }

    #[test]
    fn parses_volume_boundaries_english() {
        let outline = "# Volume 1 (Chapters 1-30)\ncontent...";
        let vols = parse_volume_boundaries(outline);
        assert_eq!(vols.len(), 1);
        assert_eq!(vols[0].name, "Volume 1");
        assert_eq!(vols[0].start_ch, 1);
        assert_eq!(vols[0].end_ch, 30);
    }

    #[test]
    fn parses_volume_boundaries_no_range() {
        let outline = "## 第一卷 起源\n无范围";
        let vols = parse_volume_boundaries(outline);
        assert!(vols.is_empty(), "无范围的卷标题应被跳过");
    }

    #[test]
    fn parses_summary_table() {
        let raw = "| 章节 | 摘要 |\n|---|---|\n| 1 | 主角觉醒 |\n| 2 | 初遇对手 |";
        let (header, rows) = parse_summary_table(raw);
        assert!(header.contains("章节"));
        assert!(header.contains("---"));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].chapter, 1);
        assert_eq!(rows[1].chapter, 2);
        assert!(rows[0].raw.contains("主角觉醒"));
    }

    #[test]
    fn parses_summary_table_empty() {
        let (header, rows) = parse_summary_table("");
        assert!(header.is_empty());
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn consolidate_returns_empty_when_files_missing() {
        use crate::infrastructure::db::connection::Database;
        use crate::infrastructure::fs::data_dir::DataDir;
        use crate::infrastructure::llm::registry::ProviderRegistry;
        use std::path::PathBuf;

        let registry = ProviderRegistry::empty();
        let db = Database::connect_in_memory().expect("in-memory db");
        let tmp_data = tempfile::tempdir().expect("tempdir for data_dir");
        let data_dir = DataDir::new(tmp_data.path().to_path_buf());
        let engine = AgentEngine::new(registry, db, data_dir, PathBuf::new(), None);

        // 不存在的书籍目录 → 文件读取失败 → 提前返回空结果
        let tmp = tempfile::tempdir().expect("tempdir");
        let nonexistent = tmp.path().join("no_such_book");
        let result = consolidate(&engine, &nonexistent).await.expect("should not error");
        assert!(result.volume_summaries.is_empty());
        assert_eq!(result.archived_volumes, 0);
        assert_eq!(result.retained_chapters, 0);
    }
}
