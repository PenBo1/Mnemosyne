//! ═══════════════════════════════════════════════════════════════════════════
//! FanficCanonImporter - 同人设定导入代理
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：从原作素材文本中提取 5 个 section 的 canonical 信息（world_rules /
//! character_profiles / key_events / power_system / writing_style），支持长文本分块编译。
//!
//! 约束：AgentEngine.prompt_once 只支持单轮对话。多轮分块编译循环
//! 逐块调用 prompt_once 并合并结果。

use std::time::Instant;

use crate::core::agent::engine::AgentEngine;
use crate::domain::pipeline::types::FanficMode;
use crate::shared::error::AppError;
use futures_util::future::try_join_all;

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
    let start = Instant::now();
    tracing::info!(function = "import_from_text", source_name, mode = ?fanfic_mode, source_chars = source_text.chars().count(), "入口");

    match prepare_source_text(engine, source_text, source_name).await {
        Ok(source) => {
            let mode_label = mode_label(fanfic_mode);
            let system_prompt = build_system_prompt(mode_label, source.compiled);
            let user_message = format!(
                "以下是原作《{}》的素材文本：\n\n{}",
                source_name, source.content
            );

            match engine.prompt_once(&system_prompt, &user_message).await {
                Ok(response) => {
                    let mut output = parse_sections(&response);
                    output.full_document = build_full_document(&output, source_name, fanfic_mode);
                    let duration_ms = start.elapsed().as_millis() as u64;
                    tracing::info!(function = "import_from_text", source_name, duration_ms, compiled = source.compiled, "出口");
                    Ok(output)
                }
                Err(e) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    tracing::error!(function = "import_from_text", source_name, duration_ms, error = %e, "错误");
                    Err(e)
                }
            }
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::error!(function = "import_from_text", source_name, duration_ms, error = %e, "错误");
            Err(e)
        }
    }
}

