// 编辑事务控制（EditTransaction Controller）。
//
// 6 种 EditRequest：
// - EntityRename: 全书范围内将 oldValue 替换为 newValue（涉及内容 + 文件改名）
// - ChapterRewrite: LLM 重写整章（本模块只做 plan，执行由 pipeline_revise_draft 处理）
// - ChapterReplace: 整章替换为新内容
// - ChapterLocalEdit: 在章节内查找并替换目标文本（三级匹配：精确 → 弹性空格 → 段落近似）
// - TruthFileEdit: 编辑真相文件（白名单内）
// - FocusEdit: 更新 current_focus.md
//
// 执行流程：plan_edit_transaction(request) → execute_edit_transaction(planned, data_dir)
// - plan 阶段：分类 affected_scope / requires_truth_rebuild / truth_authority
// - execute 阶段：实际读写文件，返回 touched_files + review_required + summary
//
// 安全约束：
// - 所有路径必须通过 DataDir.books_dir() 构造
// - book_id 走 validate_id 校验
// - truth 文件名走 assert_safe_truth_file_name 校验（白名单）
// - entity rename 拒绝路径分隔符（防止单组件逃逸目录）

use std::path::{Path, PathBuf};

use crate::domain::interaction::truth_authority::{assert_safe_truth_file_name, classify_truth_authority, normalize_truth_file_name};
use crate::domain::interaction::types::TruthAuthority;
use crate::infrastructure::fs::data_dir::DataDir;
use crate::infrastructure::validation::validate_id;
use crate::shared::error::AppError;

// ── 编辑请求（6 种） ─────────────────────────────────────────

/// 编辑请求。涵盖实体改名、章节重写/替换/局部编辑、真相文件编辑、焦点更新。
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum EditRequest {
    /// 实体改名（全书范围 oldValue → newValue，含文件改名）
    #[serde(rename_all = "camelCase")]
    EntityRename {
        book_id: String,
        old_name: String,
        new_name: String,
    },
    /// 章节重写（LLM 驱动，本模块只 plan；执行需调用 pipeline_revise_draft）
    #[serde(rename_all = "camelCase")]
    ChapterRewrite {
        book_id: String,
        chapter_number: u32,
        instruction: String,
    },
    /// 章节整章替换（用 full_text 覆盖原内容）
    #[serde(rename_all = "camelCase")]
    ChapterReplace {
        book_id: String,
        chapter_number: u32,
        new_content: String,
    },
    /// 章节局部编辑（在章节内查找 find，替换为 replace）
    #[serde(rename_all = "camelCase")]
    ChapterLocalEdit {
        book_id: String,
        chapter_number: u32,
        find: String,
        replace: String,
    },
    /// 真相文件编辑（白名单内 file_name，覆盖为 new_content）
    #[serde(rename_all = "camelCase")]
    TruthFileEdit {
        book_id: String,
        file_name: String,
        new_content: String,
    },
    /// 焦点更新（current_focus.md，本质是 TruthFileEdit 的特化）
    #[serde(rename_all = "camelCase")]
    FocusEdit {
        book_id: String,
        new_focus: String,
    },
}

impl EditRequest {
    pub fn book_id(&self) -> &str {
        match self {
            EditRequest::EntityRename { book_id, .. }
            | EditRequest::ChapterRewrite { book_id, .. }
            | EditRequest::ChapterReplace { book_id, .. }
            | EditRequest::ChapterLocalEdit { book_id, .. }
            | EditRequest::TruthFileEdit { book_id, .. }
            | EditRequest::FocusEdit { book_id, .. } => book_id,
        }
    }

    pub fn chapter_number(&self) -> Option<u32> {
        match self {
            EditRequest::ChapterRewrite { chapter_number, .. }
            | EditRequest::ChapterReplace { chapter_number, .. }
            | EditRequest::ChapterLocalEdit { chapter_number, .. } => Some(*chapter_number),
            _ => None,
        }
    }
}

// ── 编辑事务类型字符串 ───────────────────────────────────────

/// 编辑事务类型（用于 PlannedEditTransaction.transaction_type 字段）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditTransactionType {
    EntityRename,
    ChapterRewrite,
    ChapterReplace,
    ChapterLocalEdit,
    TruthFileEdit,
    FocusEdit,
}

