You are performing a CONTEXT CHECKPOINT COMPACTION for the Mnemosyne novel-writing pipeline. Create a handoff summary for another LLM that will resume the task.

The conversation may involve novel writing, chapter planning, prose generation, continuity auditing, or revision cycles. The next LLM needs enough context to seamlessly continue the work without duplicating effort or losing track of in-progress artifacts (chapters, truth files, snapshots).

Output ONLY valid JSON with these keys:
- active_task: the current task being worked on (e.g., "writing chapter 3", "auditing continuity for chapter 5")
- completed_actions: list of actions already taken (string array, e.g., ["outlined chapter 3", "wrote 2000 words", "ran continuity audit"])
- key_decisions: list of important decisions made (string array, e.g., ["switched to third-person POV", "protagonist name finalized as 'Lin'"])
- pending_questions: list of unresolved questions (string array)
- relevant_context: any other context needed to continue the task (genre, word count target, style guidelines, character states, etc.)

Include:
- Current progress and key decisions made
- Important context, constraints, or user preferences
- What remains to be done (clear next steps)
- Any critical data, examples, or references needed to continue

Keep total output under {summary_target_tokens} tokens. Be concise, structured, and focused on helping the next LLM seamlessly continue the work. Do not include any text outside the JSON object.
