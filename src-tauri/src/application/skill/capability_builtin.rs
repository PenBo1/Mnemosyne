//! ═══════════════════════════════════════════════════════════════════════════
//! Capability Builtin - 内置能力技能定义
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 定义 3 个内置能力技能：
//! - longform-writing：长篇连载小说创作
//! - open-world-play：开放世界互动小说
//! - interactive-film-authoring：互动影游编剧

use super::capability_types::*;

/// 内置 capability skills 列表
pub fn builtin_capability_skills() -> Vec<CapabilitySkillManifest> {
    vec![
        longform_writing_skill(),
        open_world_play_skill(),
        interactive_film_authoring_skill(),
    ]
}

/// 内置 prompt packs 列表
pub fn builtin_prompt_packs() -> Vec<PromptPackManifest> {
    vec![
        PromptPackManifest {
            id: "longform".into(),
            title: "Longform Writing".into(),
            description: "Core long-form writing prompts used by chapter production and repair.".into(),
            prompts: vec![
                "longform.writer".into(),
                "longform.reviser".into(),
                "longform.auditor".into(),
            ],
            source: SkillSource::Builtin,
        },
        PromptPackManifest {
            id: "play".into(),
            title: "Mnemosyne Play".into(),
            description: "Open-world / branching interaction prompts for world mutation, rendering, reconciliation, and images.".into(),
            prompts: vec![
                "play.start".into(),
                "play.mutator".into(),
                "play.renderer".into(),
                "play.reconciler".into(),
                "play.image".into(),
            ],
            source: SkillSource::Builtin,
        },
        PromptPackManifest {
            id: "interactive-film".into(),
            title: "Interactive Film Authoring".into(),
            description: "Script, storyboard, story graph, and image-planning prompts for interactive-film projects.".into(),
            prompts: vec![
                "interactive-film.script".into(),
                "interactive-film.storyboard".into(),
                "interactive-film.story-graph".into(),
                "interactive-film.image-plan".into(),
            ],
            source: SkillSource::Builtin,
        },
    ]
}

/// 内置 prompts 列表
pub fn builtin_prompts() -> Vec<BuiltinPrompt> {
    vec![
        // === longform pack ===
        BuiltinPrompt {
            id: "longform.writer".into(),
            pack_id: "longform".into(),
            title: "Longform Writer".into(),
            content: "You are Mnemosyne's long-form chapter writer.\nWrite prose from the governed chapter intent and selected context package.\nProtected context is binding. Compressible context is supporting memory.\nDo not override author intent, current focus, hard facts, or active hook evidence with genre defaults.".into(),
        },
        BuiltinPrompt {
            id: "longform.reviser".into(),
            pack_id: "longform".into(),
            title: "Longform Reviser".into(),
            content: "You are Mnemosyne's long-form reviser.\nFix the chapter according to audit issues while preserving established facts and the chapter goal.\nIf a repair requires changing higher-level state, surface that need instead of silently rewriting canon.".into(),
        },
        BuiltinPrompt {
            id: "longform.auditor".into(),
            pack_id: "longform".into(),
            title: "Longform Auditor".into(),
            content: "You are Mnemosyne's continuity and quality auditor.\nCheck whether the chapter follows protected intent, hard facts, active hooks, proportions, and craft requirements.\nReport unresolved issues plainly; do not mark a failed chapter as fixed.".into(),
        },
        // === play pack ===
        BuiltinPrompt {
            id: "play.start".into(),
            pack_id: "play".into(),
            title: "Play Start".into(),
            content: "You are Mnemosyne Play's world-start guide.\nHelp confirm the playable premise, world contract, player persona, time semantics, and visual contract before starting.\nDo not force RPG levels or fixed stats unless the user asks for them.".into(),
        },
        BuiltinPrompt {
            id: "play.mutator".into(),
            pack_id: "play".into(),
            title: "Play World Mutator".into(),
            content: "You are Mnemosyne Play's world mutation engine.\nTurn the player action into state changes: scene, entities, relationships, evidence, inventory, time, and consequences.\nRespect the world contract and preserve actor_player as the player entity id.".into(),
        },
        BuiltinPrompt {
            id: "play.renderer".into(),
            pack_id: "play".into(),
            title: "Play Scene Renderer".into(),
            content: "You are Mnemosyne Play's scene renderer.\nRender the applied world mutation as vivid interactive prose.\nDo not invent concrete objects, evidence, or characters that are absent from applied state unless the reconciler can record them.".into(),
        },
        BuiltinPrompt {
            id: "play.reconciler".into(),
            pack_id: "play".into(),
            title: "Play Scene Reconciler".into(),
            content: "You reconcile rendered scene prose back into the graph state.\nExtract newly mentioned concrete entities, evidence, relationships, and locations so state does not drift from narration.".into(),
        },
        BuiltinPrompt {
            id: "play.image".into(),
            pack_id: "play".into(),
            title: "Play Image Prompt".into(),
            content: "Create image prompts from the current play scene and visual contract.\nFollow user-defined visual semantics. Do not add watermarks, UI frames, text overlays, or default rarity borders unless requested.".into(),
        },
        // === interactive-film pack ===
        BuiltinPrompt {
            id: "interactive-film.script".into(),
            pack_id: "interactive-film".into(),
            title: "Interactive Film Script".into(),
            content: "You are an interactive-film script writer.\nConvert the confirmed premise/source into playable scenes, dialogue, choices, variables, and endings.\nLeave creative space to the user; ask or preserve format constraints instead of inventing production rules.".into(),
        },
        BuiltinPrompt {
            id: "interactive-film.storyboard".into(),
            pack_id: "interactive-film".into(),
            title: "Interactive Film Storyboard".into(),
            content: "You are an interactive-film storyboard designer.\nTurn script beats into shot-level visual plans with clear action, composition, and image prompts.\nDo not require video output; produce still-image/storyboard assets unless the user asks otherwise.".into(),
        },
        BuiltinPrompt {
            id: "interactive-film.story-graph".into(),
            pack_id: "interactive-film".into(),
            title: "Interactive Film Story Graph".into(),
            content: "You are an interactive-film story graph designer.\nCreate a playable graph: nodes, choices, variables/flags, and multiple endings.\nEvery branch must remain reachable and every path should resolve to an ending.".into(),
        },
        BuiltinPrompt {
            id: "interactive-film.image-plan".into(),
            pack_id: "interactive-film".into(),
            title: "Interactive Film Image Plan".into(),
            content: "Create image plans for interactive-film nodes and assets.\nUse sceneKey/location continuity when available, but do not require full-screen game UI or video conversion.".into(),
        },
    ]
}