impl EditTransactionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EditTransactionType::EntityRename => "entity-rename",
            EditTransactionType::ChapterRewrite => "chapter-rewrite",
            EditTransactionType::ChapterReplace => "chapter-replace",
            EditTransactionType::ChapterLocalEdit => "chapter-local-edit",
            EditTransactionType::TruthFileEdit => "truth-file-edit",
            EditTransactionType::FocusEdit => "focus-edit",
        }
    }
}

// ── 影响范围 ─────────────────────────────────────────────────

/// 编辑事务影响范围。决定是否需要触发后续 review/rebuild。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditAffectedScope {
    /// 单章
    Chapter,
    /// 下游章节
    Downstream,
    /// 未来章节
    Future,
    /// 整本书
    Book,
}

impl EditAffectedScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            EditAffectedScope::Chapter => "chapter",
            EditAffectedScope::Downstream => "downstream",
            EditAffectedScope::Future => "future",
            EditAffectedScope::Book => "book",
        }
    }
}

// ── 已规划事务 ───────────────────────────────────────────────

/// plan 阶段产物。描述事务的影响范围与是否需要真相重建。
#[derive(Debug, Clone)]
pub struct PlannedEditTransaction {
    pub request: EditRequest,
    pub transaction_type: EditTransactionType,
    pub affected_scope: EditAffectedScope,
    pub requires_truth_rebuild: bool,
    pub truth_authority: Option<TruthAuthority>,
    /// 规范化后的真相文件名（仅 TruthFileEdit / FocusEdit 有值）
    pub normalized_file_name: Option<String>,
}

// ── 已执行事务 ───────────────────────────────────────────────

/// execute 阶段产物。记录实际变更的文件与是否需要人工 review。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutedEditTransaction {
    pub transaction_type: String,
    pub book_id: String,
    /// 章节号（chapter 相关事务有值）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_number: Option<u32>,
    /// 实际写入的文件（相对于 book_dir 的相对路径）
    pub touched_files: Vec<String>,
    /// 是否需要人工 review
    pub review_required: bool,
    /// 可读的执行摘要
    pub summary: String,
}

// ── plan 阶段 ────────────────────────────────────────────────

/// 规划编辑事务。根据 EditRequest 推导 affected_scope / requires_truth_rebuild / truth_authority。
///
/// 注意：plan 阶段不做文件存在性检查，只做静态推导。
pub fn plan_edit_transaction(request: EditRequest) -> Result<PlannedEditTransaction, AppError> {
    // 校验 book_id
    validate_id(request.book_id(), "book_id").map_err(AppError::invalid_input)?;

    match &request {
        EditRequest::EntityRename { .. } => Ok(PlannedEditTransaction {
            request,
            transaction_type: EditTransactionType::EntityRename,
            affected_scope: EditAffectedScope::Book,
            requires_truth_rebuild: true,
            truth_authority: None,
            normalized_file_name: None,
        }),
        EditRequest::ChapterRewrite { .. } => Ok(PlannedEditTransaction {
            request,
            transaction_type: EditTransactionType::ChapterRewrite,
            affected_scope: EditAffectedScope::Downstream,
            requires_truth_rebuild: true,
            truth_authority: None,
            normalized_file_name: None,
        }),
        EditRequest::ChapterReplace { .. } => Ok(PlannedEditTransaction {
            request,
            transaction_type: EditTransactionType::ChapterReplace,
            affected_scope: EditAffectedScope::Chapter,
            requires_truth_rebuild: true,
            truth_authority: None,
            normalized_file_name: None,
        }),
        EditRequest::ChapterLocalEdit { .. } => Ok(PlannedEditTransaction {
            request,
            transaction_type: EditTransactionType::ChapterLocalEdit,
            affected_scope: EditAffectedScope::Chapter,
            requires_truth_rebuild: true,
            truth_authority: None,
            normalized_file_name: None,
        }),
        EditRequest::TruthFileEdit { file_name, .. } => {
            assert_safe_truth_file_name(file_name)?;
            let normalized = normalize_truth_file_name(file_name)
                .ok_or_else(|| AppError::invalid_input("非法真相文件名"))?;
            let authority = classify_truth_authority(&normalized)
                .ok_or_else(|| AppError::invalid_input("无法分类真相文件权威级别"))?;
            let requires_rebuild = matches!(
                authority,
                TruthAuthority::RuntimeTruth | TruthAuthority::Memory
            );
            Ok(PlannedEditTransaction {
                request,
                transaction_type: EditTransactionType::TruthFileEdit,
                affected_scope: EditAffectedScope::Book,
                requires_truth_rebuild: requires_rebuild,
                truth_authority: Some(authority),
                normalized_file_name: Some(normalized),
            })
        }
        EditRequest::FocusEdit { .. } => Ok(PlannedEditTransaction {
            request,
            transaction_type: EditTransactionType::FocusEdit,
            affected_scope: EditAffectedScope::Future,
            requires_truth_rebuild: false,
            truth_authority: Some(TruthAuthority::Direction),
            normalized_file_name: Some("current_focus.md".to_string()),
        }),
    }
}

