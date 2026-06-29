use std::sync::Arc;
use tokio::sync::{mpsc, watch, Mutex};
use tokio::task::JoinHandle;

use crate::shared::errors::AppError;
use crate::infrastructure::llm_client::Provider;
use crate::infrastructure::file_storage::data_dir::DataDir;
use crate::infrastructure::db::Database;
use crate::infrastructure::sandbox::enforce::SandboxEnforcer;
use crate::infrastructure::state_store::memory::MemoryStore;
use crate::infrastructure::state_store::feedback::FeedbackStore;

use super::op::{Op, Submission, SubmissionId};
use super::event::Event;

/// Configuration for a session
#[derive(Clone)]
pub struct SessionConfig {
    pub provider: Arc<dyn Provider>,
    pub model: String,
    pub project_root: std::path::PathBuf,
    pub data_dir: DataDir,
    pub db: Arc<tokio::sync::Mutex<Database>>,
    pub sandbox: Arc<tokio::sync::Mutex<SandboxEnforcer>>,
    pub memory_store: Arc<MemoryStore>,
    pub feedback_store: Arc<tokio::sync::Mutex<FeedbackStore>>,
    pub model_overrides: std::collections::HashMap<String, String>,
}

/// Status of a session
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SessionStatus {
    /// Session is starting up
    Starting,
    /// Session is idle, waiting for submissions
    Idle,
    /// Session is processing a submission
    Processing(SubmissionId),
    /// Session is waiting for tool approval
    WaitingApproval(SubmissionId, String),
    /// Session has been shut down
    Shutdown,
}

/// A pending tool approval request
#[derive(Debug, Clone)]
pub struct PendingApproval {
    pub submission_id: SubmissionId,
    pub tool_call_id: String,
    pub tool_name: String,
    pub args: String,
}

/// Response sender for tool approval
pub struct ApprovalResponse {
    pub response_tx: tokio::sync::oneshot::Sender<bool>,
}

/// The Session struct — core of the SQ/EQ pattern.
///
/// Owns:
/// - `tx_sub`: Submission queue (clients send Op values here)
/// - `rx_event`: Event queue (clients receive Event values from here)
/// - A background tokio task running the submission loop
///
/// The session runs the agent loop asynchronously, never blocking the caller.
pub struct Session {
    /// Session identifier
    pub id: String,

    /// Send submissions to the agent loop
    tx_sub: mpsc::Sender<Submission>,

    /// Receive events from the agent loop
    rx_event: mpsc::Receiver<Event>,

    /// Current session status (watched by UI)
    status: watch::Receiver<SessionStatus>,

    /// Pending tool approvals
    pub pending_approvals: Arc<Mutex<std::collections::HashMap<String, ApprovalResponse>>>,

    /// Handle to the background submission loop task
    _loop_handle: JoinHandle<()>,

    /// Cancel flag for the submission loop
    pub cancel_flag: Arc<tokio::sync::RwLock<bool>>,
}

impl Session {
    /// Create a new session and start the submission loop in the background.
    ///
    /// Returns the session handle and a receiver for events.
    pub fn spawn(config: SessionConfig, session_id: String) -> Self {
        let (tx_sub, rx_sub) = mpsc::channel::<Submission>(64);
        let (tx_event, rx_event) = mpsc::channel::<Event>(128);
        let (status_tx, status_rx) = watch::channel(SessionStatus::Starting);
        let cancel_flag = Arc::new(tokio::sync::RwLock::new(false));
        let pending_approvals: Arc<Mutex<std::collections::HashMap<String, ApprovalResponse>>> =
            Arc::new(Mutex::new(std::collections::HashMap::new()));

        let loop_cancel = cancel_flag.clone();
        let loop_approvals = pending_approvals.clone();
        let loop_session_id = session_id.clone();

        let handle = tokio::spawn(submission_loop(
            loop_session_id,
            config,
            rx_sub,
            tx_event,
            status_tx,
            loop_cancel,
            loop_approvals,
        ));

        Self {
            id: session_id,
            tx_sub,
            rx_event,
            status: status_rx,
            pending_approvals,
            _loop_handle: handle,
            cancel_flag,
        }
    }

    /// Submit an operation to the session (non-blocking).
    ///
    /// Returns the submission ID for tracking.
    pub async fn submit(&self, op: Op) -> Result<SubmissionId, AppError> {
        let submission = Submission::new(op);
        let id = submission.id.clone();
        self.tx_sub.send(submission).await
            .map_err(|_| AppError::internal("Session channel closed"))?;
        Ok(id)
    }