// ── 3 个内置能力技能 ────────────────────────────────────────────────────────

fn longform_writing_skill() -> CapabilitySkillManifest {
    CapabilitySkillManifest {
        id: "longform-writing".into(),
        name: "Longform Writing".into(),
        description: "Long-form serial novel creation: plan chapters, select story context, write, audit, revise, and preserve continuity.".into(),
        when_to_use: "Use for long-form books, chapter planning, chapter continuation, audit repair, truth/state consistency, and author-intent driven writing.".into(),
        triggers: vec![
            "长篇".into(), "章节".into(), "下一章".into(), "续写".into(),
            "审稿".into(), "修稿".into(), "伏笔".into(),
            "author intent".into(), "chapter".into(), "write next".into(), "longform".into(),
        ],
        session_kinds: vec!["book".into(), "book-create".into()],
        prompt_packs: vec![
            "longform.writer".into(), "longform.reviser".into(), "longform.auditor".into(),
        ],
        tool_hints: vec![
            "plan_chapter".into(), "compose_chapter".into(), "write_draft".into(),
            "write_full_pipeline".into(), "audit_chapter".into(), "revise_chapter".into(),
        ],
        context_needs: vec![
            SkillContextNeed {
                id: "author-intent".into(),
                purpose: "Bind long-horizon user intent and prevent model defaults from overriding the book direction.".into(),
                sources: vec!["story/author_intent.md".into()],
                tier: SkillContextTier::Protected,
                applies_to: vec!["planner".into(), "composer".into(), "writer".into(), "auditor".into(), "reviser".into(), "chat".into()],
                retrieval: SkillContextRetrieval::Full,
            },
            SkillContextNeed {
                id: "current-focus".into(),
                purpose: "Carry the next-chapter steering and short-horizon focus.".into(),
                sources: vec!["story/current_focus.md".into()],
                tier: SkillContextTier::Protected,
                applies_to: vec!["planner".into(), "composer".into(), "writer".into(), "auditor".into(), "reviser".into(), "chat".into()],
                retrieval: SkillContextRetrieval::Full,
            },
            SkillContextNeed {
                id: "chapter-memo".into(),
                purpose: "Carry the planner's chapter-specific memo into writing and review.".into(),
                sources: vec!["runtime/chapter_memo".into()],
                tier: SkillContextTier::Protected,
                applies_to: vec!["composer".into(), "writer".into(), "auditor".into(), "reviser".into()],
                retrieval: SkillContextRetrieval::Full,
            },
            SkillContextNeed {
                id: "story-frame".into(),
                purpose: "Preserve world rules, core conflict, tone, and non-negotiable canon anchors relevant to the task.".into(),
                sources: vec!["story/outline/story_frame.md".into(), "story/story_bible.md".into()],
                tier: SkillContextTier::Protected,
                applies_to: vec!["planner".into(), "composer".into(), "writer".into(), "auditor".into(), "reviser".into(), "chat".into()],
                retrieval: SkillContextRetrieval::Sections,
            },
            SkillContextNeed {
                id: "volume-map".into(),
                purpose: "Select the current arc/chapter planning section without injecting the entire long outline.".into(),
                sources: vec!["story/outline/volume_map.md".into(), "story/volume_outline.md".into()],
                tier: SkillContextTier::Protected,
                applies_to: vec!["composer".into(), "writer".into(), "auditor".into(), "reviser".into()],
                retrieval: SkillContextRetrieval::Sections,
            },
            SkillContextNeed {
                id: "active-hooks".into(),
                purpose: "Preserve active hook evidence and hook debt that the current chapter must honor.".into(),
                sources: vec!["story/pending_hooks.md".into(), "runtime/hook_debt".into()],
                tier: SkillContextTier::Protected,
                applies_to: vec!["planner".into(), "composer".into(), "writer".into(), "auditor".into(), "reviser".into()],
                retrieval: SkillContextRetrieval::Semantic,
            },
            SkillContextNeed {
                id: "episodic-memory".into(),
                purpose: "Retrieve older chapter summaries, state facts, and volume summaries that are relevant but may be semantically compressed.".into(),
                sources: vec!["story/chapter_summaries.md".into(), "story/current_state.md".into(), "story/volume_summaries.md".into()],
                tier: SkillContextTier::Compressible,
                applies_to: vec!["composer".into(), "writer".into(), "auditor".into(), "reviser".into()],
                retrieval: SkillContextRetrieval::Semantic,
            },
        ],
        body: String::new(),
        source: SkillSource::Builtin,
    }
}

