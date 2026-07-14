// 交互运行时核心（Interaction Runtime core）。
//
// 核心函数：run_interaction_request
// 流程：
// 1. 根据 intent 构建初始 ExecutionState（build_task_started_state）
// 2. 追加 task.started 事件
// 3. 根据 intent 分发到对应处理函数：
//    - write_next / continue_book → pipeline_write_next_chapter
//    - revise_chapter / rewrite_chapter → pipeline_revise_draft
//    - patch_chapter_text / replace_chapter_text → edit_controller
//    - rename_entity → edit_controller (EntityRename)
//    - edit_truth → edit_controller (TruthFileEdit)
//    - update_focus → edit_controller (FocusEdit)
//    - update_author_intent → edit_controller (TruthFileEdit: author_intent.md)
//    - list_books → 扫描 books_dir
//    - select_book → 更新 session.active_book_id
//    - pause_book / resume_book → 更新 ExecutionState
//    - chat → 简化处理（不调用 AgentEngine 流式接口，仅返回静态响应）
//    - explain_status / explain_failure → 读取 currentExecution 输出
//    - export_book / develop_book / create_book / show_book_draft / discard_book_draft → 未实现，返回错误
// 4. 根据 automation_mode 决定是否等待用户（should_wait_for_human）
// 5. 追加 task.completed 事件
// 6. 返回 InteractionRuntimeResult

use std::path::PathBuf;

use crate::core::agent::engine::AgentEngine;
use crate::domain::interaction::edit_controller::{
    self, EditRequest, ExecutedEditTransaction, PlannedEditTransaction,
};
use crate::domain::interaction::types::{
    AutomationMode, ExecutionState, ExecutionStatus, InteractionEvent, InteractionIntent,
    InteractionRequest, InteractionRuntimeResult, InteractionSession, PendingDecision,
};
use crate::domain::pipeline::agents::reviser::ReviseMode;
use crate::domain::pipeline::commands::BookSummary;
use crate::domain::pipeline::runner::{PipelineConfig, PipelineRunner};
use crate::infrastructure::fs::data_dir::DataDir;
use crate::infrastructure::validation::validate_id;
use crate::shared::error::AppError;

// ── 核心运行时函数 ───────────────────────────────────────────

/// 运行交互请求。
///
/// 入参：
/// - request: 用户请求（intent + input + book_id + ...）
/// - session: 当前会话状态（运行时会被修改）
/// - data_dir: 应用数据目录（用于 books_dir 路径与 edit_controller）
/// - agent_engine: Agent 引擎（用于驱动 pipeline LLM 调用）
///
/// 返回：InteractionRuntimeResult（含最终 session + 响应文本 + 新事件）
///
/// 注意：session 在原处被修改，同时其最终快照被克隆进返回结果。
pub async fn run_interaction_request(
    request: InteractionRequest,
    session: &mut InteractionSession,
    data_dir: &DataDir,
    agent_engine: &AgentEngine,
) -> Result<InteractionRuntimeResult, AppError> {
    // 记录起始事件数，用于在结束时提取本次新增事件
    let start_event_count = session.events.len();

    // 1. 应用 mode override
    if let Some(mode) = request.mode {
        session.update_automation_mode(mode);
    }

    // 2. 清空 pending_decision，写入初始 ExecutionState
    session.clear_pending_decision();
    let started_state = build_task_started_state(&request, session);
    session.current_execution = Some(started_state.clone());
    session.append_event(InteractionEvent::new(
        "task.started",
        serde_json::json!({
            "intent": request.intent.as_str(),
            "status": started_state.status.as_str(),
            "stageLabel": started_state.stage_label,
        }),
    ));

    // 3. 分发到对应处理函数
    let dispatch_result = dispatch_intent(&request, session, data_dir, agent_engine).await;

    let response_text = match dispatch_result {
        Ok(text) => text,
        Err(e) => {
            // 失败：写入 task.failed 事件
            session.current_execution = Some(ExecutionState::new(
                ExecutionStatus::Failed,
                format!("失败: {}", e.message),
            ));
            session.append_event(InteractionEvent::new(
                "task.failed",
                serde_json::json!({
                    "intent": request.intent.as_str(),
                    "error": e.message,
                }),
            ));
            return Err(e);
        }
    };

    // 4. 根据 automation_mode 决定是否等待用户
    if should_wait_for_human(session.automation_mode, request.intent) {
        let book_id = request.book_id.as_deref().or(session.active_book_id.as_deref());
        if let Some(_bid) = book_id {
            let pending = PendingDecision {
                decision_type: "review-next-step".to_string(),
                description: if session.automation_mode == AutomationMode::Manual {
                    "执行已完成。请明确选择下一步操作。".to_string()
                } else {
                    "执行已完成，等待你的下一步决定。".to_string()
                },
                options: vec![
                    "继续写下一章".to_string(),
                    "修订本章".to_string(),
                    "查看状态".to_string(),
                ],
            };
            session.pending_decision = Some(pending);
            session.current_execution = Some(ExecutionState::new(
                ExecutionStatus::WaitingHuman,
                "等待你的下一步决定",
            ));
        } else {
            mark_completed(session);
        }
    } else {
        mark_completed(session);
    }

    // 5. 追加 task.completed 事件
    let final_status = session
        .current_execution
        .as_ref()
        .map(|e| e.status)
        .unwrap_or(ExecutionStatus::Completed);
    session.append_event(InteractionEvent::new(
        "task.completed",
        serde_json::json!({
            "intent": request.intent.as_str(),
            "status": final_status.as_str(),
            "responseText": response_text,
        }),
    ));

    // 6. 提取本次新增事件（已并入 session.events）
    let new_events: Vec<InteractionEvent> = session
        .events
        .iter()
        .skip(start_event_count)
        .cloned()
        .collect();

    Ok(InteractionRuntimeResult {
        session: session.clone(),
        response_text,
        events: new_events,
    })
}