// ── execute 阶段 ─────────────────────────────────────────────

/// 执行已规划的编辑事务。
///
/// ChapterRewrite 不在此处执行（需要 LLM 驱动，由 commands.rs 委派给 pipeline_revise_draft），
/// 调用方应在 plan 之后判断 transaction_type，对 ChapterRewrite 走单独路径。
pub fn execute_edit_transaction(
    planned: PlannedEditTransaction,
    data_dir: &DataDir,
) -> Result<ExecutedEditTransaction, AppError> {
    let book_id = planned.request.book_id().to_string();
    let book_dir = data_dir.books_dir().join(&book_id);

    if !book_dir.exists() {
        return Err(AppError::file_not_found(format!("book dir: {}", book_id)));
    }

    match planned.transaction_type {
        EditTransactionType::EntityRename => {
            let (old_name, new_name) = match &planned.request {
                EditRequest::EntityRename { old_name, new_name, .. } => (old_name.clone(), new_name.clone()),
                _ => unreachable!(),
            };
            execute_entity_rename(&book_dir, &book_id, &old_name, &new_name)
        }
        EditTransactionType::ChapterRewrite => Err(AppError::not_implemented(
            "ChapterRewrite 需要 LLM 驱动，请通过 pipeline_revise_draft 执行",
        )),
        EditTransactionType::ChapterReplace => {
            let (chapter_number, new_content) = match &planned.request {
                EditRequest::ChapterReplace { chapter_number, new_content, .. } => (*chapter_number, new_content.clone()),
                _ => unreachable!(),
            };
            execute_chapter_replace(&book_dir, &book_id, chapter_number, &new_content)
        }
        EditTransactionType::ChapterLocalEdit => {
            let (chapter_number, find, replace) = match &planned.request {
                EditRequest::ChapterLocalEdit { chapter_number, find, replace, .. } => (*chapter_number, find.clone(), replace.clone()),
                _ => unreachable!(),
            };
            execute_chapter_local_edit(&book_dir, &book_id, chapter_number, &find, &replace)
        }
        EditTransactionType::TruthFileEdit => {
            let (file_name, new_content) = match &planned.request {
                EditRequest::TruthFileEdit { file_name, new_content, .. } => (file_name.clone(), new_content.clone()),
                _ => unreachable!(),
            };
            let normalized = normalize_truth_file_name(&file_name)
                .ok_or_else(|| AppError::invalid_input("非法真相文件名"))?;
            execute_truth_file_edit(&book_dir, &book_id, &normalized, &new_content)
        }
        EditTransactionType::FocusEdit => {
            let new_focus = match &planned.request {
                EditRequest::FocusEdit { new_focus, .. } => new_focus.clone(),
                _ => unreachable!(),
            };
            execute_truth_file_edit(&book_dir, &book_id, "current_focus.md", &new_focus)
                .map(|mut r| {
                    r.transaction_type = EditTransactionType::FocusEdit.as_str().to_string();
                    r
                })
        }
    }
}

// ── 实体改名 ─────────────────────────────────────────────────