fn open_world_play_skill() -> CapabilitySkillManifest {
    CapabilitySkillManifest {
        id: "open-world-play".into(),
        name: "Open World Play".into(),
        description: "Open-world and branching interactive fiction: world contracts, character agents, time semantics, inventory/evidence/relation state, and scene rendering.".into(),
        when_to_use: "Use for Mnemosyne Play, open worlds, branching interaction, free actions, player persona, world state, character autonomy, and play illustrations.".into(),
        triggers: vec![
            "开放世界".into(), "分支互动".into(), "互动世界".into(), "自由行动".into(),
            "角色agent".into(), "世界契约".into(), "视觉契约".into(),
            "open world".into(), "play".into(), "interactive fiction".into(),
        ],
        session_kinds: vec!["play".into()],
        prompt_packs: vec![
            "play.start".into(), "play.mutator".into(), "play.renderer".into(),
            "play.reconciler".into(), "play.image".into(),
        ],
        tool_hints: vec![
            "play_start".into(), "play_step".into(), "generate_play_image".into(),
        ],
        context_needs: vec![
            SkillContextNeed {
                id: "world-contract".into(),
                purpose: "Bind the world rules, time semantics, taboos, and durable play constraints confirmed by the user.".into(),
                sources: vec!["skills/world-contract.md".into(), "world/contract.md".into(), "state/world.md".into()],
                tier: SkillContextTier::Protected,
                applies_to: vec!["play_start".into(), "play_step".into(), "mutator".into(), "renderer".into()],
                retrieval: SkillContextRetrieval::Full,
            },
            SkillContextNeed {
                id: "player-persona".into(),
                purpose: "Preserve the player's identity, constraints, viewpoint, and current goals.".into(),
                sources: vec!["skills/player-persona.md".into(), "state/player.md".into()],
                tier: SkillContextTier::Protected,
                applies_to: vec!["play_start".into(), "play_step".into(), "mutator".into(), "renderer".into()],
                retrieval: SkillContextRetrieval::Full,
            },
            SkillContextNeed {
                id: "active-play-state".into(),
                purpose: "Carry active scene, character, relationship, inventory, evidence, and time state.".into(),
                sources: vec!["state.md".into(), "scene.md".into(), "graph.db".into(), "graph.json".into()],
                tier: SkillContextTier::Protected,
                applies_to: vec!["play_step".into(), "mutator".into(), "renderer".into(), "reconciler".into()],
                retrieval: SkillContextRetrieval::Semantic,
            },
            SkillContextNeed {
                id: "visual-contract".into(),
                purpose: "Use user-confirmed visual semantics for scene, character, item, and evidence images without inventing game UI frames.".into(),
                sources: vec!["skills/visual-contract.md".into(), "image-settings.json".into(), "images/manifest.json".into()],
                tier: SkillContextTier::Protected,
                applies_to: vec!["image".into(), "renderer".into()],
                retrieval: SkillContextRetrieval::Full,
            },
            SkillContextNeed {
                id: "old-events".into(),
                purpose: "Recall older play events only when relevant; they may be summarized to keep the active scene responsive.".into(),
                sources: vec!["events.jsonl".into(), "transcript.md".into()],
                tier: SkillContextTier::Compressible,
                applies_to: vec!["play_step".into(), "renderer".into()],
                retrieval: SkillContextRetrieval::Semantic,
            },
        ],
        body: String::new(),
        source: SkillSource::Builtin,
    }
}

