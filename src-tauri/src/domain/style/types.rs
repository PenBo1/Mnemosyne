//! ═══════════════════════════════════════════════════════════════════════════
//! 风格类型 - 数据模型定义
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 所有结构使用 camelCase 序列化。

/// 段落长度范围。
#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphRange {
    pub min: usize,
    pub max: usize,
}

/// 风格指纹画像。
#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StyleProfile {
    /// 平均句长(中文=字符数,英文=单词数)
    pub avg_sentence_length: f64,
    /// 句长标准差
    pub sentence_length_std_dev: f64,
    /// 平均段落长度
    pub avg_paragraph_length: f64,
    /// 段落长度范围
    pub paragraph_length_range: ParagraphRange,
    /// 词汇多样性(TTR, Type-Token Ratio, 0-1)
    pub vocabulary_diversity: f64,
    /// 高频句首模式(前 5)
    pub top_patterns: Vec<String>,
    /// 修辞特征(出现 ≥2 次的模式)
    pub rhetorical_features: Vec<String>,
    /// 来源名(可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_name: Option<String>,
    /// 分析时间(ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analyzed_at: Option<String>,
}

/// 风格分析请求(前端 camelCase)。
#[derive(serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StyleAnalyzeInput {
    pub text: String,
    /// 来源名(可选,用于标识参考文本)
    #[serde(default)]
    pub source_name: Option<String>,
    /// zh / en,缺省为 zh
    #[serde(default)]
    pub language: Option<String>,
}