    /// Receive the next event from the session (blocking).
    pub async fn next_event(&mut self) -> Option<Event> {
        self.rx_event.recv().await
    }

    /// Get the current session status
    pub fn status(&self) -> SessionStatus {
        self.status.borrow().clone()
    }

    /// Watch for status changes
    pub fn watch_status(&self) -> watch::Receiver<SessionStatus> {
        self.status.clone()
    }

    /// Cancel the current operation
    pub async fn cancel(&self, reason: Option<String>) -> Result<(), AppError> {
        self.submit(Op::Interrupt(super::op::InterruptPayload { reason })).await?;
        Ok(())
    }

    /// Shut down the session gracefully
    pub async fn shutdown(&self) -> Result<(), AppError> {
        let _ = self.tx_sub.send(Submission::new(Op::Shutdown)).await;
        Ok(())
    }

    /// Approve a pending tool execution
    pub async fn approve_tool(&self, tool_call_id: &str) -> Result<(), AppError> {
        self.submit(Op::ApproveTool(super::op::ApproveToolPayload {
            tool_call_id: tool_call_id.to_string(),
        })).await?;
        Ok(())
    }

    /// Reject a pending tool execution
    pub async fn reject_tool(&self, tool_call_id: &str, reason: Option<String>) -> Result<(), AppError> {
        self.submit(Op::RejectTool(super::op::RejectToolPayload {
            tool_call_id: tool_call_id.to_string(),
            reason,
        })).await?;
        Ok(())
    }
}