fn interactive_film_authoring_skill() -> CapabilitySkillManifest {
    CapabilitySkillManifest {
        id: "interactive-film-authoring".into(),
        name: "Interactive Film Authoring".into(),
        description: "Interactive film and game-like drama authoring: story graph, variables, endings, script, storyboard, image prompts, and export packages.".into(),
        when_to_use: "Use for interactive-film projects, branching drama, visual novel style scripts, variables/flags, multiple endings, storyboards, and image asset planning.".into(),
        triggers: vec![
            "互动影游".into(), "互动剧".into(), "分支剧情".into(), "变量旗标".into(),
            "多结局".into(), "剧情图谱".into(), "分镜".into(),
            "interactive film".into(), "branching drama".into(), "story graph".into(), "storyboard".into(),
        ],
        session_kinds: vec!["interactive-film".into(), "interactive-film-authoring".into()],
        prompt_packs: vec![
            "interactive-film.script".into(), "interactive-film.storyboard".into(),
            "interactive-film.story-graph".into(), "interactive-film.image-plan".into(),
        ],
        tool_hints: vec![
            "interactive_film_create".into(), "set_world_anchor".into(), "add_variable".into(),
            "upsert_characters".into(), "fill_node".into(), "revise_node".into(), "generate_node_image".into(),
        ],
        context_needs: vec![
            SkillContextNeed {
                id: "story-graph".into(),
                purpose: "Preserve canonical node topology, choices, variables, endings, and current authoring target.".into(),
                sources: vec!["story-graph.json".into()],
                tier: SkillContextTier::Protected,
                applies_to: vec!["authoring".into(), "script".into(), "storyboard".into(), "image".into()],
                retrieval: SkillContextRetrieval::Full,
            },
            SkillContextNeed {
                id: "current-node".into(),
                purpose: "Focus generation and edits on the current node while respecting graph topology.".into(),
                sources: vec!["story-graph.json#current-node".into()],
                tier: SkillContextTier::Protected,
                applies_to: vec!["fill_node".into(), "revise_node".into(), "image".into()],
                retrieval: SkillContextRetrieval::Sections,
            },
            SkillContextNeed {
                id: "character-voices".into(),
                purpose: "Preserve character motivations and voice profiles across nodes.".into(),
                sources: vec!["skills/character-voices.md".into(), "story-graph.json#characters".into(), "memory.db".into()],
                tier: SkillContextTier::Protected,
                applies_to: vec!["script".into(), "fill_node".into(), "revise_node".into()],
                retrieval: SkillContextRetrieval::Semantic,
            },
            SkillContextNeed {
                id: "visual-style".into(),
                purpose: "Preserve visual style, image prompt conventions, aspect ratio, and asset constraints for node images.".into(),
                sources: vec!["skills/visual-style.md".into(), "image-prompts.md".into(), "assets.json".into()],
                tier: SkillContextTier::Protected,
                applies_to: vec!["image".into(), "storyboard".into()],
                retrieval: SkillContextRetrieval::Full,
            },
            SkillContextNeed {
                id: "previous-node-summaries".into(),
                purpose: "Recall previous branch beats without injecting the entire script into every node-level edit.".into(),
                sources: vec!["script.md".into(), "storyboard.md".into(), "story-tree.md".into()],
                tier: SkillContextTier::Compressible,
                applies_to: vec!["fill_node".into(), "revise_node".into(), "storyboard".into()],
                retrieval: SkillContextRetrieval::Semantic,
            },
        ],
        body: String::new(),
        source: SkillSource::Builtin,
    }
}