/// 内部：分发 intent 到对应处理函数。返回响应文本。
async fn dispatch_intent(
    request: &InteractionRequest,
    session: &mut InteractionSession,
    data_dir: &DataDir,
    agent_engine: &AgentEngine,
) -> Result<String, AppError> {
    let intent = request.intent;

    match intent {
        // ── 列表 / 切换 ──
        InteractionIntent::ListBooks => {
            let books = list_books_inner(&data_dir.books_dir())?;
            let response = if books.is_empty() {
                "当前项目下没有作品。".to_string()
            } else {
                let names: Vec<String> = books
                    .iter()
                    .map(|b| format!("「{}」({})", b.title, b.id))
                    .collect();
                format!("作品列表（{} 本）：{}", books.len(), names.join("、"))
            };
            Ok(response)
        }
        InteractionIntent::SelectBook => {
            let book_id = request
                .book_id
                .as_deref()
                .ok_or_else(|| AppError::invalid_input("select_book 需要提供 book_id"))?;
            validate_id(book_id, "book_id").map_err(AppError::invalid_input)?;
            let books = list_books_inner(&data_dir.books_dir())?;
            if !books.iter().any(|b| b.id == book_id) {
                return Err(AppError::not_found(format!(
                    "当前项目中找不到作品「{}」",
                    book_id
                )));
            }
            session.bind_active_book(book_id.to_string());
            Ok(format!("当前作品：{}", book_id))
        }

        // ── 写下一章 / 继续写 ──
        InteractionIntent::WriteNext | InteractionIntent::ContinueBook => {
            let book_id = resolve_book_id(request, session)?;
            let runner = build_runner(data_dir);
            let result = runner
                .write_next_chapter(agent_engine, &book_id, None)
                .await?;
            session.bind_active_book(book_id.clone());
            Ok(format!(
                "已为 {} 完成第 {} 章写作。",
                book_id, result.chapter_number
            ))
        }

        // ── 修订 / 重写章节 ──
        InteractionIntent::ReviseChapter | InteractionIntent::RewriteChapter => {
            let book_id = resolve_book_id(request, session)?;
            let chapter_number = request.chapter_number.ok_or_else(|| {
                AppError::invalid_input("revise_chapter/rewrite_chapter 需要 chapter_number")
            })?;
            let mode = if intent == InteractionIntent::RewriteChapter {
                ReviseMode::Rewrite
            } else {
                ReviseMode::Auto
            };
            let runner = build_runner(data_dir);
            runner
                .revise_draft(agent_engine, &book_id, Some(chapter_number), mode)
                .await?;
            session.bind_active_book(book_id.clone());
            Ok(format!(
                "已为 {} 完成第 {} 章{}。",
                book_id,
                chapter_number,
                if intent == InteractionIntent::RewriteChapter {
                    "重写"
                } else {
                    "修订"
                }
            ))
        }

        // ── 章节局部修补 ──
        InteractionIntent::PatchChapterText => {
            let book_id = resolve_book_id(request, session)?;
            let chapter_number = request.chapter_number.ok_or_else(|| {
                AppError::invalid_input("patch_chapter_text 需要 chapter_number")
            })?;
            let find = request
                .params
                .get("targetText")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    if request.input.is_empty() {
                        None
                    } else {
                        Some(request.input.as_str())
                    }
                })
                .ok_or_else(|| {
                    AppError::invalid_input("patch_chapter_text 需要 targetText（或 input）")
                })?
                .to_string();
            let replace = request
                .params
                .get("replacementText")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    AppError::invalid_input("patch_chapter_text 需要 replacementText")
                })?
                .to_string();
            let req = EditRequest::ChapterLocalEdit {
                book_id: book_id.clone(),
                chapter_number,
                find,
                replace,
            };
            let executed = execute_edit_request(req, data_dir)?;
            session.bind_active_book(book_id.clone());
            Ok(format!(
                "已修补 {} 第 {} 章正文（涉及 {} 个文件，需 review）。",
                book_id,
                chapter_number,
                executed.touched_files.len()
            ))
        }

        // ── 整章替换 ──
        InteractionIntent::ReplaceChapterText => {
            let book_id = resolve_book_id(request, session)?;
            let chapter_number = request.chapter_number.ok_or_else(|| {
                AppError::invalid_input("replace_chapter_text 需要 chapter_number")
            })?;
            let new_content = request
                .params
                .get("fullText")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    if request.input.is_empty() {
                        None
                    } else {
                        Some(request.input.as_str())
                    }
                })
                .ok_or_else(|| {
                    AppError::invalid_input("replace_chapter_text 需要 fullText（或 input）")
                })?
                .to_string();
            let req = EditRequest::ChapterReplace {
                book_id: book_id.clone(),
                chapter_number,
                new_content,
            };
            let executed = execute_edit_request(req, data_dir)?;
            session.bind_active_book(book_id.clone());
            Ok(format!(
                "已替换 {} 第 {} 章正文（涉及 {} 个文件，需 review）。",
                book_id,
                chapter_number,
                executed.touched_files.len()
            ))
        }

        // ── 实体改名 ──
        InteractionIntent::RenameEntity => {
            let book_id = resolve_book_id(request, session)?;
            let old_name = request
                .params
                .get("oldValue")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::invalid_input("rename_entity 需要 oldValue"))?
                .to_string();
            let new_name = request
                .params
                .get("newValue")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::invalid_input("rename_entity 需要 newValue"))?
                .to_string();
            let req = EditRequest::EntityRename {
                book_id: book_id.clone(),
                old_name,
                new_name: new_name.clone(),
            };
            let executed = execute_edit_request(req, data_dir)?;
            session.bind_active_book(book_id.clone());
            Ok(format!(
                "已在 {} 中改名（涉及 {} 个文件）。",
                book_id,
                executed.touched_files.len()
            ))
        }

        // ── 编辑真相文件 ──
        InteractionIntent::EditTruth => {
            let book_id = resolve_book_id(request, session)?;
            let file_name = request
                .params
                .get("fileName")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::invalid_input("edit_truth 需要 fileName"))?
                .to_string();
            let new_content = request
                .params
                .get("content")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    if request.input.is_empty() {
                        None
                    } else {
                        Some(request.input.as_str())
                    }
                })
                .ok_or_else(|| AppError::invalid_input("edit_truth 需要 content（或 input）"))?
                .to_string();
            let req = EditRequest::TruthFileEdit {
                book_id: book_id.clone(),
                file_name: file_name.clone(),
                new_content,
            };
            execute_edit_request(req, data_dir)?;
            session.bind_active_book(book_id.clone());
            Ok(format!("已更新 {} 的 {}。", book_id, file_name))
        }

        // ── 更新焦点 ──
        InteractionIntent::UpdateFocus => {
            let book_id = resolve_book_id(request, session)?;
            let new_focus = if !request.input.is_empty() {
                request.input.clone()
            } else {
                request
                    .params
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::invalid_input("update_focus 需要 content（或 input）"))?
                    .to_string()
            };
            let req = EditRequest::FocusEdit {
                book_id: book_id.clone(),
                new_focus,
            };
            execute_edit_request(req, data_dir)?;
            session.bind_active_book(book_id.clone());
            Ok(format!("已更新 {} 的当前焦点。", book_id))
        }

        // ── 更新作者意图 ──
        InteractionIntent::UpdateAuthorIntent => {
            let book_id = resolve_book_id(request, session)?;
            let new_content = if !request.input.is_empty() {
                request.input.clone()
            } else {
                request
                    .params
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AppError::invalid_input("update_author_intent 需要 content（或 input）")
                    })?
                    .to_string()
            };
            let req = EditRequest::TruthFileEdit {
                book_id: book_id.clone(),
                file_name: "author_intent.md".to_string(),
                new_content,
            };
            execute_edit_request(req, data_dir)?;
            session.bind_active_book(book_id.clone());
            Ok(format!("已更新 {} 的作者意图。", book_id))
        }

        // ── 暂停 / 恢复 ──
        InteractionIntent::PauseBook => {
            let book_id = request
                .book_id
                .as_deref()
                .or(session.active_book_id.as_deref());
            session.current_execution = Some(ExecutionState::new(
                ExecutionStatus::Blocked,
                "已由用户暂停",
            ));
            Ok(format!("已暂停{}。", book_id.unwrap_or("当前作品")))
        }
        InteractionIntent::ResumeBook => {
            let book_id = request
                .book_id
                .as_deref()
                .or(session.active_book_id.as_deref());
            session.current_execution = Some(ExecutionState::new(
                ExecutionStatus::Completed,
                "可继续执行",
            ));
            Ok(format!("已恢复{}。", book_id.unwrap_or("当前作品")))
        }

        // ── 解释状态 / 失败 ──
        InteractionIntent::ExplainStatus | InteractionIntent::ExplainFailure => {
            let book_id = request
                .book_id
                .as_deref()
                .or(session.active_book_id.as_deref());
            let stage = session
                .current_execution
                .as_ref()
                .map(|e| e.stage_label.clone())
                .unwrap_or_else(|| "idle".to_string());
            let summary = if intent == InteractionIntent::ExplainFailure {
                format!(
                    "当前失败上下文：{} 处于 {}。",
                    book_id.unwrap_or("当前无激活作品"),
                    stage
                )
            } else {
                format!(
                    "当前状态：{} 处于 {}。",
                    book_id.unwrap_or("当前无激活作品"),
                    stage
                )
            };
            Ok(summary)
        }

        // ── 聊天（简化处理，不调 AgentEngine 流式接口） ──
        InteractionIntent::Chat => {
            let book_id = session.active_book_id.clone();
            let response = if let Some(bid) = book_id {
                format!(
                    "我在。当前作品是 {}。你可以让我继续写、修订章节、重写、调整焦点，或者查看流水线为何停止。",
                    bid
                )
            } else {
                "我在。当前还没有绑定作品。先打开作品、列出作品，或者直接描述你要写什么。".to_string()
            };
            Ok(response)
        }

        // ── 未实现的意图 ──
        InteractionIntent::DevelopBook
        | InteractionIntent::ShowBookDraft
        | InteractionIntent::CreateBook
        | InteractionIntent::DiscardBookDraft
        | InteractionIntent::ExportBook => Err(AppError::not_implemented(format!(
            "交互运行时暂未实现意图「{}」",
            intent.as_str()
        ))),
    }
}

