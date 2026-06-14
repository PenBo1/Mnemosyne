use crate::errors::AppError;
use crate::domain::harness::ContextBuilder;
use crate::domain::story::{ChapterContent, WriteResult, AuditResult};

use super::PipelineRunner;

impl PipelineRunner {
    pub async fn write_chapter(
        &self,
        book_id: &str,
        target_words: Option<u32>,
    ) -> Result<WriteResult, AppError> {
        let sm = self.story_manager();
        let mut state = sm.load_state(book_id)?;
        let book_config = sm.load_book_config(book_id)?;

        let next_chapter = state.current_chapter + 1;
        let words = target_words.unwrap_or(book_config.chapter_words);

        tracing::info!(book_id, chapter = next_chapter, target_words = words, "Writing chapter");
        let start = std::time::Instant::now();

        let agent_config = self.get_agent_config("writer");
        let ctx = self.build_context(book_id, "writer");
        let system = ContextBuilder::build_system_prompt(agent_config, &ctx, "");

        let intent_path = sm.story_dir(book_id).join("state").join(format!("chapter_{:04}_intent.json", next_chapter));
        let intent = if intent_path.exists() {
            std::fs::read_to_string(&intent_path).unwrap_or_default()
        } else {
            "{}".to_string()
        };

        let context_path = sm.story_dir(book_id).join("state").join(format!("chapter_{:04}_context.json", next_chapter));
        let context = if context_path.exists() {
            std::fs::read_to_string(&context_path).unwrap_or_default()
        } else {
            "{}".to_string()
        };

        let previous_content = if next_chapter > 1 {
            sm.load_chapter(book_id, next_chapter - 1)?
                .map(|c| c.content)
        } else {
            None
        };

        let prev_snippet = previous_content
            .as_ref()
            .map(|c| {
                let chars: Vec<char> = c.chars().collect();
                let len = chars.len();
                if len > 500 {
                    format!("...{}", chars[len-500..].iter().collect::<String>())
                } else {
                    c.clone()
                }
            })
            .unwrap_or_default();

        let user = format!(
            "章节意图：{}\n\n上下文包：{}\n\n目标字数：{}字\n\n前一章结尾（参考）：\n{}\n\n请生成第{}章正文",
            intent,
            context,
            words,
            prev_snippet,
            next_chapter
        );

        let response = self.call_llm(&system, &user).await?;
        let (title, content) = parse_writer_output(&response, next_chapter);
        let word_count = count_chinese_words(&content);

        tracing::info!(book_id, chapter = next_chapter, word_count, title = %title, "Chapter draft generated");

        let chapter = ChapterContent {
            number: next_chapter,
            title: title.clone(),
            content: content.clone(),
        };

        sm.save_chapter(book_id, &chapter)?;

        state.current_chapter = next_chapter;
        state.total_words += word_count;
        sm.save_state(book_id, &state)?;

        let elapsed = start.elapsed().as_secs();
        tracing::info!(book_id, chapter = next_chapter, word_count, elapsed_secs = elapsed, "Chapter write completed");

        Ok(WriteResult {
            chapter_number: next_chapter,
            title,
            content,
            word_count,
            audit: AuditResult {
                passed: true,
                score: 0.0,
                issues: Vec::new(),
                summary: String::new(),
            },
        })
    }

