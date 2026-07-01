use async_trait::async_trait;
use crate::shared::errors::AppError;
use crate::infrastructure::file_storage::data_dir::DataDir;
use super::base::{AgentContext, BaseAgent};
use super::types::AgentRole;
use super::governance::*;
use super::planner::PlanOutput;
use super::agent_identity::AgentIdentity;
use crate::infrastructure::utils::context_assembly::{
    build_governed_rule_stack, build_governed_trace,
    apply_context_budget_if_needed, build_compiled_context_package,
    render_context_entries, BudgetApplication,
};

pub struct ComposerAgent;

impl Default for ComposerAgent {
    fn default() -> Self { Self }
}
impl ComposerAgent {
    pub fn new() -> Self { Self }

    /// S5.4: Compose chapter runtime context from truth files using governance.
    ///
    /// 当传入 `context_budget` 且总 token 超预算时，composer 会调用 LLM
    /// 编译可压缩段（compressible sources），protected 段永不压缩。
    /// 与 inkos `composeGovernedChapter` 行为对齐。
    pub async fn compose_chapter(
        &self,
        ctx: &AgentContext,
        book_dir: &std::path::Path,
        chapter_number: u32,
        plan: &PlanOutput,
        data_dir: &DataDir,
        context_budget: Option<ContextBudget>,
    ) -> Result<ComposeOutput, AppError> {
        let _identity = AgentIdentity::load(data_dir, "composer");
        let story_dir = book_dir.join("story");
        let truth_files = read_truth_files(&story_dir);

        // 1. 收集初始 context sources
        let selected_context = select_relevant_context(&truth_files, plan, chapter_number);
        let mut context_package = ContextPackage {
            chapter: chapter_number,
            selected_context,
        };
        let mut trace_notes: Vec<String> = Vec::new();

        // 2. S5.4: 应用上下文预算（如果提供）
        if let Some(budget) = context_budget {
            match apply_context_budget_if_needed(&context_package, budget) {
                BudgetApplication::WithinBudget => {
                    // 未超预算，原样使用
                }
                BudgetApplication::ProtectedExceedsBudget {
                    protected_tokens,
                    budget_tokens,
                } => {
                    return Err(AppError::internal(format!(
                        "Protected context exceeds available input budget ({} / {} tokens). \
                         Composer will not compress protected author intent, current focus, \
                         hard state, or active hook evidence.",
                        protected_tokens, budget_tokens
                    )));
                }
                BudgetApplication::OverBudgetNoCompressible => {
                    tracing::warn!(
                        chapter = chapter_number,
                        "[composer] context over budget but no compressible entries — keeping as-is"
                    );
                    trace_notes.push("context-over-budget-no-compressible-entries".to_string());
                }
                BudgetApplication::Compiled {
                    protected_tokens,
                    compressible_tokens,
                    compile_budget,
                    ..
                } => {
                    tracing::info!(
                        chapter = chapter_number,
                        protected_tokens,
                        compressible_tokens,
                        compile_budget,
                        "[composer] compiling compressible context"
                    );
                    let language = crate::infrastructure::state_store::gc::utils::read_book_language_from_dir(book_dir)
                        .unwrap_or_else(|| "zh".to_string());
                    let compiled = self
                        .compile_compressible_context(
                            ctx,
                            chapter_number,
                            &plan.intent.goal,
                            &language,
                            compile_budget,
                            &context_package,
                        )
                        .await?;
                    context_package = build_compiled_context_package(&context_package, compiled);
                    trace_notes.push("compiled-compressible-context".to_string());
                }
            }
        }

        // 3. 构建 rule stack
        let rule_stack = build_governed_rule_stack(plan, chapter_number);

        // 4. 构建 trace
        let trace = build_governed_trace(
            chapter_number,
            plan,
            &context_package,
            vec!["truth_files".to_string()],
            trace_notes,
        );

        // 5. 保存到磁盘
        let runtime_dir = story_dir.join("runtime");
        std::fs::create_dir_all(&runtime_dir)?;

        let context_path = runtime_dir.join(format!("chapter_{:04}_context.json", chapter_number));
        let context_json = serde_json::to_string_pretty(&context_package)?;
        std::fs::write(context_path, context_json)?;

        let rule_stack_path = runtime_dir.join(format!("chapter_{:04}_rules.json", chapter_number));
        let rule_stack_json = serde_json::to_string_pretty(&rule_stack)?;
        std::fs::write(rule_stack_path, rule_stack_json)?;

        let trace_path = runtime_dir.join(format!("chapter_{:04}_trace.json", chapter_number));
        let trace_json = serde_json::to_string_pretty(&trace)?;
        std::fs::write(trace_path, trace_json)?;

        Ok(ComposeOutput {
            context_package,
            rule_stack,
            trace,
        })
    }

