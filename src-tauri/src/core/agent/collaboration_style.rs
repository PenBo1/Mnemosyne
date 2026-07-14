// Collaboration Style —— 协作风格系统。
//
// 四种风格:
// - Efficient:高效极简 —— 简洁直接,不加修饰,聚焦解决问题
// - Thoughtful:深思熟虑 —— 充分分析,权衡取舍,考虑边界情况
// - Patient:温和耐心 —— 循序渐进,解释原理,适合学习场景
// - Decisive:果断执行 —— 行动导向,快速决策,少问多做
//
// 注入方式:
// - 风格对应的 prompt 片段会作为 "Additional Instructions" 的一部分
// - 追加到 custom_instructions 之后(不覆盖用户自定义指令)
// - 与 Effort 正交:Effort 控制"做多少",Style 控制"怎么做"

use serde::{Deserialize, Serialize};

/// 协作风格(四档)
///
/// 用户可通过 UI 选择,也可在每次 ChatRequest 中覆盖。
/// 默认 Efficient(高效极简)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CollaborationStyle {
    /// 高效极简:简洁直接,聚焦解决问题
    Efficient,
    /// 深思熟虑:充分分析,权衡取舍
    Thoughtful,
    /// 温和耐心:循序渐进,解释原理
    Patient,
    /// 果断执行:行动导向,快速决策
    Decisive,
}

impl CollaborationStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Efficient => "efficient",
            Self::Thoughtful => "thoughtful",
            Self::Patient => "patient",
            Self::Decisive => "decisive",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s.to_lowercase().as_str() {
            "efficient" | "minimal" => Self::Efficient,
            "thoughtful" => Self::Thoughtful,
            "patient" => Self::Patient,
            "decisive" => Self::Decisive,
            _ => return None,
        })
    }

    /// 获取该风格对应的 system prompt 片段。
    ///
    /// 这段文字会作为 "Additional Instructions" 的一部分注入到 system prompt,
    /// 影响 Agent 的回复风格和行为倾向。
    pub fn prompt_fragment(&self) -> &'static str {
        match self {
            Self::Efficient => "\
# 协作风格:高效极简\n\
- 回复简洁直接,不添加不必要的解释和修饰\n\
- 聚焦解决问题,先给出方案再补充细节\n\
- 优先使用代码/命令而非自然语言描述\n\
- 如无歧义,直接执行而不过度确认\n\
- 避免重复用户已说明的信息",
            Self::Thoughtful => "\
# 协作风格:深思熟虑\n\
- 充分分析问题,考虑多种可能性和边界情况\n\
- 权衡取舍时明确列出 pros/cons\n\
- 对不确定的地方主动提出疑问\n\
- 提供多个方案并解释推荐理由\n\
- 预判潜在风险和副作用",
            Self::Patient => "\
# 协作风格:温和耐心\n\
- 循序渐进,从基础概念开始解释\n\
- 对复杂操作分步骤说明,每步标注目的\n\
- 主动解释\"为什么这样做\"而非仅给出\"做什么\"\n\
- 用类比和示例帮助理解抽象概念\n\
- 鼓励提问,不催促用户",
            Self::Decisive => "\
# 协作风格:果断执行\n\
- 行动导向:先做再说,边做边调整\n\
- 遇到选择时快速决策,说明理由但不纠结\n\
- 优先完成核心目标,细节后续优化\n\
- 减少不必要的确认,除非涉及破坏性操作\n\
- 失败时立即给出补救方案而非停下",
        }
    }
}

impl Default for CollaborationStyle {
    fn default() -> Self {
        Self::Efficient
    }
}

impl std::fmt::Display for CollaborationStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 将协作风格 prompt 片段合并到 custom_instructions。
///
/// 规则:
/// - 若 style 为 None,返回原 custom_instructions(不变)
/// - 若 custom_instructions 为空,返回 style 的 prompt 片段
/// - 若两者都有,用换行符连接(custom 在前,style 在后)
///
/// 这样用户自定义指令优先,风格作为补充约束。
pub fn merge_style_into_instructions(
    custom_instructions: Option<&str>,
    style: Option<CollaborationStyle>,
) -> Option<String> {
    let style_fragment = style.map(|s| s.prompt_fragment());
    let custom = custom_instructions
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    match (custom, style_fragment) {
        (None, None) => None,
        (None, Some(frag)) => Some(frag.to_string()),
        (Some(c), None) => Some(c.to_string()),
        (Some(c), Some(frag)) => Some(format!("{c}\n\n{frag}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_str_roundtrip() {
        for &s in &[
            CollaborationStyle::Efficient,
            CollaborationStyle::Thoughtful,
            CollaborationStyle::Patient,
            CollaborationStyle::Decisive,
        ] {
            assert_eq!(CollaborationStyle::from_str(s.as_str()), Some(s));
        }
        assert_eq!(CollaborationStyle::from_str("minimal"), Some(CollaborationStyle::Efficient));
        assert_eq!(CollaborationStyle::from_str("unknown"), None);
    }

    #[test]
    fn default_is_efficient() {
        assert_eq!(CollaborationStyle::default(), CollaborationStyle::Efficient);
    }

    #[test]
    fn prompt_fragment_nonempty() {
        for &s in &[
            CollaborationStyle::Efficient,
            CollaborationStyle::Thoughtful,
            CollaborationStyle::Patient,
            CollaborationStyle::Decisive,
        ] {
            assert!(!s.prompt_fragment().is_empty());
            assert!(s.prompt_fragment().contains("协作风格"));
        }
    }

    #[test]
    fn merge_none_style_returns_original() {
        let result = merge_style_into_instructions(Some("do X"), None);
        assert_eq!(result.as_deref(), Some("do X"));
    }

    #[test]
    fn merge_none_custom_returns_style_fragment() {
        let result = merge_style_into_instructions(None, Some(CollaborationStyle::Efficient));
        assert!(result.is_some());
        assert!(result.unwrap().contains("高效极简"));
    }

    #[test]
    fn merge_both_concatenates() {
        let result =
            merge_style_into_instructions(Some("custom task"), Some(CollaborationStyle::Patient));
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.contains("custom task"));
        assert!(r.contains("温和耐心"));
        // custom 在前,style 在后
        assert!(r.find("custom task") < r.find("温和耐心"));
    }

    #[test]
    fn merge_empty_custom_returns_style_only() {
        let result =
            merge_style_into_instructions(Some("   "), Some(CollaborationStyle::Decisive));
        assert!(result.is_some());
        assert!(result.unwrap().contains("果断执行"));
    }
}