/// 内部：解析 book_id（request.book_id 优先，回退到 session.active_book_id）
fn resolve_book_id(
    request: &InteractionRequest,
    session: &InteractionSession,
) -> Result<String, AppError> {
    let bid = request
        .book_id
        .as_deref()
        .or(session.active_book_id.as_deref())
        .ok_or_else(|| {
            AppError::invalid_input("当前交互会话还没有绑定作品（请先 select_book 或提供 book_id）")
        })?;
    validate_id(bid, "book_id").map_err(AppError::invalid_input)?;
    Ok(bid.to_string())
}

/// 内部：执行 EditRequest（plan + execute）
fn execute_edit_request(
    req: EditRequest,
    data_dir: &DataDir,
) -> Result<ExecutedEditTransaction, AppError> {
    let planned: PlannedEditTransaction = edit_controller::plan_edit_transaction(req)?;
    edit_controller::execute_edit_transaction(planned, data_dir)
}

/// 构造 PipelineRunner（从 DataDir 读取 books_dir）
fn build_runner(data_dir: &DataDir) -> PipelineRunner {
    let config = PipelineConfig {
        books_dir: data_dir.books_dir(),
        ..Default::default()
    };
    PipelineRunner::new(config)
}

/// 判断是否需要等待用户决策。
///
/// - Auto: 永不等待
/// - Semi: 内容生成类操作等待
/// - Manual: 内容生成 + 编辑类操作都等待
pub fn should_wait_for_human(mode: AutomationMode, intent: InteractionIntent) -> bool {
    let content_intent = matches!(
        intent,
        InteractionIntent::WriteNext
            | InteractionIntent::ContinueBook
            | InteractionIntent::ReviseChapter
            | InteractionIntent::RewriteChapter
            | InteractionIntent::PatchChapterText
            | InteractionIntent::ReplaceChapterText
    );
    let edit_intent = matches!(
        intent,
        InteractionIntent::UpdateFocus
            | InteractionIntent::UpdateAuthorIntent
            | InteractionIntent::EditTruth
            | InteractionIntent::RenameEntity
    );

    match mode {
        AutomationMode::Auto => false,
        AutomationMode::Semi => content_intent,
        AutomationMode::Manual => content_intent || edit_intent,
    }
}