    /// S5.4: 用 LLM 编译可压缩上下文段。
    ///
    /// 与 inkos `compileCompressibleContext` 对齐：
    /// - 只编译 compressible 段
    /// - protected 段作为参照传入，但不得改写
    /// - 输出简洁 Markdown，保留人名、未兑现承诺、证据、时间点
    async fn compile_compressible_context(
        &self,
        ctx: &AgentContext,
        chapter_number: u32,
        goal: &str,
        language: &str,
        compile_budget: u32,
        context_package: &ContextPackage,
    ) -> Result<String, AppError> {
        let protected_entries: Vec<&ContextSource> = context_package
            .selected_context
            .iter()
            .filter(|e| crate::infrastructure::utils::context_assembly::is_protected_context_source(&e.source))
            .collect();
        let compressible_entries: Vec<&ContextSource> = context_package
            .selected_context
            .iter()
            .filter(|e| !crate::infrastructure::utils::context_assembly::is_protected_context_source(&e.source))
            .collect();

        let protected_block = render_context_entries(
            &protected_entries.into_iter().cloned().collect::<Vec<_>>(),
        );
        let compressible_block = render_context_entries(
            &compressible_entries.into_iter().cloned().collect::<Vec<_>>(),
        );

        let (system, user) = if language == "en" {
            (
                "You are the semantic context compiler.\n\
                 Only compile the COMPRESSIBLE CONTEXT. The PROTECTED CONTEXT is binding reference material and must not be rewritten, summarized as a substitute, or weakened.\n\
                 Output concise Markdown with source pointers. Preserve names, unresolved promises, evidence, timing, and constraints that may affect the next chapter. Drop low-relevance noise.".to_string(),
                format!(
                    "Chapter: {}\nGoal: {}\nTarget budget for compiled context: <= {} estimated input tokens\n\n\
                     ## Protected Context (reference only, do not compile)\n{}\n\n\
                     ## Compressible Context (compile this)\n{}",
                    chapter_number, goal, compile_budget,
                    if protected_block.is_empty() { "(none)" } else { &protected_block },
                    if compressible_block.is_empty() { "(none)" } else { &compressible_block },
                ),
            )
        } else {
            (
                "你是语义上下文编译器。\n\
                 只能编译【可压缩上下文】。【受保护上下文】是绑定参照，不得改写、不得替代总结、不得削弱。\n\
                 输出简洁 Markdown，保留来源指针。保留会影响下一章的人名、未兑现承诺、证据、时间点和约束，丢弃低相关噪声。".to_string(),
                format!(
                    "章节：第{}章\n目标：{}\n压缩后目标预算：不超过 {} 估算输入 tokens\n\n\
                     ## 受保护上下文（只作为参照，不要编译它）\n{}\n\n\
                     ## 可压缩上下文（只编译这一部分）\n{}",
                    chapter_number, goal, compile_budget,
                    if protected_block.is_empty() { "（无）" } else { &protected_block },
                    if compressible_block.is_empty() { "（无）" } else { &compressible_block },
                ),
            )
        };

        let response = self.chat(ctx, &system, &user).await?;
        let compiled = response.content.trim().to_string();
        if compiled.is_empty() {
            return Err(AppError::internal(
                "Compressible context compiler returned empty output.".to_string()
            ));
        }
        Ok(compiled)
    }
}

#[async_trait]
impl BaseAgent for ComposerAgent {
    fn role(&self) -> AgentRole {
        AgentRole::Composer
    }

    fn name(&self) -> &str {
        "composer"
    }
}

pub struct ComposeOutput {
    pub context_package: ContextPackage,
    pub rule_stack: RuleStack,
    pub trace: ChapterTrace,
}

struct TruthFiles {
    story_frame: String,
    volume_map: String,
    book_rules: String,
    current_state: String,
    pending_hooks: String,
    chapter_summaries: String,
    character_matrix: String,
    author_intent: String,
    current_focus: String,
}

fn read_truth_files(story_dir: &std::path::Path) -> TruthFiles {
    let read_safe = |path: &std::path::Path| -> String {
        std::fs::read_to_string(path).unwrap_or_default()
    };

    let outline_dir = story_dir.join("outline");
    let story_frame = {
        let primary = read_safe(&outline_dir.join("story_frame.md"));
        if primary.is_empty() { read_safe(&story_dir.join("story_bible.md")) } else { primary }
    };
    let volume_map = {
        let primary = read_safe(&outline_dir.join("volume_map.md"));
        if primary.is_empty() { read_safe(&story_dir.join("volume_outline.md")) } else { primary }
    };

    TruthFiles {
        story_frame,
        volume_map,
        book_rules: read_safe(&story_dir.join("book_rules.md")),
        current_state: read_safe(&story_dir.join("current_state.md")),
        pending_hooks: read_safe(&story_dir.join("pending_hooks.md")),
        chapter_summaries: read_safe(&story_dir.join("chapter_summaries.md")),
        character_matrix: read_safe(&story_dir.join("character_matrix.md")),
        author_intent: read_safe(&story_dir.join("author_intent.md")),
        current_focus: read_safe(&story_dir.join("current_focus.md")),
    }
}