/// The submission loop — runs as a background tokio task.
///
/// Processes submissions sequentially:
/// 1. Receive an Op from the submission queue
/// 2. Dispatch to the appropriate handler
/// 3. Emit events on the event queue
/// 4. Repeat
///
/// This is the core of the SQ/EQ pattern, modeled after Codex CLI's
/// `submission_loop` in `core/src/session/handlers.rs`.
async fn submission_loop(
    session_id: String,
    config: SessionConfig,
    mut rx_sub: mpsc::Receiver<Submission>,
    tx_event: mpsc::Sender<Event>,
    status_tx: watch::Sender<SessionStatus>,
    cancel_flag: Arc<tokio::sync::RwLock<bool>>,
    pending_approvals: Arc<Mutex<std::collections::HashMap<String, ApprovalResponse>>>,
) {
    tracing::info!(session_id = %session_id, "Submission loop started");

    // Signal that the session is configured and ready
    let _ = tx_event.send(Event::SessionConfigured(
        super::event::SessionConfiguredPayload {
            session_id: session_id.clone(),
            model: config.model.clone(),
        },
    )).await;

    let _ = status_tx.send(SessionStatus::Idle);

    while let Some(submission) = rx_sub.recv().await {
        let Submission { id, op } = submission;
        tracing::debug!(
            session_id = %session_id,
            submission_id = %id.0,
            op = ?op,
            "Processing submission"
        );

        match op {
            Op::Shutdown => {
                tracing::info!(session_id = %session_id, "Shutdown requested");
                let _ = tx_event.send(Event::SessionShutdown).await;
                let _ = status_tx.send(SessionStatus::Shutdown);
                break;
            }

            Op::Interrupt(payload) => {
                *cancel_flag.write().await = true;
                tracing::warn!(
                    session_id = %session_id,
                    reason = ?payload.reason,
                    "Interrupt received"
                );
            }

            Op::UserInput(payload) => {
                *cancel_flag.write().await = false;
                let _ = status_tx.send(SessionStatus::Processing(id.clone()));

                let result = handle_user_input(
                    &session_id,
                    &config,
                    &payload.session_id,
                    &payload.content,
                    &tx_event,
                    &cancel_flag,
                ).await;

                if let Err(e) = result {
                    let _ = tx_event.send(Event::TurnFailed(
                        super::event::TurnFailedPayload {
                            session_id: payload.session_id,
                            submission_id: id.0,
                            error: e.to_string(),
                        },
                    )).await;
                }

                let _ = status_tx.send(SessionStatus::Idle);
            }

            Op::WriteNextChapter(payload) => {
                *cancel_flag.write().await = false;
                let _ = status_tx.send(SessionStatus::Processing(id.clone()));

                let result = handle_write_next_chapter(
                    &session_id,
                    &config,
                    &payload,
                    &tx_event,
                    &cancel_flag,
                ).await;

                if let Err(e) = result {
                    let _ = tx_event.send(Event::Error(
                        super::event::ErrorPayload {
                            session_id: Some(session_id.clone()),
                            error: e.to_string(),
                            fatal: false,
                        },
                    )).await;
                }

                let _ = status_tx.send(SessionStatus::Idle);
            }

            Op::CreateBook(payload) => {
                *cancel_flag.write().await = false;
                let _ = status_tx.send(SessionStatus::Processing(id.clone()));

                let result = handle_create_book(
                    &session_id,
                    &config,
                    &payload,
                    &tx_event,
                ).await;

                if let Err(e) = result {
                    let _ = tx_event.send(Event::Error(
                        super::event::ErrorPayload {
                            session_id: Some(session_id.clone()),
                            error: e.to_string(),
                            fatal: false,
                        },
                    )).await;
                }

                let _ = status_tx.send(SessionStatus::Idle);
            }

            Op::ApproveTool(payload) => {
                if let Some(_approval) = pending_approvals.lock().await.remove(&payload.tool_call_id) {
                    let _ = tx_event.send(Event::ToolApprovalGranted(
                        super::event::ToolApprovalGrantedPayload {
                            session_id: session_id.clone(),
                            tool_call_id: payload.tool_call_id,
                        },
                    )).await;
                }
            }

            Op::RejectTool(payload) => {
                if let Some(_approval) = pending_approvals.lock().await.remove(&payload.tool_call_id) {
                    let _ = tx_event.send(Event::ToolApprovalRejected(
                        super::event::ToolApprovalRejectedPayload {
                            session_id: session_id.clone(),
                            tool_call_id: payload.tool_call_id,
                            reason: payload.reason,
                        },
                    )).await;
                }
            }

            Op::PlanChapter(payload) => {
                *cancel_flag.write().await = false;
                let _ = status_tx.send(SessionStatus::Processing(id.clone()));

                let result = handle_plan_chapter(
                    &session_id,
                    &config,
                    &payload,
                    &tx_event,
                    &cancel_flag,
                ).await;

                if let Err(e) = result {
                    let _ = tx_event.send(Event::Error(
                        super::event::ErrorPayload {
                            session_id: Some(session_id.clone()),
                            error: e.to_string(),
                            fatal: false,
                        },
                    )).await;
                }

                let _ = status_tx.send(SessionStatus::Idle);
            }

            Op::AuditChapter(payload) => {
                *cancel_flag.write().await = false;
                let _ = status_tx.send(SessionStatus::Processing(id.clone()));

                let result = handle_audit_chapter(
                    &session_id,
                    &config,
                    &payload,
                    &tx_event,
                    &cancel_flag,
                ).await;

                if let Err(e) = result {
                    let _ = tx_event.send(Event::Error(
                        super::event::ErrorPayload {
                            session_id: Some(session_id.clone()),
                            error: e.to_string(),
                            fatal: false,
                        },
                    )).await;
                }

                let _ = status_tx.send(SessionStatus::Idle);
            }

            Op::ReviseChapter(payload) => {
                *cancel_flag.write().await = false;
                let _ = status_tx.send(SessionStatus::Processing(id.clone()));

                let result = handle_revise_chapter(
                    &session_id,
                    &config,
                    &payload,
                    &tx_event,
                    &cancel_flag,
                ).await;

                if let Err(e) = result {
                    let _ = tx_event.send(Event::Error(
                        super::event::ErrorPayload {
                            session_id: Some(session_id.clone()),
                            error: e.to_string(),
                            fatal: false,
                        },
                    )).await;
                }

                let _ = status_tx.send(SessionStatus::Idle);
            }
        }
    }

    tracing::info!(session_id = %session_id, "Submission loop ended");
}

