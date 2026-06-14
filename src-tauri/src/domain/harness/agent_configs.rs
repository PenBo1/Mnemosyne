use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub role: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub prompt_template: String,
    #[serde(default)]
    pub tools: AgentTools,
    #[serde(default)]
    pub context: AgentContext,
    #[serde(default)]
    pub output: AgentOutputConfig,
    #[serde(default)]
    pub constraints: AgentConstraints,
    #[serde(default)]
    pub quality_standards: AgentQualityStandards,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentTools {
    #[serde(default)]
    pub allowed: Vec<String>,
    #[serde(default)]
    pub denied: Vec<String>,
    #[serde(default)]
    pub requires_approval: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentContext {
    #[serde(default)]
    pub required_sections: Vec<String>,
    #[serde(default)]
    pub optional_sections: Vec<String>,
    #[serde(default = "default_max_context_tokens")]
    pub max_context_tokens: u32,
    #[serde(default)]
    pub protected_sections: Vec<String>,
}

fn default_max_context_tokens() -> u32 {
    16000
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentOutputConfig {
    #[serde(default = "default_output_format")]
    pub format: String,
    pub structure: Option<serde_json::Value>,
    #[serde(default)]
    pub validation: OutputValidation,
}

fn default_output_format() -> String {
    "json".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OutputValidation {
    #[serde(default)]
    pub forbidden_patterns: Vec<String>,
    #[serde(default)]
    pub required_elements: Vec<String>,
    pub max_consecutive_short_sentences: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentConstraints {
    #[serde(default)]
    pub must_do: Vec<String>,
    #[serde(default)]
    pub must_not_do: Vec<String>,
    #[serde(default)]
    pub style_rules: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentQualityStandards {
    #[serde(default)]
    pub gate_ids: Vec<String>,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
}

/// Embedded agent configuration manager.
///
/// All agent configs are compiled into the binary.
/// No file-based loading — configs live in code only.
#[derive(Clone)]
pub struct AgentConfigManager {
    configs: Vec<AgentConfig>,
}

impl AgentConfigManager {
    /// Create with all embedded default configs.
    pub fn new() -> Self {
        Self {
            configs: vec![
                Self::architect_config(),
                Self::planner_config(),
                Self::composer_config(),
                Self::writer_config(),
                Self::auditor_config(),
                Self::reviser_config(),
                Self::observer_config(),
                Self::reflector_config(),
            ],
        }
    }

    pub fn get(&self, role: &str) -> Option<&AgentConfig> {
        self.configs.iter().find(|c| c.role == role)
    }

    pub fn all(&self) -> &[AgentConfig] {
        &self.configs
    }

    // ── Embedded configs ─────────────────────────────────────

    fn architect_config() -> AgentConfig {
        AgentConfig {
            role: "architect".into(),
            name: "Architect".into(),
            description: "Creates story framework, world settings, characters, and book-level rules".into(),
            prompt_template: "You are a story architecture specialist. Given the author's brief and genre, create the foundational story structure.\n\n## Output format (JSON)\n\n```json\n{\n  \"story_frame\": {\n    \"premise\": \"\",\n    \"conflict\": \"\",\n    \"resolution_direction\": \"\",\n    \"themes\": [\"\", \"\"]\n  },\n  \"story_bible\": {\n    \"world_rules\": [\"\", \"\"],\n    \"power_system\": \"\",\n    \"society_structure\": \"\"\n  },\n  \"characters\": [\n    {\n      \"name\": \"\",\n      \"role\": \"protagonist|antagonist|supporting\",\n      \"personality\": [\"\", \"\"],\n      \"motivation\": \"\",\n      \"arc_direction\": \"\"\n    }\n  ],\n  \"book_rules\": {\n    \"style_rules\": [\"\", \"\"],\n    \"pacing_rules\": [\"\", \"\"],\n    \"forbidden_patterns\": [\"\", \"\"]\n  },\n  \"author_intent\": {\n    \"tone\": \"\",\n    \"target_audience\": \"\",\n    \"core_message\": \"\"\n  }\n}\n```\n\n## Rules\n- World must be internally consistent\n- Characters must have depth, avoid stereotypes\n- Book rules must be concrete and executable\n- No more than 5 main characters".into(),
            tools: AgentTools {
                allowed: vec!["novel_info".into()],
                denied: vec!["write_file".into(), "read_file".into(), "grep".into(), "glob".into()],
                requires_approval: vec![],
            },
            context: AgentContext {
                required_sections: vec!["author_brief".into(), "genre_preset".into()],
                optional_sections: vec![],
                max_context_tokens: 8000,
                protected_sections: vec![],
            },
            output: AgentOutputConfig {
                format: "json".into(),
                ..Default::default()
            },
            constraints: AgentConstraints {
                must_do: vec![
                    "World must be internally consistent".into(),
                    "Characters must have depth".into(),
                    "Book rules must be concrete".into(),
                    "Output JSON format".into(),
                ],
                must_not_do: vec![
                    "Must not create without brief".into(),
                    "Must not create more than 5 main characters".into(),
                    "Must not create contradictory settings".into(),
                ],
                style_rules: vec![],
            },
            quality_standards: AgentQualityStandards {
                gate_ids: vec!["g_plan_completeness".into()],
                acceptance_criteria: vec![
                    "Complete story framework".into(),
                    "At least 2 main characters".into(),
                    "World without internal contradictions".into(),
                ],
            },
            extra: HashMap::new(),
        }
    }

    fn planner_config() -> AgentConfig {
        AgentConfig {
            role: "planner".into(),
            name: "Planner".into(),
            description: "Plans chapter content and structure, produces chapter memo".into(),
            prompt_template: "You are this novel's editor-in-chief. Your job is to produce a chapter_memo for the next chapter. You do NOT write prose — you plan what this chapter must accomplish.\n\n## Output format (strict)\n\n# Chapter N memo\n\n## Chapter goal\n<one sentence>\n\n## Current task\n<concrete action the protagonist must complete>\n\n## What the reader is waiting for right now\n<what the reader expects, what this chapter does with that expectation>\n\n## To pay off / to keep buried\n- Pay off: X\n- Keep buried: Y\n\n## Required end-of-chapter change\n<1-3 concrete changes>\n\n## Hook ledger for this chapter\nopen:\n- [new] description\nadvance:\n- H0XX description\nresolve:\n- H0XX description\ndefer:\n- H0XX description\n\n## Do not\n<2-4 hard prohibitions>".into(),
            tools: AgentTools {
                allowed: vec!["novel_info".into(), "chapter_list".into(), "character_list".into(), "memory_search".into()],
                denied: vec!["write_file".into()],
                requires_approval: vec![],
            },
            context: AgentContext {
                required_sections: vec!["author_intent".into(), "current_focus".into(), "book_rules".into(), "story_state".into(), "recent_summaries".into()],
                optional_sections: vec!["active_hooks".into(), "relevant_facts".into()],
                max_context_tokens: 10000,
                protected_sections: vec!["author_intent".into(), "current_focus".into()],
            },
            output: AgentOutputConfig {
                format: "json".into(),
                ..Default::default()
            },
            constraints: AgentConstraints {
                must_do: vec![
                    "Plan based on author intent and current focus".into(),
                    "Consider advancing active hooks".into(),
                    "Consider character development arcs".into(),
                ],
                must_not_do: vec![
                    "Must not deviate from author's long-term intent".into(),
                    "Must not ignore unresolved hooks".into(),
                    "Must not generate empty must_keep".into(),
                ],
                style_rules: vec![],
            },
            quality_standards: AgentQualityStandards {
                gate_ids: vec!["g_plan_completeness".into()],
                acceptance_criteria: vec![
                    "must_keep non-empty".into(),
                    "must_avoid non-empty".into(),
                    "Consistent with current focus".into(),
                ],
            },
            extra: HashMap::new(),
        }
    }

    fn composer_config() -> AgentConfig {
        AgentConfig {
            role: "composer".into(),
            name: "Composer".into(),
            description: "Assembles a concise context package for the writer".into(),
            prompt_template: "You are a context assembly specialist. Given the chapter intent and story state, assemble a concise context package for the writer.\n\n## Output format (JSON)\n\n```json\n{\n  \"relevant_facts\": [{ \"subject\": \"\", \"predicate\": \"\", \"object\": \"\" }],\n  \"active_hooks\": [{ \"name\": \"\", \"status\": \"\", \"description\": \"\" }],\n  \"recent_summaries\": [{ \"chapter\": N, \"title\": \"\", \"events\": [] }],\n  \"selected_rules\": [\"rule1\", \"rule2\"]\n}\n```\n\n## Rules\n- Filter by relevance, do NOT inject everything\n- Prioritize protected sections (chapter_intent)\n- Stay within the writer's context budget\n- Include all facts related to must_keep items".into(),
            tools: AgentTools {
                allowed: vec!["novel_info".into(), "chapter_list".into(), "character_list".into(), "world_setting_list".into(), "memory_search".into(), "read_file".into()],
                denied: vec!["write_file".into()],
                requires_approval: vec![],
            },
            context: AgentContext {
                required_sections: vec!["chapter_intent".into(), "story_state".into(), "author_intent".into(), "book_rules".into()],
                optional_sections: vec!["current_focus".into()],
                max_context_tokens: 20000,
                protected_sections: vec!["chapter_intent".into()],
            },
            output: AgentOutputConfig {
                format: "json".into(),
                ..Default::default()
            },
            constraints: AgentConstraints {
                must_do: vec![
                    "Filter information by relevance".into(),
                    "Prioritize protected information".into(),
                    "Control total tokens within writer's budget".into(),
                ],
                must_not_do: vec![
                    "Must not omit must_keep related info".into(),
                    "Must not inject irrelevant info".into(),
                    "Must not exceed context budget".into(),
                ],
                style_rules: vec![],
            },
            quality_standards: AgentQualityStandards {
                gate_ids: vec!["g_compose_completeness".into()],
                acceptance_criteria: vec![
                    "Contains chapter intent".into(),
                    "Contains relevant facts".into(),
                    "Total tokens < 12000".into(),
                ],
            },
            extra: HashMap::new(),
        }
    }

    fn writer_config() -> AgentConfig {
        AgentConfig {
            role: "writer".into(),
            name: "Writer".into(),
            description: "Writes chapter prose following all constraints and style rules".into(),
            prompt_template: "You are a skilled novelist. Write chapter prose following the chapter memo and context package.\n\n## Output format\n\n=== PRE_WRITE_CHECK ===\n<brief pre-flight check of what you will write>\n\n=== CHAPTER_TITLE ===\n<title>\n\n=== CHAPTER_CONTENT ===\n<full chapter prose>\n\n## Rules\n- Follow the chapter memo strictly\n- Maintain character voice consistency\n- Use natural Chinese/Avoid AI-flavored phrases\n- Target word count as specified\n- End with a hook or cliffhanger".into(),
            tools: AgentTools {
                allowed: vec!["novel_info".into(), "chapter_list".into(), "character_list".into(), "memory_search".into()],
                denied: vec!["write_file".into()],
                requires_approval: vec![],
            },
            context: AgentContext {
                required_sections: vec!["chapter_intent".into(), "chapter_context".into(), "previous_chapter_snippet".into(), "story_state".into()],
                optional_sections: vec!["active_hooks".into(), "relevant_facts".into(), "constraint_lessons".into()],
                max_context_tokens: 20000,
                protected_sections: vec!["chapter_intent".into()],
            },
            output: AgentOutputConfig {
                format: "markdown".into(),
                validation: OutputValidation {
                    forbidden_patterns: vec![
                        "值得一提的是".into(),
                        "不禁".into(),
                        "缓缓".into(),
                        "仿佛".into(),
                        "宛如".into(),
                    ],
                    required_elements: vec![],
                    max_consecutive_short_sentences: Some(3),
                },
                ..Default::default()
            },
            constraints: AgentConstraints {
                must_do: vec![
                    "Follow the chapter memo".into(),
                    "Maintain character voice".into(),
                    "Use natural language".into(),
                    "Include required hook progression".into(),
                ],
                must_not_do: vec![
                    "Must not introduce AI-flavored phrases".into(),
                    "Must not contradict established facts".into(),
                    "Must not skip required story elements".into(),
                ],
                style_rules: vec![
                    "Avoid consecutive short sentences".into(),
                    "Maintain consistent narrative distance".into(),
                    "Use concrete sensory details".into(),
                ],
            },
            quality_standards: AgentQualityStandards {
                gate_ids: vec!["g_write_wordcount".into(), "g_write_forbidden".into()],
                acceptance_criteria: vec![
                    "Word count within target range".into(),
                    "No forbidden phrases".into(),
                    "All required elements present".into(),
                ],
            },
            extra: HashMap::new(),
        }
    }

    fn auditor_config() -> AgentConfig {
        AgentConfig {
            role: "auditor".into(),
            name: "Auditor".into(),
            description: "Checks chapter draft for continuity and quality across 10 dimensions".into(),
            prompt_template: "You are a novel quality assurance expert. Audit the chapter draft across these 10 dimensions:\n\n1. **OOC Check**: Are characters behaving consistently with their established personality?\n2. **Timeline Check**: Are events in correct chronological order? Any time paradoxes?\n3. **Lore Conflict**: Does the chapter contradict established world rules or settings?\n4. **Hook Check**: Are existing hooks being advanced or resolved appropriately?\n5. **Pacing Check**: Is the pacing appropriate for the story's current arc?\n6. **Style Check**: Is the writing style consistent with previous chapters?\n7. **Lexical Fatigue**: Are there repeated phrases or words that feel AI-generated?\n8. **Dialogue Authenticity**: Do characters speak naturally and distinctly?\n9. **Paragraph Uniformity**: Are paragraph lengths varied and appropriate?\n10. **Cliche Density**: Are there overused tropes or cliches?\n\n## Output format (JSON)\n\n```json\n{\n  \"passed\": true/false,\n  \"score\": 0-100,\n  \"issues\": [\n    {\n      \"severity\": \"critical|warning|info\",\n      \"category\": \"dimension_name\",\n      \"description\": \"What is wrong\",\n      \"suggestion\": \"How to fix it\"\n    }\n  ],\n  \"summary\": \"Overall assessment\"\n}\n```\n\n## Rules\n- Check each dimension systematically\n- Only report real issues, not hypothetical ones\n- Every issue must have a concrete fix suggestion\n- Critical = breaks story logic; Warning = noticeable quality issue; Info = minor suggestion".into(),
            tools: AgentTools {
                allowed: vec!["read_file".into(), "memory_search".into(), "chapter_list".into(), "character_list".into()],
                denied: vec!["write_file".into()],
                requires_approval: vec![],
            },
            context: AgentContext {
                required_sections: vec!["chapter_content".into(), "chapter_intent".into(), "story_state".into(), "continuity_rules".into()],
                optional_sections: vec!["previous_chapter_content".into(), "active_hooks".into(), "recent_summaries".into(), "character_list".into()],
                max_context_tokens: 16000,
                protected_sections: vec!["continuity_rules".into()],
            },
            output: AgentOutputConfig {
                format: "json".into(),
                ..Default::default()
            },
            constraints: AgentConstraints {
                must_do: vec![
                    "Check each dimension systematically".into(),
                    "Only report real issues".into(),
                    "Provide fix suggestions for every issue".into(),
                ],
                must_not_do: vec![
                    "Must not fabricate issues".into(),
                    "Must not ignore Critical-level problems".into(),
                ],
                style_rules: vec![],
            },
            quality_standards: AgentQualityStandards {
                gate_ids: vec!["g_audit_score".into(), "g_audit_critical".into()],
                acceptance_criteria: vec![
                    "Total score >= 70".into(),
                    "Critical issues = 0".into(),
                ],
            },
            extra: HashMap::new(),
        }
    }

    fn reviser_config() -> AgentConfig {
        AgentConfig {
            role: "reviser".into(),
            name: "Reviser".into(),
            description: "Revises chapter based on audit feedback, fixing critical and warning issues".into(),
            prompt_template: "You are a revision specialist. Revise the chapter to fix all critical and warning issues identified in the audit.\n\n## Revision principles\n1. Fix all Critical issues first\n2. Then fix Warning issues\n3. Preserve the original style and tone\n4. Minimize changes — only modify what's necessary\n5. Do NOT introduce new problems\n\n## Output\nReturn the full revised chapter text. Do NOT include explanations or meta-commentary.".into(),
            tools: AgentTools {
                allowed: vec!["read_file".into(), "write_file".into()],
                denied: vec!["delete_file".into()],
                requires_approval: vec!["write_file".into()],
            },
            context: AgentContext {
                required_sections: vec!["original_chapter".into(), "audit_result".into(), "gate_failures".into(), "constraint_lessons".into()],
                optional_sections: vec!["chapter_intent".into(), "style_profile".into()],
                max_context_tokens: 16000,
                protected_sections: vec!["constraint_lessons".into()],
            },
            output: AgentOutputConfig {
                format: "markdown".into(),
                validation: OutputValidation {
                    forbidden_patterns: vec!["值得一提的是".into(), "不禁".into(), "缓缓".into()],
                    required_elements: vec![],
                    max_consecutive_short_sentences: Some(3),
                },
                ..Default::default()
            },
            constraints: AgentConstraints {
                must_do: vec![
                    "Fix all Critical issues".into(),
                    "Fix Warning issues when possible".into(),
                    "Preserve original style and rhythm".into(),
                    "Do not introduce new problems".into(),
                ],
                must_not_do: vec![
                    "Must not do major rewrites".into(),
                    "Must not change plot direction".into(),
                    "Must not delete characters or scenes".into(),
                    "Must not introduce forbidden phrases".into(),
                ],
                style_rules: vec![
                    "Minimize changes".into(),
                    "Maintain character voice consistency".into(),
                    "Maintain plot logic continuity".into(),
                ],
            },
            quality_standards: AgentQualityStandards {
                gate_ids: vec!["g_write_wordcount".into(), "g_write_forbidden".into(), "g_audit_score".into(), "g_audit_critical".into()],
                acceptance_criteria: vec![
                    "Revised version passes all gates".into(),
                    "All Critical issues fixed".into(),
                    "No new issues introduced".into(),
                ],
            },
            extra: HashMap::new(),
        }
    }

    fn observer_config() -> AgentConfig {
        AgentConfig {
            role: "observer".into(),
            name: "Observer".into(),
            description: "Extracts structured facts from chapter text across 9 categories".into(),
            prompt_template: "You are a fact extraction specialist. Read the chapter text and extract EVERY observable fact change.\n\n## Extraction Categories\n\n1. **Character actions**: Who did what, to whom, why\n2. **Location changes**: Who moved where\n3. **Resource changes**: Items gained, lost, consumed\n4. **Relationship changes**: New encounters, trust shifts\n5. **Emotional shifts**: Character mood before → after\n6. **Information flow**: Who learned what\n7. **Plot threads**: New mysteries, advances, resolutions\n8. **Time progression**: How much time passed\n9. **Physical state**: Injuries, healing, fatigue\n\n## Rules\n- Extract from the TEXT ONLY — do not infer\n- Over-extract: if unsure, include it\n- Be specific\n\n## Output format (JSON)\n\n```json\n{\n  \"facts\": [{ \"subject\": \"\", \"predicate\": \"\", \"object\": \"\", \"category\": \"\" }],\n  \"hooks_new\": [{ \"name\": \"\", \"type\": \"\", \"description\": \"\" }],\n  \"hooks_advanced\": [{ \"name\": \"\", \"status\": \"Open|Progressing|Resolved|Deferred\", \"description\": \"\" }],\n  \"chapter_summary\": { \"chapter\": N, \"title\": \"\", \"characters\": [], \"events\": [], \"state_changes\": [], \"mood\": \"\" }\n}\n```".into(),
            tools: AgentTools {
                allowed: vec!["read_file".into(), "memory_search".into()],
                denied: vec!["write_file".into()],
                requires_approval: vec![],
            },
            context: AgentContext {
                required_sections: vec!["chapter_content".into()],
                optional_sections: vec!["story_state".into(), "character_list".into(), "previous_observations".into()],
                max_context_tokens: 12000,
                protected_sections: vec![],
            },
            output: AgentOutputConfig {
                format: "json".into(),
                ..Default::default()
            },
            constraints: AgentConstraints {
                must_do: vec![
                    "Extract facts with clear textual evidence".into(),
                    "Distinguish new facts from state changes".into(),
                    "Record hook progression status".into(),
                ],
                must_not_do: vec![
                    "Must not infer information not in the text".into(),
                    "Must not miss Critical-level facts".into(),
                    "Must not re-extract known facts".into(),
                ],
                style_rules: vec![],
            },
            quality_standards: AgentQualityStandards {
                gate_ids: vec![],
                acceptance_criteria: vec![
                    "Fact count reasonable (5-30)".into(),
                    "Each fact has clear subject/predicate/object".into(),
                ],
            },
            extra: HashMap::new(),
        }
    }

    fn reflector_config() -> AgentConfig {
        AgentConfig {
            role: "reflector".into(),
            name: "Reflector".into(),
            description: "Updates story state from observer's extraction results".into(),
            prompt_template: "You are a state management specialist. Given the observer's extraction results, produce a state delta that updates the story state.\n\n## Output format (JSON)\n\n```json\n{\n  \"hooks_new\": [{ \"name\": \"\", \"type\": \"foreshadowing|recurring|setup|callback\", \"description\": \"\" }],\n  \"hooks_advanced\": [{ \"name\": \"\", \"status\": \"Open|Progressing|Resolved|Deferred\", \"description\": \"\" }],\n  \"facts_new\": [{ \"subject\": \"\", \"predicate\": \"\", \"object\": \"\", \"category\": \"\" }],\n  \"summary_new\": { \"chapter\": N, \"title\": \"\", \"characters\": [], \"events\": [], \"state_changes\": [], \"mood\": \"\" }\n}\n```\n\n## Rules\n- Only include CHANGES (delta), not the full state\n- Do not delete existing facts\n- Do not modify state from other chapters\n- Validate JSON schema before output".into(),
            tools: AgentTools {
                allowed: vec!["write_file".into(), "memory_search".into()],
                denied: vec![],
                requires_approval: vec!["write_file".into()],
            },
            context: AgentContext {
                required_sections: vec!["observation".into(), "current_story_state".into()],
                optional_sections: vec!["constraint_lessons".into()],
                max_context_tokens: 10000,
                protected_sections: vec![],
            },
            output: AgentOutputConfig {
                format: "json".into(),
                ..Default::default()
            },
            constraints: AgentConstraints {
                must_do: vec![
                    "Only update changes (delta)".into(),
                    "Validate JSON schema before writing".into(),
                    "Create snapshot before update".into(),
                ],
                must_not_do: vec![
                    "Must not delete existing facts".into(),
                    "Must not modify state from other chapters".into(),
                    "Must not skip schema validation".into(),
                ],
                style_rules: vec![],
            },
            quality_standards: AgentQualityStandards {
                gate_ids: vec![],
                acceptance_criteria: vec![
                    "Output matches RuntimeStateDelta schema".into(),
                    "Snapshot created successfully".into(),
                    "State readable after update".into(),
                ],
            },
            extra: HashMap::new(),
        }
    }
}

impl Default for AgentConfigManager {
    fn default() -> Self {
        Self::new()
    }
}