/// 构建初始执行状态。
pub fn build_task_started_state(
    request: &InteractionRequest,
    _session: &InteractionSession,
) -> ExecutionState {
    match request.intent {
        InteractionIntent::WriteNext | InteractionIntent::ContinueBook => {
            ExecutionState::new(ExecutionStatus::Planning, "准备章节输入")
        }
        InteractionIntent::CreateBook => {
            ExecutionState::new(ExecutionStatus::Planning, "创建作品基础")
        }
        InteractionIntent::ExportBook => {
            ExecutionState::new(ExecutionStatus::Persisting, "导出作品文件")
        }
        InteractionIntent::ReviseChapter | InteractionIntent::RewriteChapter => {
            ExecutionState::new(
                ExecutionStatus::Repairing,
                if request.intent == InteractionIntent::RewriteChapter {
                    "重写章节"
                } else {
                    "修订章节"
                },
            )
        }
        InteractionIntent::PatchChapterText | InteractionIntent::ReplaceChapterText => {
            ExecutionState::new(ExecutionStatus::Repairing, "修补章节正文")
        }
        InteractionIntent::UpdateFocus
        | InteractionIntent::UpdateAuthorIntent
        | InteractionIntent::EditTruth
        | InteractionIntent::RenameEntity => {
            ExecutionState::new(ExecutionStatus::Persisting, "应用项目修改")
        }
        InteractionIntent::PauseBook | InteractionIntent::DiscardBookDraft => {
            ExecutionState::new(ExecutionStatus::Blocked, "已由用户暂停")
        }
        InteractionIntent::ResumeBook => {
            ExecutionState::new(ExecutionStatus::Completed, "可继续执行")
        }
        InteractionIntent::ListBooks | InteractionIntent::SelectBook => {
            ExecutionState::new(ExecutionStatus::Planning, "查询作品")
        }
        InteractionIntent::Chat
        | InteractionIntent::ExplainStatus
        | InteractionIntent::ExplainFailure => {
            ExecutionState::new(ExecutionStatus::Idle, "处理对话")
        }
        InteractionIntent::DevelopBook | InteractionIntent::ShowBookDraft => {
            ExecutionState::new(ExecutionStatus::Planning, "处理创作草案")
        }
    }
}