/// 准备源文本：若超过 SOURCE_CHUNK_CHARS，分块编译为语义资料包。
async fn prepare_source_text(
    engine: &AgentEngine,
    source_text: &str,
    source_name: &str,
) -> Result<CompiledSource, AppError> {
    let start = Instant::now();
    tracing::info!(function = "prepare_source_text", source_name, source_chars = source_text.chars().count(), "入口");

    if source_text.chars().count() <= SOURCE_CHUNK_CHARS {
        tracing::info!(function = "prepare_source_text", source_name, duration_ms = 0u64, compiled = false, reason = "under_threshold", "出口");
        return Ok(CompiledSource {
            content: source_text.to_string(),
            compiled: false,
        });
    }

    let chunks = split_into_chunks(source_text, SOURCE_CHUNK_CHARS);
    let total = chunks.len();

    tracing::info!(function = "prepare_source_text", source_name, total_chunks = total, "开始分块编译");

    let futures = chunks
        .iter()
        .enumerate()
        .map(|(index, chunk)| compile_chunk(engine, chunk, index, total, source_name));
    match try_join_all(futures).await {
        Ok(compiled_results) => {
            let mut notes: Vec<String> = Vec::with_capacity(total);
            for (index, compiled) in compiled_results.iter().enumerate() {
                let trimmed = compiled.trim();
                if !trimmed.is_empty() {
                    notes.push(format!(
                        "## 片段 {}/{}\n\n{}",
                        index + 1,
                        total,
                        trimmed
                    ));
                }
            }

            let content = format!(
                "# 《{}》语义资料包\n\n以下内容由创作系统逐片段读完原作素材后压缩生成，供后续正典抽取使用。它不是原作文本的截断副本。\n\n{}",
                source_name,
                notes.join("\n\n")
            );

            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::info!(function = "prepare_source_text", source_name, duration_ms, compiled = true, total_chunks = total, "出口");

            Ok(CompiledSource {
                content,
                compiled: true,
            })
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::error!(function = "prepare_source_text", source_name, duration_ms, error = %e, "错误");
            Err(e)
        }
    }
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
    let start = Instant::now();
    tracing::info!(function = "compile_chunk", source_name, chunk_index = index + 1, total_chunks = total, "入口");

    let system_prompt = "你是同人正典资料编译器。你的任务是把原作的一个片段压缩成 Markdown 资料包，供后续抽取使用。\n不要续写故事，不要创作新内容，不要补足未出现的信息。只保留本片段中实际出现的世界规则、角色、关系、关键事件、力量体系、口头禅、说话风格、以及有原文支撑的证据。\n若某一类信息在本片段中缺失，整类省略。保留片段编号以便后续追溯。\n\n<safety>\n- NEVER 续写故事或创作新内容；只做压缩，不做加法。\n- NEVER 补足未在原文中出现的细节或推断。\n- NEVER 遗漏片段编号；后续追溯依赖它。\n</safety>";
    let user_message = format!(
        "原作：《{}》\n片段：{}/{}\n\n{}",
        source_name,
        index + 1,
        total,
        chunk
    );
    match engine.prompt_once(system_prompt, &user_message).await {
        Ok(response) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::info!(function = "compile_chunk", source_name, chunk_index = index + 1, duration_ms, "出口");
            Ok(response)
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::error!(function = "compile_chunk", source_name, chunk_index = index + 1, duration_ms, error = %e, "错误");
            Err(e)
        }
    }
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
        "\n注意：原作素材较长。下方输入是逐片段读完完整素材后生成的语义资料包——并非原作文本的截断副本。请依据资料包内的片段编号与证据进行抽取。"
    } else {
        ""
    };

    format!(
        r###"<identity>
你是一位专业的同人创作素材分析师。你的任务是从用户提供的原作素材中抽取结构化正典信息，供同人创作系统使用。
</identity>

同人模式：{mode_label}

<responsibilities>
从原作素材中抽取以下 5 个 section。每个 section 以 `=== SECTION: <name> ===` 标记分隔。必须按顺序输出每个 section，即便某些 section 内容稀疏也不得省略。
</responsibilities>

=== SECTION: world_rules ===
世界规则（地理、物理法则、魔法/力量体系、派系与组织、社会结构）。
若素材中未含明确世界规则，可基于已有信息合理推断。

=== SECTION: character_profiles ===
角色档案表，每个重要角色一行：

| Character | Identity | Personality baseline | Catchphrase / verbal tic | Speech style | Behavioral pattern | Key relationships | Information boundary |
|-----------|----------|----------------------|--------------------------|--------------|--------------------|--------------------|----------------------|

要求：
- 口头禅 / 口癖必须从原文逐字摘录（若原文存在）。
- 说话风格描述角色的语气、用词偏好与句式习惯。
- 行为模式描述角色在特定情境下的典型反应。
- 信息边界标注角色知道什么、不知道什么。
- 至少抽取 3 个角色，最多 15 个。

=== SECTION: key_events ===
关键事件时间线：

| # | Event | Characters involved | Constraint on fanfic writing |
|---|-------|---------------------|------------------------------|

按时间 / 出场顺序排列，并标注每个事件对同人写作的约束强度。

=== SECTION: power_system ===
力量 / 能力体系（若适用）。包含等级划分、核心规则、已知限制。
若原作无明确力量体系，输出"（原作无明确力量体系）"。

=== SECTION: writing_style ===
原作写作风格特征（供同人写手模仿）：

1. 叙事人称与视角（第一人称 / 有限第三人称 / 全知视角；是否频繁切换）。
2. 句式节奏（长短句交替、平均段落长度感、对白占比）。
3. 场景描写技法（偏好的感官、意象选择、环境描写密度）。
4. 对白提示语习惯（"说 / 答 / 笑"等词的使用；对白是否伴随动作或表情）。
5. 情感表达模式（直接内心独白 vs. 外化行动 vs. 环境投射）。
6. 比喻 / 修辞倾向（常见比喻类型、修辞频率）。
7. 节奏过渡（紧张如何放松到平静；章末习惯）。

每条用 1-2 句原文逐字例句支撑。只抽取文本中实际存在的特征——不要用空泛概括。

<rules>
- 忠于原作素材；永不编造原作中不存在的信息。
- 信息不足时，标注"（素材中未提及）"，而非虚构。
- 角色口头禅是最重要的字段——同人读者最在意角色"听起来像不像"。
- 写作风格抽取必须基于实际文本特征，并由例句支撑。
</rules>

<safety>
- NEVER 编造原作中不存在的信息；信息不足时标注"（素材中未提及）"。
- NEVER 用空泛概括替代有原文例句支撑的风格特征。
- NEVER 遗漏 5 个 SECTION 中的任何一个；即便内容稀疏也要按顺序输出。
- NEVER 续写故事或创作新内容；这是抽取任务，不是创作任务。
</safety>

<verification>
在交付前自检：
1. 5 个 `=== SECTION: xxx ===` 块是否按顺序齐全？
2. 角色口头禅是否从原文逐字摘录（而非改写）？
3. 写作风格每条是否都有 1-2 句原文例句支撑？
4. 缺失信息是否标注"（素材中未提及）"而非虚构？
</verification>{compiled_note}"###,
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

/// fanfic_mode 对应的中文标签。
fn mode_label(mode: FanficMode) -> &'static str {
    match mode {
        FanficMode::Canon => "正典向（严格遵守原作设定）",
        FanficMode::Au => "AU / 平行世界（世界规则可变；角色保留）",
        FanficMode::Ooc => "OOC（角色性格可偏离原作）",
        FanficMode::Cp => "CP（以配对关系为中心）",
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
        assert_eq!(mode_label(FanficMode::Canon), "正典向（严格遵守原作设定）");
        assert_eq!(mode_label(FanficMode::Au), "AU / 平行世界（世界规则可变；角色保留）");
        assert_eq!(mode_label(FanficMode::Ooc), "OOC（角色性格可偏离原作）");
        assert_eq!(mode_label(FanficMode::Cp), "CP（以配对关系为中心）");
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
        let prompt = build_system_prompt("正典向（严格遵守原作设定）", false);
        assert!(prompt.contains("同人模式：正典向（严格遵守原作设定）"));
        assert!(prompt.contains("=== SECTION: world_rules ==="));
        assert!(prompt.contains("=== SECTION: writing_style ==="));
        assert!(!prompt.contains("原作素材较长"));
    }

    #[test]
    fn build_system_prompt_includes_compiled_note() {
        let prompt = build_system_prompt("AU / 平行世界", true);
        assert!(prompt.contains("原作素材较长"));
        assert!(prompt.contains("语义资料包"));
    }
}
