// FanficCanonImporter。
//
// 职责：从原作素材文本中提取 5 个 section 的 canonical 信息（world_rules /
// character_profiles / key_events / power_system / writing_style），支持长文本分块编译。
//
// 约束：AgentEngine.prompt_once 只支持单轮对话。多轮分块编译循环
// 逐块调用 prompt_once 并合并结果。

use crate::core::agent::engine::AgentEngine;
use crate::domain::pipeline::types::FanficMode;
use crate::shared::error::AppError;

/// 同人 canonical 导入单段素材超过此字符数时触发分块编译。
const SOURCE_CHUNK_CHARS: usize = 50_000;

/// 同人 canonical 导出结果
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct FanficCanonOutput {
    pub world_rules: String,
    pub character_profiles: String,
    pub key_events: String,
    pub power_system: String,
    pub writing_style: String,
    pub full_document: String,
}

/// 分块编译后的源文本
struct CompiledSource {
    content: String,
    compiled: bool,
}

/// 从文本导入 canonical。
pub async fn import_from_text(
    engine: &AgentEngine,
    source_text: &str,
    source_name: &str,
    fanfic_mode: FanficMode,
) -> Result<FanficCanonOutput, AppError> {
    let source = prepare_source_text(engine, source_text, source_name).await?;

    let mode_label = mode_label(fanfic_mode);
    let system_prompt = build_system_prompt(mode_label, source.compiled);
    let user_message = format!(
        "Below is the source material for the original work \"{}\":\n\n{}",
        source_name, source.content
    );

    // temp=0.3（prompt_once 不支持自定义 temperature，使用引擎默认值）
    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    let mut output = parse_sections(&response);
    output.full_document = build_full_document(&output, source_name, fanfic_mode);
    Ok(output)
}

/// 准备源文本：若超过 SOURCE_CHUNK_CHARS，分块编译为语义资料包。
async fn prepare_source_text(
    engine: &AgentEngine,
    source_text: &str,
    source_name: &str,
) -> Result<CompiledSource, AppError> {
    if source_text.chars().count() <= SOURCE_CHUNK_CHARS {
        return Ok(CompiledSource {
            content: source_text.to_string(),
            compiled: false,
        });
    }

    let chunks = split_into_chunks(source_text, SOURCE_CHUNK_CHARS);
    let total = chunks.len();
    let mut notes: Vec<String> = Vec::with_capacity(total);

    for (index, chunk) in chunks.iter().enumerate() {
        let compiled = compile_chunk(engine, chunk, index, total, source_name).await?;
        let trimmed = compiled.trim();
        if !trimmed.is_empty() {
            notes.push(format!(
                "## Chunk {}/{}\n\n{}",
                index + 1,
                total,
                trimmed
            ));
        }
    }

    let content = format!(
        "# Semantic Resource Pack for \"{}\"\n\nThe content below was produced by the authoring system after reading the complete source material chunk by chunk and compressing it, for use in subsequent canon extraction. It is not a truncated copy of the original text.\n\n{}",
        source_name,
        notes.join("\n\n")
    );

    Ok(CompiledSource {
        content,
        compiled: true,
    })
}

/// 编译单个片段。
/// temp=0.2（prompt_once 使用引擎默认 temperature）。
async fn compile_chunk(
    engine: &AgentEngine,
    chunk: &str,
    index: usize,
    total: usize,
    source_name: &str,
) -> Result<String, AppError> {
    let system_prompt = "You are a fanfic canon resource compiler. Your task is to compress a single chunk of the original work into a Markdown resource pack that subsequent extraction can use.\nDo not continue the story, do not create new content, and do not fill in information that is not present. Keep only the world rules, characters, relationships, key events, power systems, catchphrases, speech styles, and text-backed evidence that actually appear in this chunk.\nIf a given category of information is absent from the chunk, omit that category entirely. Preserve the chunk number so it can be traced later.";
    let user_message = format!(
        "Original work: \"{}\"\nChunk: {}/{}\n\n{}",
        source_name,
        index + 1,
        total,
        chunk
    );
    engine.prompt_once(system_prompt, &user_message).await
}

