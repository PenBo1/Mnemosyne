//! ═══════════════════════════════════════════════════════════════════════════
//! 用户类型 - 数据模型定义
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

/// 用户配置文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    /// 用户名称
    pub name: String,
    /// 语言偏好
    pub language: String,
    /// 写作风格
    pub style: WritingStyle,
    /// 读者类型
    pub reader_type: ReaderType,
    /// 喜好题材
    pub genres: Vec<String>,
    /// 自定义指令
    pub custom_instructions: Vec<String>,
    /// 语调偏好
    pub tone: Option<String>,
    /// 字数偏好
    pub word_count_preference: Option<WordCountPreference>,
}

/// 写作风格配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WritingStyle {
    /// 正式程度（standard/formal/casual）
    pub formality: String,
    /// 节奏（fast/moderate/slow）
    pub pacing: String,
    /// 描写密度（sparse/moderate/dense）
    pub description_density: String,
    /// 对话风格（natural/stylized/minimal）
    pub dialogue_style: String,
}

/// 读者类型分类
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReaderType {
    /// 青少年读者
    YoungAdult,
    /// 一般读者
    General,
    /// 文学读者
    Literary,
    /// 类型小说读者
    Genre,
    /// 网文读者
    WebNovel,
    /// 自定义类型
    Custom(String),
}

impl std::fmt::Display for ReaderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::YoungAdult => write!(f, "young_adult"),
            Self::General => write!(f, "general"),
            Self::Literary => write!(f, "literary"),
            Self::Genre => write!(f, "genre"),
            Self::WebNovel => write!(f, "web_novel"),
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

/// 字数偏好配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordCountPreference {
    /// 最小字数
    pub min_words: u32,
    /// 最大字数
    pub max_words: u32,
    /// 目标字数
    pub target_words: u32,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            name: "Writer".to_string(),
            language: "auto".to_string(),
            style: WritingStyle::default(),
            reader_type: ReaderType::General,
            genres: Vec::new(),
            custom_instructions: Vec::new(),
            tone: None,
            word_count_preference: None,
        }
    }
}

impl UserProfile {
    /// 格式化为 system prompt 片段,供 build_system_prompt 拼装。
    ///
    /// 输出形如:
    /// ```text
    /// ## User Profile
    /// User: ...
    /// Language: ...
    /// Style: formality=..., pacing=..., ...
    /// ...
    /// ```
    pub fn format_for_prompt(&self) -> String {
        let mut sections = Vec::new();
        sections.push(format!("User: {}", self.name));
        sections.push(format!("Language: {}", self.language));
        sections.push(format!(
            "Style: formality={}, pacing={}, descriptions={}, dialogue={}",
            self.style.formality,
            self.style.pacing,
            self.style.description_density,
            self.style.dialogue_style
        ));
        sections.push(format!("Target readers: {}", self.reader_type));
        if !self.genres.is_empty() {
            sections.push(format!("Preferred genres: {}", self.genres.join(", ")));
        }
        if let Some(ref tone) = self.tone {
            sections.push(format!("Tone: {}", tone));
        }
        if let Some(ref wc) = self.word_count_preference {
            sections.push(format!(
                "Word count: {}-{} (target {})",
                wc.min_words, wc.max_words, wc.target_words
            ));
        }
        for inst in &self.custom_instructions {
            sections.push(format!("Instruction: {}", inst));
        }
        format!("## User Profile\n{}\n", sections.join("\n"))
    }
}

impl Default for WritingStyle {
    fn default() -> Self {
        Self {
            formality: "standard".to_string(),
            pacing: "moderate".to_string(),
            description_density: "moderate".to_string(),
            dialogue_style: "natural".to_string(),
        }
    }
}

/// 用户配置错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserError {
    /// 错误信息
    pub message: String,
}