/// 内部：将 session 标记为 completed
fn mark_completed(session: &mut InteractionSession) {
    session.current_execution = Some(ExecutionState::new(
        ExecutionStatus::Completed,
        "已完成",
    ));
}

/// 内部：扫描 books_dir 列出所有书籍（与 pipeline_list_books 同源逻辑，简化版）
fn list_books_inner(books_dir: &PathBuf) -> Result<Vec<BookSummary>, AppError> {
    if !books_dir.exists() {
        return Ok(Vec::new());
    }

    let mut summaries = Vec::new();
    for entry in std::fs::read_dir(books_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let book_id = entry.file_name().to_string_lossy().to_string();
        if book_id.starts_with('.') || book_id.starts_with(".tmp") {
            continue;
        }

        let book_dir = entry.path();
        let config_path = book_dir.join("book.json");
        if !config_path.exists() {
            continue;
        }

        let config_content = match std::fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let book: crate::domain::pipeline::types::BookConfig = match serde_json::from_str(&config_content) {
            Ok(b) => b,
            Err(_) => continue,
        };

        let chapter_count = load_chapter_count(&book_dir);

        summaries.push(BookSummary {
            id: book.id.clone(),
            title: book.title,
            genre: book.genre,
            status: format!("{:?}", book.status).to_lowercase(),
            target_chapters: book.target_chapters,
            chapter_word_count: book.chapter_word_count,
            language: format!("{:?}", book.language.unwrap_or_default()).to_lowercase(),
            chapter_count,
            created_at: book.created_at,
            updated_at: book.updated_at,
        });
    }

    summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(summaries)
}