fn select_relevant_context(
    truth_files: &TruthFiles,
    _plan: &PlanOutput,
    chapter_number: u32,
) -> Vec<ContextSource> {
    let mut sources = Vec::new();

    // S5.5: 从 volume_map 提取本章 POV 角色
    let pov_character = crate::infrastructure::utils::pov_filter::extract_pov_from_outline(
        &truth_files.volume_map,
        chapter_number,
    );
    if let Some(ref pov) = pov_character {
        tracing::debug!(chapter = chapter_number, pov = %pov, "[composer] POV detected, applying POV-aware filtering");
    }

    // Protected sources (always included, never compressed)
    if !truth_files.story_frame.is_empty() {
        sources.push(ContextSource {
            source: "outline/story_frame.md".to_string(),
            reason: "World and story foundation".to_string(),
            excerpt: Some(cap(&truth_files.story_frame, 6000)),
        });
    }
    if !truth_files.volume_map.is_empty() {
        sources.push(ContextSource {
            source: "outline/volume_map.md".to_string(),
            reason: "Volume outline and pacing".to_string(),
            excerpt: Some(cap(&truth_files.volume_map, 5000)),
        });
    }
    if !truth_files.current_state.is_empty() {
        sources.push(ContextSource {
            source: "story/current_state.md".to_string(),
            reason: "Current story state".to_string(),
            excerpt: Some(cap(&truth_files.current_state, 3000)),
        });
    }

    // Compressible sources
    if !truth_files.book_rules.is_empty() {
        sources.push(ContextSource {
            source: "story/book_rules.md".to_string(),
            reason: "Writing rules and constraints".to_string(),
            excerpt: Some(cap(&truth_files.book_rules, 2000)),
        });
    }
    if !truth_files.author_intent.is_empty() {
        sources.push(ContextSource {
            source: "story/author_intent.md".to_string(),
            reason: "Author's long-term direction".to_string(),
            excerpt: Some(cap(&truth_files.author_intent, 2000)),
        });
    }
    if !truth_files.current_focus.is_empty() {
        sources.push(ContextSource {
            source: "story/current_focus.md".to_string(),
            reason: "Short-term focus".to_string(),
            excerpt: Some(cap(&truth_files.current_focus, 2000)),
        });
    }
    if !truth_files.pending_hooks.is_empty() {
        // S5.5: 应用 POV 过滤（如果有 POV 角色）
        let hooks_after_context_filter = crate::infrastructure::utils::context_filter::filter_hooks(&truth_files.pending_hooks);
        let filtered = if let Some(ref pov) = pov_character {
            crate::infrastructure::utils::pov_filter::filter_hooks_by_pov(
                &hooks_after_context_filter,
                pov,
                &truth_files.chapter_summaries,
            )
        } else {
            hooks_after_context_filter
        };
        sources.push(ContextSource {
            source: "story/pending_hooks.md".to_string(),
            reason: "Active hooks and foreshadowing".to_string(),
            excerpt: Some(cap(&filtered, 3000)),
        });
    }
    if !truth_files.chapter_summaries.is_empty() {
        let filtered = crate::infrastructure::utils::context_filter::filter_summaries(
            &truth_files.chapter_summaries, chapter_number, 5,
        );
        sources.push(ContextSource {
            source: "story/chapter_summaries.md".to_string(),
            reason: "Recent chapter summaries".to_string(),
            excerpt: Some(cap(&filtered, 3000)),
        });
    }
    if !truth_files.character_matrix.is_empty() {
        // S5.5: 应用 POV 过滤（如果有 POV 角色）
        let filtered = if let Some(ref pov) = pov_character {
            crate::infrastructure::utils::pov_filter::filter_matrix_by_pov(
                &truth_files.character_matrix,
                pov,
            )
        } else {
            truth_files.character_matrix.clone()
        };
        sources.push(ContextSource {
            source: "story/character_matrix.md".to_string(),
            reason: "Character relationships".to_string(),
            excerpt: Some(cap(&filtered, 3000)),
        });
    }

    sources
}

fn cap(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        text.to_string()
    } else {
        format!("{}…", &text[..max_chars])
    }
}