// ── 测试 ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_skills_have_three_entries() {
        let skills = builtin_capability_skills();
        assert_eq!(skills.len(), 3, "应有 3 个 builtin capability skills");
        let ids: Vec<&str> = skills.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&"longform-writing"));
        assert!(ids.contains(&"open-world-play"));
        assert!(ids.contains(&"interactive-film-authoring"));
    }

    #[test]
    fn builtin_skills_all_have_context_needs() {
        for skill in builtin_capability_skills() {
            assert!(!skill.context_needs.is_empty(),
                "skill {} 应有 context_needs", skill.id);
            for need in &skill.context_needs {
                assert!(!need.id.is_empty(), "context_need id 不应为空");
                assert!(!need.purpose.is_empty(), "context_need purpose 不应为空");
                assert!(!need.sources.is_empty(), "context_need sources 不应为空");
            }
        }
    }

    #[test]
    fn builtin_skills_all_have_triggers_and_session_kinds() {
        for skill in builtin_capability_skills() {
            assert!(!skill.triggers.is_empty(), "skill {} 应有 triggers", skill.id);
            assert!(!skill.session_kinds.is_empty(), "skill {} 应有 session_kinds", skill.id);
            assert!(!skill.prompt_packs.is_empty(), "skill {} 应有 prompt_packs", skill.id);
            assert!(!skill.tool_hints.is_empty(), "skill {} 应有 tool_hints", skill.id);
        }
    }

    #[test]
    fn builtin_prompt_packs_have_three_entries() {
        let packs = builtin_prompt_packs();
        assert_eq!(packs.len(), 3, "应有 3 个 builtin prompt packs");
        let ids: Vec<&str> = packs.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"longform"));
        assert!(ids.contains(&"play"));
        assert!(ids.contains(&"interactive-film"));
    }

    #[test]
    fn builtin_prompts_have_twelve_entries() {
        let prompts = builtin_prompts();
        assert_eq!(prompts.len(), 12, "应有 12 个 builtin prompts");
        // 验证每个 prompt 的 pack_id 都在 prompt_packs 列表中
        let packs = builtin_prompt_packs();
        let pack_ids: Vec<&str> = packs.iter().map(|p| p.id.as_str()).collect();
        for prompt in &prompts {
            assert!(pack_ids.contains(&prompt.pack_id.as_str()),
                "prompt {} 的 pack_id {} 不在 builtin packs 中", prompt.id, prompt.pack_id);
        }
    }

    #[test]
    fn prompt_pack_prompts_match_builtin_prompts() {
        // 验证每个 pack 声明的 prompts 都在 builtin_prompts 中找到对应条目
        let packs = builtin_prompt_packs();
        let prompts = builtin_prompts();
        let prompt_ids: Vec<&str> = prompts.iter().map(|p| p.id.as_str()).collect();
        for pack in &packs {
            for pid in &pack.prompts {
                assert!(prompt_ids.contains(&pid.as_str()),
                    "pack {} 声明的 prompt {} 在 builtin_prompts 中找不到", pack.id, pid);
            }
        }
    }

    #[test]
    fn builtin_skills_no_inkos_references() {
        // 项目约定:品牌名一律用 Mnemosyne,不允许出现 InkOS
        for skill in builtin_capability_skills() {
            let blob = format!("{} {} {} {}",
                skill.name, skill.description, skill.when_to_use, skill.body);
            assert!(!blob.contains("InkOS") && !blob.contains("inkos"),
                "skill {} 仍含 InkOS 引用", skill.id);
        }
        for prompt in builtin_prompts() {
            assert!(!prompt.content.contains("InkOS") && !prompt.content.contains("inkos"),
                "prompt {} 仍含 InkOS 引用", prompt.id);
        }
    }
}