/// 拒绝路径分隔符：实体名嵌入文件名时必须是单一路径组件。
fn assert_entity_rename_target_safe(new_name: &str) -> Result<(), AppError> {
    if new_name.contains('/') || new_name.contains('\\') {
        return Err(AppError::invalid_input(format!(
            "非法的实体名「{}」: 不能包含路径分隔符",
            new_name
        )));
    }
    if new_name.trim().is_empty() {
        return Err(AppError::invalid_input("实体名不能为空"));
    }
    Ok(())
}

fn execute_entity_rename(
    book_dir: &Path,
    book_id: &str,
    old_name: &str,
    new_name: &str,
) -> Result<ExecutedEditTransaction, AppError> {
    assert_entity_rename_target_safe(new_name)?;
    if old_name.trim().is_empty() {
        return Err(AppError::invalid_input("old_name 不能为空"));
    }

    let files = collect_editable_files(book_dir)?;
    let mut touched: Vec<String> = Vec::new();

    for file_path in &files {
        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let next_content = content.replace(old_name, new_name);
        if next_content == content {
            continue;
        }
        std::fs::write(file_path, next_content.as_bytes())?;
        let rel = relative_to(book_dir, file_path);
        touched.push(rel);
    }

    if touched.is_empty() {
        return Err(AppError::not_found(format!(
            "在书籍 {} 中未找到「{}」的出现",
            book_id, old_name
        )));
    }

    Ok(ExecutedEditTransaction {
        transaction_type: EditTransactionType::EntityRename.as_str().to_string(),
        book_id: book_id.to_string(),
        chapter_number: None,
        touched_files: touched,
        review_required: false,
        summary: format!("已将「{}」改为「{}」", old_name, new_name),
    })
}

// ── 章节整章替换 ─────────────────────────────────────────────

fn execute_chapter_replace(
    book_dir: &Path,
    book_id: &str,
    chapter_number: u32,
    new_content: &str,
) -> Result<ExecutedEditTransaction, AppError> {
    let trimmed = new_content.trim();
    if trimmed.is_empty() {
        return Err(AppError::invalid_input("整章替换需要 new_content 非空"));
    }

    let chapter_path = find_chapter_path(book_dir, chapter_number)?;
    let content_with_newline = if new_content.ends_with('\n') {
        new_content.to_string()
    } else {
        format!("{}\n", new_content)
    };
    std::fs::write(&chapter_path, content_with_newline.as_bytes())?;

    // 清理章节运行时缓存文件（story/runtime/chapter-XXXX.*）
    let removed_runtime = clear_chapter_runtime_files(book_dir, chapter_number)?;

    let mut touched = vec![relative_to(book_dir, &chapter_path)];
    touched.extend(removed_runtime);

    Ok(ExecutedEditTransaction {
        transaction_type: EditTransactionType::ChapterReplace.as_str().to_string(),
        book_id: book_id.to_string(),
        chapter_number: Some(chapter_number),
        touched_files: touched,
        review_required: true,
        summary: format!("已替换书籍 {} 第 {} 章正文，需人工 review", book_id, chapter_number),
    })
}

// ── 章节局部编辑 ─────────────────────────────────────────────

fn execute_chapter_local_edit(
    book_dir: &Path,
    book_id: &str,
    chapter_number: u32,
    find: &str,
    replace: &str,
) -> Result<ExecutedEditTransaction, AppError> {
    if find.is_empty() {
        return Err(AppError::invalid_input("find 不能为空"));
    }

    let chapter_path = find_chapter_path(book_dir, chapter_number)?;
    let content = std::fs::read_to_string(&chapter_path)?;
    let next_content = replace_chapter_target_text(&content, find, replace)?;

    if next_content == content {
        return Err(AppError::not_found(format!(
            "在第 {} 章中未找到目标文本",
            chapter_number
        )));
    }
    std::fs::write(&chapter_path, next_content.as_bytes())?;

    let removed_runtime = clear_chapter_runtime_files(book_dir, chapter_number)?;
    let mut touched = vec![relative_to(book_dir, &chapter_path)];
    touched.extend(removed_runtime);

    Ok(ExecutedEditTransaction {
        transaction_type: EditTransactionType::ChapterLocalEdit.as_str().to_string(),
        book_id: book_id.to_string(),
        chapter_number: Some(chapter_number),
        touched_files: touched,
        review_required: true,
        summary: format!("已修补书籍 {} 第 {} 章正文，需人工 review", book_id, chapter_number),
    })
}

