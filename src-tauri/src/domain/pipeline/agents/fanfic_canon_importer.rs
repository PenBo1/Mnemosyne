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
        "以下是原作《{}》的素材：\n\n{}",
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
                "## 片段 {}/{}\n\n{}",
                index + 1,
                total,
                trimmed
            ));
        }
    }

    let content = format!(
        "# 《{}》语义资料包\n\n以下内容由创作系统逐段读取完整原作素材后压缩生成，用于后续正典抽取。它不是原文截断。\n\n{}",
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
    let system_prompt = "你是同人正典资料编译器。任务是把一个原作片段压成后续抽取可用的 Markdown 资料包。\n不要续写、不要创作、不要补不存在的信息。只保留片段里实际出现的世界规则、人物、关系、关键事件、能力体系、口头禅、说话风格和原文证据。\n如果片段没有某类信息，直接省略该类。保留片段编号，方便后续追溯。";
    let user_message = format!(
        "原作：《{}》\n片段：{}/{}\n\n{}",
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
        "\n注意：原作素材较长。下面输入是逐段读取完整素材后生成的语义资料包，不是截断文本；请以资料包中的片段编号和证据为准。"
    } else {
        ""
    };

    format!(
        r###"你是一个专业的同人创作素材分析师。你的任务是从用户提供的原作素材中提取结构化正典信息，供同人写作系统使用。

同人模式：{mode_label}

你需要从原作素材中提取以下内容，每个部分用 === SECTION: <name> === 分隔：

=== SECTION: world_rules ===
世界规则（地理、物理法则、魔法/力量体系、阵营组织、社会结构）。
如果原作素材不包含明确的世界规则，从已有信息合理推断。

=== SECTION: character_profiles ===
角色档案表格，每个重要角色一行：

| 角色 | 身份 | 性格底色 | 语癖/口头禅 | 说话风格 | 行为模式 | 关键关系 | 信息边界 |
|------|------|----------|-------------|----------|----------|----------|----------|

要求：
- 语癖/口头禅必须从原文中精确提取，如有的话
- 说话风格描述该角色的语气、用词偏好、句式特征
- 行为模式描述该角色在特定情境下的典型反应
- 信息边界标注该角色知道什么、不知道什么
- 至少提取 3 个角色，不超过 15 个

=== SECTION: key_events ===
关键事件时间线：

| 序号 | 事件 | 涉及角色 | 对同人写作的约束 |
|------|------|----------|------------------|

按时间/出现顺序排列，标注每个事件对同人创作的约束程度。

=== SECTION: power_system ===
力量/能力体系（如果适用）。包括等级划分、核心规则、已知限制。
如果原作没有明确的力量体系，输出"（原作无明确力量体系）"。

=== SECTION: writing_style ===
原作写作风格特征（供同人写作模仿）：

1. 叙事人称与视角（第一人称/第三人称有限/全知，是否频繁切换）
2. 句式节奏（长短句交替模式、段落平均长度感受、对话占比）
3. 场景描写手法（五感偏好、意象选择、环境描写密度）
4. 对话标记习惯（说/道/笑道 等用法，对话前后是否有动作/表情补充）
5. 情绪表达方式（直白内心独白 vs 动作外化 vs 环境映射）
6. 比喻/修辞倾向（常用比喻类型、修辞频率）
7. 节奏转换（紧张→舒缓的过渡方式、章节结尾习惯）

每项用1-2个原文例句佐证。只提取原文实际存在的特征，不要泛泛描述。

提取原则：
- 忠实于原作素材，不捏造原作中没有的信息
- 信息不足时标注"（素材未提及）"而非编造
- 角色语癖是最重要的字段——同人读者最在意角色"像不像"
- 写作风格提取必须基于实际文本特征，附原文例句{compiled_note}"###,
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
        FanficMode::Canon => "原作向（严格遵守原作设定）",
        FanficMode::Au => "AU/平行世界（世界规则可改，角色保留）",
        FanficMode::Ooc => "OOC（角色性格可偏离原作）",
        FanficMode::Cp => "CP（以配对关系为核心）",
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
        assert_eq!(mode_label(FanficMode::Canon), "原作向（严格遵守原作设定）");
        assert_eq!(mode_label(FanficMode::Au), "AU/平行世界（世界规则可改，角色保留）");
        assert_eq!(mode_label(FanficMode::Ooc), "OOC（角色性格可偏离原作）");
        assert_eq!(mode_label(FanficMode::Cp), "CP（以配对关系为核心）");
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
        let prompt = build_system_prompt("原作向（严格遵守原作设定）", false);
        assert!(prompt.contains("同人模式：原作向（严格遵守原作设定）"));
        assert!(prompt.contains("=== SECTION: world_rules ==="));
        assert!(prompt.contains("=== SECTION: writing_style ==="));
        assert!(!prompt.contains("注意：原作素材较长"));
    }

    #[test]
    fn build_system_prompt_includes_compiled_note() {
        let prompt = build_system_prompt("AU/平行世界", true);
        assert!(prompt.contains("注意：原作素材较长"));
        assert!(prompt.contains("语义资料包"));
    }
}