/// Handle a user input submission (interactive chat).
///
/// This mirrors the logic from `agent_send_message` but runs inside the
/// submission loop instead of blocking a Tauri command handler.
async fn handle_user_input(
    _session_id: &str,
    config: &SessionConfig,
    chat_session_id: &str,
    content: &str,
    tx_event: &mpsc::Sender<Event>,
    cancel_flag: &Arc<tokio::sync::RwLock<bool>>,
) -> Result<(), AppError> {
    use crate::infrastructure::llm_client::types::{Message, StreamEvent};
    use futures::StreamExt;

    let _ = tx_event.send(Event::TurnStarted(
        super::event::TurnStartedPayload {
            session_id: chat_session_id.to_string(),
            submission_id: String::new(),
        },
    )).await;

    // Save user message to DB
    {
        let db = config.db.lock().await;
        if let Err(e) = db.create_message(chat_session_id, "user", content, None, None).await {
            tracing::error!(error = %e, "Failed to save user message");
        }
    }

    // Build system prompt
    let system_prompt = {
        let feedback = config.feedback_store.lock().await;
        let lessons = feedback.format_lessons_for_prompt();
        let mut prompt = "你是 Mnemosyne，一个专业的 AI 创作助手。你帮助用户进行小说创作、角色设计、世界观构建、情节分析和趋势研究。请用中文回复。".to_string();
        if !lessons.is_empty() {
            prompt = format!("{}\n\n{}", prompt, lessons);
        }
        prompt
    };

    // Load history
    let all_messages: Vec<Message> = {
        let db = config.db.lock().await;
        let db_messages = db.list_messages(chat_session_id).await
            .map_err(|e| AppError::internal(format!("Failed to load messages: {}", e)))?;
        let start = db_messages.len().saturating_sub(50);
        db_messages[start..].iter().map(|m| {
            Message {
                role: m.role.clone(),
                content: m.content.clone(),
                tool_calls: None,
                tool_call_id: None,
            }
        }).collect()
    };

    // Stream LLM response
    let stream = config.provider.stream(&config.model, &system_prompt, &all_messages, &[]).await?;
    let mut text_buf = String::new();
    let mut total_input: u32 = 0;
    let mut total_output: u32 = 0;

    tokio::pin!(stream);
    while let Some(event) = stream.next().await {
        if *cancel_flag.read().await {
            tracing::warn!("Turn cancelled");
            break;
        }

        match event {
            StreamEvent::TextDelta { content } => {
                text_buf.push_str(&content);
                let _ = tx_event.send(Event::StreamDelta(
                    super::event::StreamDeltaPayload {
                        session_id: chat_session_id.to_string(),
                        content,
                    },
                )).await;
            }
            StreamEvent::Finish { usage, .. } => {
                total_input += usage.input_tokens;
                total_output += usage.output_tokens;
            }
            StreamEvent::Error(e) => {
                let _ = tx_event.send(Event::Error(
                    super::event::ErrorPayload {
                        session_id: Some(chat_session_id.to_string()),
                        error: e,
                        fatal: true,
                    },
                )).await;
                return Err(AppError::internal("Stream error"));
            }
            _ => {}
        }
    }

    // Save final response
    if !text_buf.is_empty() {
        let db = config.db.lock().await;
        let _ = db.create_message(chat_session_id, "assistant", &text_buf, None, None).await;
    }

    // Update token counts
    {
        let db = config.db.lock().await;
        if let Ok(Some(mut session)) = db.get_session(chat_session_id).await {
            session.input_tokens += total_input;
            session.output_tokens += total_output;
            let _ = db.update_session(&session).await;
        }
    }

    let _ = tx_event.send(Event::TurnCompleted(
        super::event::TurnCompletedPayload {
            session_id: chat_session_id.to_string(),
            submission_id: String::new(),
            input_tokens: total_input,
            output_tokens: total_output,
        },
    )).await;

    Ok(())
}