// ── 真相文件编辑 ─────────────────────────────────────────────

fn execute_truth_file_edit(
    book_dir: &Path,
    book_id: &str,
    normalized_file_name: &str,
    new_content: &str,
) -> Result<ExecutedEditTransaction, AppError> {
    let story_dir = book_dir.join("story");
    std::fs::create_dir_all(&story_dir)?;
    let file_path = story_dir.join(normalized_file_name);
    std::fs::write(&file_path, new_content.as_bytes())?;

    Ok(ExecutedEditTransaction {
        transaction_type: EditTransactionType::TruthFileEdit.as_str().to_string(),
        book_id: book_id.to_string(),
        chapter_number: None,
        touched_files: vec![relative_to(book_dir, &file_path)],
        review_required: false,
        summary: format!("已更新 {}", normalized_file_name),
    })
}

// ── 三级文本匹配（精确 → 弹性空格 → 段落近似） ───────────────

/// 在 content 中查找 find 并替换为 replace。
/// 三级匹配策略：
/// 1. 精确字面匹配（split + join）
/// 2. 弹性空格匹配（把 find 拆词，允许任意空白连接）
/// 3. 段落近似匹配（Dice 二元组相似度，高阈值 + 显著领先判定）
///
/// 若三级都未命中，返回原 content（调用方根据返回值是否变化判断成功）。
fn replace_chapter_target_text(content: &str, find: &str, replace: &str) -> Result<String, AppError> {
    // 1. 精确匹配
    if content.contains(find) {
        return Ok(content.replace(find, replace));
    }

    // 2. 弹性空格匹配
    if let Some(replaced) = try_flexible_whitespace_replace(content, find, replace) {
        return Ok(replaced);
    }

    // 3. 段落近似匹配
    Ok(replace_approximate_paragraph(content, find, replace))
}

/// 弹性空格匹配：把 find 拆分为词，允许词之间任意空白（含换行）。
fn try_flexible_whitespace_replace(content: &str, find: &str, replace: &str) -> Option<String> {
    let parts: Vec<&str> = find.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    // 构造正则：parts 之间用 \s+ 连接
    let pattern_str = parts
        .iter()
        .map(|p| regex_escape(p))
        .collect::<Vec<_>>()
        .join(r"\s+");
    let re = match regex::Regex::new(&pattern_str) {
        Ok(r) => r,
        Err(_) => return None,
    };
    if re.is_match(content) {
        Some(re.replace_all(content, replace).to_string())
    } else {
        None
    }
}

/// 段落近似匹配：在段落粒度查找与 find 最相似的段落，若相似度足够高则替换。
fn replace_approximate_paragraph(content: &str, find: &str, replace: &str) -> String {
    let target = normalize_approximate_text(find);
    if target.chars().count() < 24 {
        return content.to_string();
    }

    let target_bigrams = to_bigrams(&target);
    if target_bigrams.is_empty() {
        return content.to_string();
    }

    let mut best: Option<(usize, usize, f64)> = None; // (start, end, score)
    let mut second_best_score = 0.0_f64;

    for (start, end, raw) in iter_paragraphs(content) {
        let raw_trimmed = raw.trim();
        if raw_trimmed.is_empty() {
            continue;
        }
        let normalized = normalize_approximate_text(raw_trimmed);
        let normalized_len = normalized.chars().count();
        if normalized_len < 24 {
            continue;
        }
        let target_len = target.chars().count();
        if normalized_len < (target_len as f64 * 0.35) as usize
            || normalized_len > target_len * 3
        {
            continue;
        }
        let score = approximate_text_score(&target_bigrams, &target, &normalized);
        match best {
            None => best = Some((start, end, score)),
            Some((_, _, best_score)) => {
                if score > best_score {
                    second_best_score = best_score;
                    best = Some((start, end, score));
                } else if score > second_best_score {
                    second_best_score = score;
                }
            }
        }
    }

    let (start, end, score) = match best {
        Some(b) => b,
        None => return content.to_string(),
    };

    // 高阈值 + 显著领先：保留为定位器，不做语义改写
    if score < 0.72 || (score < 0.86 && score - second_best_score < 0.06) {
        return content.to_string();
    }

    let mut result = String::with_capacity(content.len() + replace.len());
    result.push_str(&content[..start]);
    result.push_str(replace);
    result.push_str(&content[end..]);
    result
}