    pub async fn write_next_chapter(
        &self,
        book_id: &str,
        target_words: Option<u32>,
    ) -> Result<WriteResult, AppError> {
        tracing::info!(book_id, "Starting full pipeline: plan -> compose -> write -> audit -> revise -> reflect");
        let start = std::time::Instant::now();

        tracing::info!(book_id, "Stage: Plan");
        self.plan_chapter(book_id, None).await?;

        tracing::info!(book_id, "Stage: Compose");
        self.compose_chapter(book_id).await?;

        tracing::info!(book_id, "Stage: Write");
        let mut result = self.write_chapter(book_id, target_words).await?;

        tracing::info!(book_id, chapter = result.chapter_number, "Stage: Audit");
        let audit = self.audit_chapter(book_id, result.chapter_number).await?;

        let gate_eval = self.evaluate_gates(&result.content, Some(result.word_count), Some(&audit), book_id);

        if !audit.passed || !gate_eval.passed {
            let max_rounds = self.config.global_harness.project().pipeline_config.max_revision_rounds;
            let mut round = 0;
            let mut current_audit = audit;
            while round < max_rounds {
                if current_audit.issues.iter().any(|i| i.severity == crate::domain::story::AuditSeverity::Critical) {
                    tracing::info!(book_id, chapter = result.chapter_number, round, "Stage: Revise");
                    self.revise_chapter(book_id, result.chapter_number, &current_audit).await?;
                    current_audit = self.audit_chapter(book_id, result.chapter_number).await?;
                    round += 1;
                } else {
                    break;
                }
            }
        }

        tracing::info!(book_id, chapter = result.chapter_number, "Stage: Reflect");
        self.reflect_chapter(book_id, result.chapter_number).await?;

        let final_audit = self.audit_chapter(book_id, result.chapter_number).await?;
        let audit_passed = final_audit.passed;
        let audit_score = final_audit.score;
        result.audit = final_audit;

        self.record_audit_feedback(book_id, result.chapter_number, &result.audit).await;

        let gc_policy = &self.config.global_harness.project().gc_policy;
        if result.chapter_number % gc_policy.compact_state_every_n_chapters == 0 {
            tracing::info!(book_id, chapter = result.chapter_number, "Stage: GC");
            match crate::domain::harness::EntropyManager::gc_novel(
                &self.config.project_root,
                book_id,
                gc_policy,
            ) {
                Ok(report) => {
                    if report.snapshots_cleaned > 0 || report.state_compacted > 0 {
                        tracing::info!(
                            snapshots = report.snapshots_cleaned,
                            compacted = report.state_compacted,
                            "GC completed"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "GC failed");
                }
            }
        }

        let elapsed = start.elapsed().as_secs();
        tracing::info!(
            book_id,
            chapter = result.chapter_number,
            word_count = result.word_count,
            audit_passed,
            audit_score,
            elapsed_secs = elapsed,
            "Full pipeline completed"
        );

        Ok(result)
    }

    async fn record_audit_feedback(&self, book_id: &str, chapter_number: u32, audit: &AuditResult) {
        let rules = &self.config.global_harness.project().feedback_rules;
        if rules.is_empty() {
            return;
        }
        let db = match self.config.db.try_lock() {
            Ok(db) => db,
            Err(_) => return,
        };
        for issue in &audit.issues {
            if issue.severity == crate::domain::story::AuditSeverity::Critical
                || issue.severity == crate::domain::story::AuditSeverity::Warning
            {
                let severity = match issue.severity {
                    crate::domain::story::AuditSeverity::Critical => "critical",
                    crate::domain::story::AuditSeverity::Warning => "warning",
                    crate::domain::story::AuditSeverity::Info => "info",
                };
                if let Err(e) = self.record_feedback(
                    book_id,
                    chapter_number,
                    "auditor",
                    &issue.category,
                    Some(&issue.category),
                    severity,
                    &issue.description,
                    Some(issue.suggestion.as_str()),
                    rules,
                    &db,
                ) {
                    tracing::warn!(error = %e, "Failed to record feedback");
                }
            }
        }
    }
}

fn count_chinese_words(text: &str) -> u32 {
    let mut count = 0u32;
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch.is_ascii_punctuation() {
        } else if !ch.is_whitespace() {
            count += 1;
        }
    }
    let ascii_words: u32 = text.split_whitespace()
        .filter(|w| w.bytes().all(|b| b.is_ascii()))
        .count() as u32;
    count + ascii_words
}

fn parse_writer_output(raw: &str, chapter_number: u32) -> (String, String) {
    let lines: Vec<&str> = raw.lines().collect();
    let mut title = format!("第{}章", chapter_number);
    let mut content = String::new();
    let mut section: Option<&str> = None;

    for line in &lines {
        let trimmed = line.trim();
        if trimmed == "=== PRE_WRITE_CHECK ===" {
            section = Some("pre_write");
            continue;
        } else if trimmed == "=== CHAPTER_TITLE ===" {
            section = Some("title");
            continue;
        } else if trimmed == "=== CHAPTER_CONTENT ===" {
            section = Some("content");
            continue;
        }

        match section {
            Some("title") => {
                let t = trimmed.trim();
                if !t.is_empty() {
                    title = t.to_string();
                }
                section = None;
            }
            Some("content") => {
                if !content.is_empty() {
                    content.push('\n');
                }
                content.push_str(line);
            }
            Some(_) => {
                // pre_write or any other section — skip
            }
            None => {
                // No structured markers found — treat entire response as content
                if content.is_empty() && trimmed.is_empty() {
                    continue;
                }
                if !content.is_empty() {
                    content.push('\n');
                }
                content.push_str(line);
            }
        }
    }

    if content.trim().is_empty() {
        content = raw.to_string();
    }

    (title, content.trim().to_string())
}