/// Handle a write next chapter submission.
async fn handle_write_next_chapter(
    _session_id: &str,
    config: &SessionConfig,
    payload: &super::op::WriteNextChapterPayload,
    tx_event: &mpsc::Sender<Event>,
    cancel_flag: &Arc<tokio::sync::RwLock<bool>>,
) -> Result<(), AppError> {
    use crate::core::agent::pipeline::{PipelineConfig, PipelineRunner};

    let workspace_path = {
        let db = config.db.lock().await;
        let ws = db.get_workspace(&payload.workspace_id).await?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        std::path::PathBuf::from(ws.path)
    };

    let runner = {
        let pipeline_config = PipelineConfig {
            provider: config.provider.clone(),
            model: config.model.clone(),
            project_root: workspace_path,
            model_overrides: config.model_overrides.clone(),
            memory_store: Some(config.memory_store.clone()),
            data_dir: config.data_dir.clone(),
            user_profile: None,
        };
        PipelineRunner::new(pipeline_config)
    };

    // Emit stage events as the pipeline progresses
    let stages = ["plan", "compose", "write", "audit", "revise"];
    for stage in &stages {
        if *cancel_flag.read().await {
            return Err(AppError::internal("Pipeline cancelled"));
        }

        let _ = tx_event.send(Event::PipelineStageStarted(
            super::event::PipelineStageStartedPayload {
                book_id: payload.book_id.clone(),
                chapter_number: 0, // Will be determined by pipeline
                stage: stage.to_string(),
                label: format!("Running {} stage", stage),
            },
        )).await;
    }

    let start = std::time::Instant::now();
    let result = runner.write_next_chapter(&payload.book_id, payload.target_words).await?;
    let elapsed = start.elapsed().as_secs();

    let _ = tx_event.send(Event::PipelineCompleted(
        super::event::PipelineCompletedPayload {
            book_id: payload.book_id.clone(),
            chapter_number: result.chapter_number,
            word_count: result.word_count,
            audit_passed: result.audit.passed,
            elapsed_secs: elapsed,
        },
    )).await;

    Ok(())
}

/// Handle a book creation submission.
async fn handle_create_book(
    _session_id: &str,
    config: &SessionConfig,
    payload: &super::op::CreateBookPayload,
    tx_event: &mpsc::Sender<Event>,
) -> Result<(), AppError> {
    use crate::core::agent::pipeline::{PipelineConfig, PipelineRunner};

    let workspace_path = {
        let db = config.db.lock().await;
        let ws = db.get_workspace(&payload.workspace_id).await?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        std::path::PathBuf::from(ws.path)
    };

    let runner = {
        let pipeline_config = PipelineConfig {
            provider: config.provider.clone(),
            model: config.model.clone(),
            project_root: workspace_path,
            model_overrides: config.model_overrides.clone(),
            memory_store: Some(config.memory_store.clone()),
            data_dir: config.data_dir.clone(),
            user_profile: None,
        };
        PipelineRunner::new(pipeline_config)
    };

    let book = runner.create_book(&payload.title, &payload.genre, payload.brief.as_deref()).await?;

    // Save to DB
    {
        let db = config.db.lock().await;
        db.insert_novel(&book.id, &crate::infrastructure::db::models::CreateNovelRequest {
            workspace_id: payload.workspace_id.clone(),
            title: payload.title.clone(),
            genre: payload.genre.clone(),
            platform: "local".to_string(),
            language: "zh".to_string(),
            target_chapters: 100,
            chapter_words: 3000,
        }).await?;
    }

    let _ = tx_event.send(Event::Progress(
        super::event::ProgressPayload {
            message: format!("Book '{}' created successfully", payload.title),
            percent: Some(100.0),
        },
    )).await;

    Ok(())
}

/// Handle a plan chapter submission.
async fn handle_plan_chapter(
    _session_id: &str,
    config: &SessionConfig,
    payload: &super::op::PlanChapterPayload,
    tx_event: &mpsc::Sender<Event>,
    _cancel_flag: &Arc<tokio::sync::RwLock<bool>>,
) -> Result<(), AppError> {
    use crate::core::agent::pipeline::{PipelineConfig, PipelineRunner};

    let workspace_path = {
        let db = config.db.lock().await;
        let ws = db.get_workspace(&payload.workspace_id).await?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        std::path::PathBuf::from(ws.path)
    };

    let runner = {
        let pipeline_config = PipelineConfig {
            provider: config.provider.clone(),
            model: config.model.clone(),
            project_root: workspace_path,
            model_overrides: config.model_overrides.clone(),
            memory_store: Some(config.memory_store.clone()),
            data_dir: config.data_dir.clone(),
            user_profile: None,
        };
        PipelineRunner::new(pipeline_config)
    };

    let _ = tx_event.send(Event::PipelineStageStarted(
        super::event::PipelineStageStartedPayload {
            book_id: payload.book_id.clone(),
            chapter_number: 0,
            stage: "plan".to_string(),
            label: "Planning chapter".to_string(),
        },
    )).await;

    let _plan = runner.plan_chapter(&payload.book_id, payload.context.as_deref()).await?;

    let _ = tx_event.send(Event::PipelineStageCompleted(
        super::event::PipelineStageCompletedPayload {
            book_id: payload.book_id.clone(),
            chapter_number: 0,
            stage: "plan".to_string(),
            label: "Chapter planned".to_string(),
        },
    )).await;

    let _ = tx_event.send(Event::Progress(
        super::event::ProgressPayload {
            message: format!("Chapter planned for book {}", payload.book_id),
            percent: Some(100.0),
        },
    )).await;

    Ok(())
}