/// 迭代段落：返回 (start_byte, end_byte, raw_text)
/// 段落以空行（连续两个换行）分隔；EOF 也作为段落终止。
fn iter_paragraphs(content: &str) -> impl Iterator<Item = (usize, usize, String)> + '_ {
    let mut pos = 0;
    std::iter::from_fn(move || {
        // 跳过连续的空行（寻找下一段开始）
        while pos < content.len() {
            let rest = &content[pos..];
            // 当前位置是否是空行起始（\n\n 或开头即空白）
            if rest.starts_with("\n\n") {
                pos += 2;
                continue;
            }
            // 跳过开头孤立换行
            if rest.starts_with('\n') {
                pos += 1;
                continue;
            }
            break;
        }
        if pos >= content.len() {
            return None;
        }
        let start = pos;
        let rest = &content[start..];
        // 寻找下一个空行（\n\n）作为段落结束
        let end_offset = rest.find("\n\n").unwrap_or(rest.len());
        let end = start + end_offset;
        let raw = content[start..end].to_string();
        pos = if end_offset < rest.len() {
            end + 2 // skip "\n\n"
        } else {
            end
        };
        Some((start, end, raw))
    })
}

/// 归一化文本：NFKC + lowercase + 去除所有非字母数字字符
fn normalize_approximate_text(text: &str) -> String {
    // NFKC 等价：Rust 标准库无直接 unicode normalization，简化为 lowercase + 移除非字母数字
    // 对于中文场景，NFKC 影响较小（中文字符无 NFKC 变体）
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// 计算近似文本分数：包含关系优先，否则用 Dice 系数
fn approximate_text_score(
    target_bigrams: &[String],
    target: &str,
    candidate: &str,
) -> f64 {
    if candidate.contains(target) || target.contains(candidate) {
        let min_len = target.chars().count().min(candidate.chars().count());
        let max_len = target.chars().count().max(candidate.chars().count());
        if max_len == 0 {
            return 0.0;
        }
        return min_len as f64 / max_len as f64;
    }
    let candidate_bigrams = to_bigrams(candidate);
    dice_coefficient_bigrams(target_bigrams, &candidate_bigrams)
}

/// 计算二元组 Dice 相似度
pub fn dice_coefficient(a: &str, b: &str) -> f64 {
    let a_bigrams = to_bigrams(a);
    let b_bigrams = to_bigrams(b);
    dice_coefficient_bigrams(&a_bigrams, &b_bigrams)
}

fn dice_coefficient_bigrams(left: &[String], right: &[String]) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    use std::collections::HashMap;
    let mut counts: HashMap<&str, u32> = HashMap::new();
    for item in left {
        *counts.entry(item.as_str()).or_insert(0) += 1;
    }
    let mut overlap = 0_u32;
    for item in right {
        let count = counts.get(item.as_str()).copied().unwrap_or(0);
        if count == 0 {
            continue;
        }
        overlap += 1;
        counts.insert(item.as_str(), count - 1);
    }
    (2.0 * overlap as f64) / (left.len() + right.len()) as f64
}

/// 生成字符串的二元组（bigrams）
fn to_bigrams(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < 2 {
        return if chars.is_empty() { Vec::new() } else { vec![chars.iter().collect()] };
    }
    (0..chars.len() - 1)
        .map(|i| chars[i..i + 2].iter().collect())
        .collect()
}

// ── 工具函数 ─────────────────────────────────────────────────

