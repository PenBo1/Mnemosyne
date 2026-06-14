use super::types::FanficMode;

/// 同人正典参照
pub fn build_fanfic_canon_section(fanfic_canon: &str, mode: &FanficMode) -> String {
    let preamble = match mode {
        FanficMode::Canon => r#"你正在写**原作向同人**。严格遵守正典：
- 角色的语癖、说话风格、行为模式必须与原作一致
- 世界规则不可违反
- 关键事件时间线不可矛盾"#,
        FanficMode::Au => r#"你正在写**AU（平行世界）同人**：
- 世界规则可以改变
- 角色的核心性格和说话方式应保持辨识度
- AU 设定偏离必须内部一致"#,
        FanficMode::Ooc => r#"你正在写**OOC 同人**：
- 角色在极端情境下可以偏离性格底色
- 但偏离必须有情境驱动
- 保留角色的语癖和说话特征"#,
        FanficMode::Cp => r#"你正在写**CP 同人**，以角色互动和关系发展为核心：
- 配对双方每章必须有有效互动
- 互动风格要有化学反应
- 关系发展应有节奏感"#,
    };

    format!(
        r#"
## 同人正典参照

{}

以下是原作正典信息，写作时必须参照：

{}"#,
        preamble, fanfic_canon
    )
}