/// 按字符数分块。
fn split_into_chunks(text: &str, chunk_chars: usize) -> Vec<String> {
    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut count = 0usize;

    for c in text.chars() {
        current.push(c);
        count += 1;
        if count >= chunk_chars {
            chunks.push(std::mem::take(&mut current));
            count = 0;
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

/// 构建主抽取 system prompt。
fn build_system_prompt(mode_label: &str, compiled: bool) -> String {
    let compiled_note = if compiled {
        "\nNote: the source material is long. The input below is a semantic resource pack produced after reading the complete material chunk by chunk — it is not a truncated copy of the original text. Rely on the chunk numbers and evidence inside the resource pack."
    } else {
        ""
    };

    format!(
        r###"<identity>
You are a professional fanfic-creation source analyst. Your task is to extract structured canon information from the source material the user provides, for use by the fanfic authoring system.
</identity>

Fanfic mode: {mode_label}

<responsibilities>
Extract the following five sections from the source material. Each section is delimited by an `=== SECTION: <name> ===` marker. Output every section in order, even if some are sparse.
</responsibilities>

=== SECTION: world_rules ===
World rules (geography, physical laws, magic/power systems, factions and organizations, social structure).
If the source material does not contain explicit world rules, infer them reasonably from the available information.

=== SECTION: character_profiles ===
A character profile table, one row per important character:

| Character | Identity | Personality baseline | Catchphrase / verbal tic | Speech style | Behavioral pattern | Key relationships | Information boundary |
|-----------|----------|----------------------|--------------------------|--------------|--------------------|--------------------|----------------------|

Requirements:
- Catchphrases / verbal tics must be extracted verbatim from the original text, if present.
- Speech style describes the character's tone, word-choice preferences, and sentence patterns.
- Behavioral pattern describes the character's typical reactions in specific situations.
- Information boundary annotates what the character knows and what they do not know.
- Extract at least 3 characters and no more than 15.

=== SECTION: key_events ===
A timeline of key events:

| # | Event | Characters involved | Constraint on fanfic writing |
|---|-------|---------------------|------------------------------|

Arrange in order of time / appearance, and annotate how constraining each event is for fanfic writing.

=== SECTION: power_system ===
The power / ability system (if applicable). Include tier divisions, core rules, and known limitations.
If the original work has no explicit power system, output "(the original work has no explicit power system)".

=== SECTION: writing_style ===
The original work's writing-style traits (for the fanfic writer to imitate):

1. Narrative person and POV (first person / limited third / omniscient; whether it switches frequently).
2. Sentence rhythm (long-short alternation, average paragraph-length feel, dialogue ratio).
3. Scene-description technique (preferred senses, imagery choices, density of environmental description).
4. Dialogue-tag habits (usage of words like "said / replied / laughed"; whether action or expression accompanies the dialogue).
5. Emotional-expression mode (direct interior monologue vs. externalized action vs. environmental projection).
6. Simile / rhetoric tendency (common simile types, rhetoric frequency).
7. Pacing transitions (how tension relaxes into calm; chapter-ending habits).

Back each item with 1-2 verbatim example sentences from the original text. Extract only traits that actually exist in the text — do not describe in vague generalities.

<rules>
- Stay faithful to the source material; never fabricate information that is not in the original.
- When information is insufficient, mark "(not mentioned in the source material)" rather than inventing.
- Character catchphrases are the most important field — fanfic readers care most about whether a character "sounds right."
- Writing-style extraction must be grounded in actual textual traits and backed by example sentences.
</rules>{compiled_note}"###,
        mode_label = mode_label,
        compiled_note = compiled_note,
    )
}

/// 解析 5 个 section。
/// 缺失的 section 默认为空字符串（不报错）。
fn parse_sections(content: &str) -> FanficCanonOutput {
    let sections = [
        "world_rules",
        "character_profiles",
        "key_events",
        "power_system",
        "writing_style",
    ];

    let mut output = FanficCanonOutput::default();
    for section_name in &sections {
        let marker = format!("=== SECTION: {} ===", section_name);
        if let Some(start) = content.find(&marker) {
            let content_start = start + marker.len();
            let section_end = sections
                .iter()
                .filter_map(|s| {
                    let m = format!("=== SECTION: {} ===", s);
                    content[content_start..].find(&m).map(|pos| content_start + pos)
                })
                .min()
                .unwrap_or(content.len());
            let section_content = content[content_start..section_end].trim();
            match *section_name {
                "world_rules" => output.world_rules = section_content.to_string(),
                "character_profiles" => output.character_profiles = section_content.to_string(),
                "key_events" => output.key_events = section_content.to_string(),
                "power_system" => output.power_system = section_content.to_string(),
                "writing_style" => output.writing_style = section_content.to_string(),
                _ => {}
            }
        }
    }
    output
}

/// 构建完整文档（含 YAML meta 头）。
fn build_full_document(
    output: &FanficCanonOutput,
    source_name: &str,
    fanfic_mode: FanficMode,
) -> String {
    let meta = format!(
        "---\nmeta:\n  sourceFile: \"{}\"\n  fanficMode: \"{}\"\n  generatedAt: \"{}\"",
        source_name,
        fanfic_mode_str(fanfic_mode),
        chrono::Utc::now().to_rfc3339()
    );

    let world_rules = if output.world_rules.is_empty() {
        "（素材中未提取到明确世界规则）"
    } else {
        &output.world_rules
    };
    let character_profiles = if output.character_profiles.is_empty() {
        "（素材中未提取到角色信息）"
    } else {
        &output.character_profiles
    };
    let key_events = if output.key_events.is_empty() {
        "（素材中未提取到关键事件）"
    } else {
        &output.key_events
    };
    let power_system = if output.power_system.is_empty() {
        "（原作无明确力量体系）"
    } else {
        &output.power_system
    };
    let writing_style = if output.writing_style.is_empty() {
        "（素材不足以提取风格特征）"
    } else {
        &output.writing_style
    };

    vec![
        format!("# 同人正典（《{}》）", source_name),
        String::new(),
        "## 世界规则".to_string(),
        world_rules.to_string(),
        String::new(),
        "## 角色档案".to_string(),
        character_profiles.to_string(),
        String::new(),
        "## 关键事件时间线".to_string(),
        key_events.to_string(),
        String::new(),
        "## 力量体系".to_string(),
        power_system.to_string(),
        String::new(),
        "## 原作写作风格".to_string(),
        writing_style.to_string(),
        String::new(),
        meta,
    ]
    .join("\n")
}

/// fanfic_mode 对应的英文标签。
fn mode_label(mode: FanficMode) -> &'static str {
    match mode {
        FanficMode::Canon => "Canon-faithful (strictly obey the original work's settings)",
        FanficMode::Au => "AU / parallel world (world rules may change; characters retained)",
        FanficMode::Ooc => "OOC (character personality may deviate from the original)",
        FanficMode::Cp => "CP (centered on a paired relationship)",
    }
}

/// fanfic_mode 序列化字符串。
fn fanfic_mode_str(mode: FanficMode) -> &'static str {
    match mode {
        FanficMode::Canon => "canon",
        FanficMode::Au => "au",
        FanficMode::Ooc => "ooc",
        FanficMode::Cp => "cp",
    }
}

// ── 测试 ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_into_chunks_respects_size() {
        let text = "a".repeat(120_000);
        let chunks = split_into_chunks(&text, SOURCE_CHUNK_CHARS);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].chars().count(), 50_000);
        assert_eq!(chunks[1].chars().count(), 50_000);
        assert_eq!(chunks[2].chars().count(), 20_000);
    }

    #[test]
    fn split_into_chunks_short_text_single_chunk() {
        let chunks = split_into_chunks("short", SOURCE_CHUNK_CHARS);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "short");
    }

    #[test]
    fn split_into_chunks_handles_cjk() {
        let text: String = "你".repeat(50_001);
        let chunks = split_into_chunks(&text, SOURCE_CHUNK_CHARS);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].chars().count(), 50_000);
        assert_eq!(chunks[1].chars().count(), 1);
    }

    #[test]
    fn parse_sections_extracts_all_five() {
        let content = r#"
=== SECTION: world_rules ===
世界规则内容

=== SECTION: character_profiles ===
角色档案内容

=== SECTION: key_events ===
关键事件内容

=== SECTION: power_system ===
力量体系内容

=== SECTION: writing_style ===
写作风格内容
"#;
        let output = parse_sections(content);
        assert_eq!(output.world_rules, "世界规则内容");
        assert_eq!(output.character_profiles, "角色档案内容");
        assert_eq!(output.key_events, "关键事件内容");
        assert_eq!(output.power_system, "力量体系内容");
        assert_eq!(output.writing_style, "写作风格内容");
    }

    #[test]
    fn parse_sections_missing_defaults_empty() {
        let content = "=== SECTION: world_rules ===\n只有世界规则";
        let output = parse_sections(content);
        assert_eq!(output.world_rules, "只有世界规则");
        assert!(output.character_profiles.is_empty());
        assert!(output.key_events.is_empty());
    }

    #[test]
    fn build_full_document_contains_all_sections() {
        let output = FanficCanonOutput {
            world_rules: "世界规则".to_string(),
            character_profiles: "角色档案".to_string(),
            key_events: "关键事件".to_string(),
            power_system: "力量体系".to_string(),
            writing_style: "写作风格".to_string(),
            full_document: String::new(),
        };
        let doc = build_full_document(&output, "测试原作", FanficMode::Canon);
        assert!(doc.contains("# 同人正典（《测试原作》）"));
        assert!(doc.contains("## 世界规则"));
        assert!(doc.contains("世界规则"));
        assert!(doc.contains("## 角色档案"));
        assert!(doc.contains("## 关键事件时间线"));
        assert!(doc.contains("## 力量体系"));
        assert!(doc.contains("## 原作写作风格"));
        assert!(doc.contains("sourceFile: \"测试原作\""));
        assert!(doc.contains("fanficMode: \"canon\""));
    }

    #[test]
    fn build_full_document_uses_fallback_for_empty() {
        let output = FanficCanonOutput::default();
        let doc = build_full_document(&output, "空原作", FanficMode::Au);
        assert!(doc.contains("（素材中未提取到明确世界规则）"));
        assert!(doc.contains("（素材中未提取到角色信息）"));
        assert!(doc.contains("（素材中未提取到关键事件）"));
        assert!(doc.contains("（原作无明确力量体系）"));
        assert!(doc.contains("（素材不足以提取风格特征）"));
        assert!(doc.contains("fanficMode: \"au\""));
    }

    #[test]
    fn mode_label_returns_correct_label() {
        assert_eq!(mode_label(FanficMode::Canon), "Canon-faithful (strictly obey the original work's settings)");
        assert_eq!(mode_label(FanficMode::Au), "AU / parallel world (world rules may change; characters retained)");
        assert_eq!(mode_label(FanficMode::Ooc), "OOC (character personality may deviate from the original)");
        assert_eq!(mode_label(FanficMode::Cp), "CP (centered on a paired relationship)");
    }

    #[test]
    fn fanfic_mode_str_returns_lowercase() {
        assert_eq!(fanfic_mode_str(FanficMode::Canon), "canon");
        assert_eq!(fanfic_mode_str(FanficMode::Au), "au");
        assert_eq!(fanfic_mode_str(FanficMode::Ooc), "ooc");
        assert_eq!(fanfic_mode_str(FanficMode::Cp), "cp");
    }

    #[test]
    fn build_system_prompt_includes_mode_label() {
        let prompt = build_system_prompt("Canon-faithful (strictly obey the original work's settings)", false);
        assert!(prompt.contains("Fanfic mode: Canon-faithful (strictly obey the original work's settings)"));
        assert!(prompt.contains("=== SECTION: world_rules ==="));
        assert!(prompt.contains("=== SECTION: writing_style ==="));
        assert!(!prompt.contains("the source material is long"));
    }

    #[test]
    fn build_system_prompt_includes_compiled_note() {
        let prompt = build_system_prompt("AU / parallel world", true);
        assert!(prompt.contains("the source material is long"));
        assert!(prompt.contains("semantic resource pack"));
    }
}
