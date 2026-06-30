use async_trait::async_trait;
use crate::shared::errors::AppError;
use crate::infrastructure::file_storage::data_dir::DataDir;
use super::base::{AgentContext, BaseAgent};
use super::types::AgentRole;
use super::governance::*;
use super::planner::PlanOutput;
use super::agent_identity::AgentIdentity;
use crate::infrastructure::utils::context_assembly::{build_governed_rule_stack, build_governed_trace};

pub struct ComposerAgent;

impl Default for ComposerAgent {
    fn default() -> Self { Self }
}
impl ComposerAgent {
    pub fn new() -> Self { Self }

    /// Compose chapter runtime context from truth files using governance.
    ///
    /// The composer is a pure-logic agent (no LLM calls), but it loads its
    /// identity for consistency with the agent identity system. The identity
    /// is available if future versions add LLM-based context selection.
    pub async fn compose_chapter(
        &self,
        _ctx: &AgentContext,
        book_dir: &std::path::Path,
        chapter_number: u32,
        plan: &PlanOutput,
        data_dir: &DataDir,
    ) -> Result<ComposeOutput, AppError> {
        // Load identity for consistency (pure-logic agent, but maintains the pattern)
        let _identity = AgentIdentity::load(data_dir, "composer");
        // TODO: In future versions, identity could influence context selection strategy
        let story_dir = book_dir.join("story");
        let truth_files = read_truth_files(&story_dir);

        // Build governed context package
        let selected_context = select_relevant_context(&truth_files, plan, chapter_number);

        let context_package = ContextPackage {
            chapter: chapter_number,
            selected_context,
        };

        // Build rule stack
        let rule_stack = build_governed_rule_stack(plan, chapter_number);

        // Build trace
        let trace = build_governed_trace(
            chapter_number,
            plan,
            &context_package,
            vec!["truth_files".to_string()],
            Vec::new(),
        );

        // Save to disk
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
    _chapter_number: u32,
) -> Vec<ContextSource> {
    let mut sources = Vec::new();

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
        let filtered = crate::infrastructure::utils::context_filter::filter_hooks(&truth_files.pending_hooks);
        sources.push(ContextSource {
            source: "story/pending_hooks.md".to_string(),
            reason: "Active hooks and foreshadowing".to_string(),
            excerpt: Some(cap(&filtered, 3000)),
        });
    }
    if !truth_files.chapter_summaries.is_empty() {
        let filtered = crate::infrastructure::utils::context_filter::filter_summaries(
            &truth_files.chapter_summaries, _chapter_number, 5,
        );
        sources.push(ContextSource {
            source: "story/chapter_summaries.md".to_string(),
            reason: "Recent chapter summaries".to_string(),
            excerpt: Some(cap(&filtered, 3000)),
        });
    }
    if !truth_files.character_matrix.is_empty() {
        sources.push(ContextSource {
            source: "story/character_matrix.md".to_string(),
            reason: "Character relationships".to_string(),
            excerpt: Some(cap(&truth_files.character_matrix, 3000)),
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