/// 读取书籍的章节数
fn load_chapter_count(book_dir: &std::path::Path) -> u32 {
    let index_path = book_dir.join("chapters.json");
    let content = match std::fs::read_to_string(&index_path) {
        Ok(c) => c,
        Err(_) => return 0,
    };
    let index: Vec<crate::domain::pipeline::types::ChapterMeta> = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return 0,
    };
    index.len() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::interaction::types::SessionKind;

    #[test]
    fn should_wait_auto_never_waits() {
        for intent in [
            InteractionIntent::WriteNext,
            InteractionIntent::ReviseChapter,
            InteractionIntent::EditTruth,
            InteractionIntent::RenameEntity,
        ] {
            assert!(!should_wait_for_human(AutomationMode::Auto, intent));
        }
    }

    #[test]
    fn should_wait_semi_waits_for_content_only() {
        assert!(should_wait_for_human(
            AutomationMode::Semi,
            InteractionIntent::WriteNext
        ));
        assert!(should_wait_for_human(
            AutomationMode::Semi,
            InteractionIntent::ReviseChapter
        ));
        assert!(!should_wait_for_human(
            AutomationMode::Semi,
            InteractionIntent::EditTruth
        ));
        assert!(!should_wait_for_human(
            AutomationMode::Semi,
            InteractionIntent::RenameEntity
        ));
        assert!(!should_wait_for_human(
            AutomationMode::Semi,
            InteractionIntent::ListBooks
        ));
    }

    #[test]
    fn should_wait_manual_waits_for_content_and_edit() {
        assert!(should_wait_for_human(
            AutomationMode::Manual,
            InteractionIntent::WriteNext
        ));
        assert!(should_wait_for_human(
            AutomationMode::Manual,
            InteractionIntent::EditTruth
        ));
        assert!(should_wait_for_human(
            AutomationMode::Manual,
            InteractionIntent::RenameEntity
        ));
        assert!(!should_wait_for_human(
            AutomationMode::Manual,
            InteractionIntent::ListBooks
        ));
    }

    #[test]
    fn build_task_started_state_matches_intent() {
        let session = InteractionSession::new("test", SessionKind::Book);
        let req = InteractionRequest {
            intent: InteractionIntent::WriteNext,
            input: String::new(),
            book_id: None,
            chapter_number: None,
            session_id: None,
            mode: None,
            params: serde_json::Value::Null,
        };
        let state = build_task_started_state(&req, &session);
        assert_eq!(state.status, ExecutionStatus::Planning);
        assert_eq!(state.stage_label, "准备章节输入");
    }

    #[test]
    fn resolve_book_id_returns_active_when_request_missing() {
        let mut session = InteractionSession::new("test", SessionKind::Book);
        session.bind_active_book("book-1");
        let req = InteractionRequest {
            intent: InteractionIntent::WriteNext,
            input: String::new(),
            book_id: None,
            chapter_number: None,
            session_id: None,
            mode: None,
            params: serde_json::Value::Null,
        };
        let bid = resolve_book_id(&req, &session).unwrap();
        assert_eq!(bid, "book-1");
    }

    #[test]
    fn resolve_book_id_fails_when_neither_provided() {
        let session = InteractionSession::new("test", SessionKind::Chat);
        let req = InteractionRequest {
            intent: InteractionIntent::WriteNext,
            input: String::new(),
            book_id: None,
            chapter_number: None,
            session_id: None,
            mode: None,
            params: serde_json::Value::Null,
        };
        assert!(resolve_book_id(&req, &session).is_err());
    }
}