/// Handle an audit chapter submission.
async fn handle_audit_chapter(
    _session_id: &str,
    config: &SessionConfig,
    payload: &super::op::AuditChapterPayload,
    tx_event: &mpsc::Sender<Event>,
    _cancel_flag: &Arc<tokio::sync::RwLock<bool>>,
) -> Result<(), AppError> {
    use crate::core::agent::pipeline::{PipelineConfig, PipelineRunner};

    let workspace_path = {
        let db = config.db.lock().await;
        let ws = db.get_workspace(&payload.workspace_id).await?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        std::path::PathBuf::from(ws.path)
    };

    let runner = {
        let pipeline_config = PipelineConfig {
            provider: config.provider.clone(),
            model: config.model.clone(),
            project_root: workspace_path,
            model_overrides: config.model_overrides.clone(),
            memory_store: Some(config.memory_store.clone()),
            data_dir: config.data_dir.clone(),
            user_profile: None,
        };
        PipelineRunner::new(pipeline_config)
    };

    let _ = tx_event.send(Event::PipelineStageStarted(
        super::event::PipelineStageStartedPayload {
            book_id: payload.book_id.clone(),
            chapter_number: payload.chapter_number,
            stage: "audit".to_string(),
            label: "Auditing chapter".to_string(),
        },
    )).await;

    let result = runner.audit_chapter(&payload.book_id, payload.chapter_number).await?;

    let _ = tx_event.send(Event::PipelineStageCompleted(
        super::event::PipelineStageCompletedPayload {
            book_id: payload.book_id.clone(),
            chapter_number: payload.chapter_number,
            stage: "audit".to_string(),
            label: format!("Audit completed: {}", if result.passed { "PASSED" } else { "FAILED" }),
        },
    )).await;

    let _ = tx_event.send(Event::Progress(
        super::event::ProgressPayload {
            message: format!("Chapter {} audit: {}", payload.chapter_number, if result.passed { "passed" } else { "failed" }),
            percent: Some(100.0),
        },
    )).await;

    Ok(())
}

/// Handle a revise chapter submission.
async fn handle_revise_chapter(
    _session_id: &str,
    config: &SessionConfig,
    payload: &super::op::ReviseChapterPayload,
    tx_event: &mpsc::Sender<Event>,
    _cancel_flag: &Arc<tokio::sync::RwLock<bool>>,
) -> Result<(), AppError> {
    use crate::core::agent::pipeline::{PipelineConfig, PipelineRunner};

    let workspace_path = {
        let db = config.db.lock().await;
        let ws = db.get_workspace(&payload.workspace_id).await?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        std::path::PathBuf::from(ws.path)
    };

    let runner = {
        let pipeline_config = PipelineConfig {
            provider: config.provider.clone(),
            model: config.model.clone(),
            project_root: workspace_path,
            model_overrides: config.model_overrides.clone(),
            memory_store: Some(config.memory_store.clone()),
            data_dir: config.data_dir.clone(),
            user_profile: None,
        };
        PipelineRunner::new(pipeline_config)
    };

    let _ = tx_event.send(Event::PipelineStageStarted(
        super::event::PipelineStageStartedPayload {
            book_id: payload.book_id.clone(),
            chapter_number: payload.chapter_number,
            stage: "revise".to_string(),
            label: "Revising chapter".to_string(),
        },
    )).await;

    let _revised = runner.revise_chapter(&payload.book_id, payload.chapter_number, Default::default()).await?;

    let _ = tx_event.send(Event::PipelineStageCompleted(
        super::event::PipelineStageCompletedPayload {
            book_id: payload.book_id.clone(),
            chapter_number: payload.chapter_number,
            stage: "revise".to_string(),
            label: "Chapter revised".to_string(),
        },
    )).await;

    let _ = tx_event.send(Event::Progress(
        super::event::ProgressPayload {
            message: format!("Chapter {} revised", payload.chapter_number),
            percent: Some(100.0),
        },
    )).await;

    Ok(())
}