/// 正则元字符转义
fn regex_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    for c in text.chars() {
        if matches!(c, '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' | '/') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// 收集 book_dir 下所有可编辑文件（递归，跳过 snapshots 目录）。
fn collect_editable_files(book_dir: &Path) -> Result<Vec<PathBuf>, AppError> {
    let mut out = Vec::new();
    if !book_dir.exists() {
        return Ok(out);
    }
    collect_editable_files_inner(book_dir, &mut out)?;
    Ok(out)
}

fn collect_editable_files_inner(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), AppError> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            // 跳过 snapshots 目录
            if path.file_name().and_then(|n| n.to_str()) == Some("snapshots") {
                continue;
            }
            // 跳过 hidden 临时目录（.tmp-*）
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with(".tmp"))
                .unwrap_or(false)
            {
                continue;
            }
            collect_editable_files_inner(&path, out)?;
        } else if file_type.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_lowercase();
                if matches!(ext_lower.as_str(), "md" | "json" | "yaml" | "yml" | "txt") {
                    out.push(path);
                }
            }
        }
    }
    Ok(())
}

/// 查找章节文件路径（chapters/XXXX_*.md）
fn find_chapter_path(book_dir: &Path, chapter_number: u32) -> Result<PathBuf, AppError> {
    let chapters_dir = book_dir.join("chapters");
    let padded = format!("{:04}", chapter_number);
    if !chapters_dir.exists() {
        return Err(AppError::directory_not_found(format!(
            "chapters dir of book"
        )));
    }
    for entry in std::fs::read_dir(&chapters_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(&padded) && name.ends_with(".md") {
            return Ok(entry.path());
        }
    }
    Err(AppError::file_not_found(format!(
        "chapter {} file (looking for {}_*.md)",
        chapter_number, padded
    )))
}

/// 清理章节运行时缓存文件（story/runtime/chapter-XXXX.*）
fn clear_chapter_runtime_files(book_dir: &Path, chapter_number: u32) -> Result<Vec<String>, AppError> {
    let padded = format!("{:04}", chapter_number);
    let runtime_dir = book_dir.join("story").join("runtime");
    let mut removed = Vec::new();
    if !runtime_dir.exists() {
        return Ok(removed);
    }
    let prefix = format!("chapter-{}.", padded);
    for entry in std::fs::read_dir(&runtime_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(&prefix) {
            let _ = std::fs::remove_file(entry.path());
            removed.push(relative_to(book_dir, &entry.path()));
        }
    }
    Ok(removed)
}

/// 计算 path 相对于 root 的相对路径字符串（失败时回退为绝对路径）
fn relative_to(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dice_basic() {
        // hello 与 hallo 共享 "ll" bigram
        let score = dice_coefficient("hello", "hallo");
        assert!(score > 0.0 && score <= 1.0);
        // 完全相同 → 1.0
        assert_eq!(dice_coefficient("abc", "abc"), 1.0);
        // 完全不同 → 0.0
        assert_eq!(dice_coefficient("abc", "xyz"), 0.0);
    }

    #[test]
    fn replace_exact_match() {
        let content = "hello world foo bar";
        let result = replace_chapter_target_text(content, "world", "rust").unwrap();
        assert_eq!(result, "hello rust foo bar");
    }

    #[test]
    fn replace_flexible_whitespace() {
        let content = "hello    world\n\nfoo";
        let result = replace_chapter_target_text(content, "hello world", "hi rust").unwrap();
        assert_eq!(result, "hi rust\n\nfoo");
    }

    #[test]
    fn replace_no_match_returns_original() {
        let content = "hello world";
        let result = replace_chapter_target_text(content, "xxxxx", "yyyyy").unwrap();
        assert_eq!(result, content);
    }

    #[test]
    fn normalize_removes_punctuation() {
        assert_eq!(normalize_approximate_text("Hello, World!"), "helloworld");
    }

    #[test]
    fn regex_escape_handles_metachars() {
        assert_eq!(regex_escape("a.b*c"), r"a\.b\*c");
        assert_eq!(regex_escape("normal"), "normal");
    }

    #[test]
    fn to_bigrams_basic() {
        let bg = to_bigrams("abc");
        assert_eq!(bg, vec!["ab".to_string(), "bc".to_string()]);
        let empty = to_bigrams("");
        assert!(empty.is_empty());
        let single = to_bigrams("a");
        assert_eq!(single, vec!["a".to_string()]);
    }
}
