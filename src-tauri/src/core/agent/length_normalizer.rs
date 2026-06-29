use async_trait::async_trait;
use crate::shared::errors::AppError;
use crate::infrastructure::state_store::gc::utils;
use super::base::{AgentContext, BaseAgent};
use super::types::AgentRole;

pub struct LengthNormalizerAgent;

impl Default for LengthNormalizerAgent {
    fn default() -> Self { Self }
}
impl LengthNormalizerAgent {
    pub fn new() -> Self { Self }

    /// Normalize chapter length to target word count
    pub async fn normalize(
        &self,
        ctx: &AgentContext,
        content: &str,
        target_words: u32,
        language: &str,
    ) -> Result<NormalizeOutput, AppError> {
        let current_words = utils::count_words(content, language);
        let spec = LengthSpec::from_chapter_words(target_words);

        match spec.check(current_words) {
            LengthCheck::Ok => Ok(NormalizeOutput {
                content: content.to_string(),
                word_count: current_words,
                applied: false,
            }),
            LengthCheck::TooShort | LengthCheck::TooLong => {
                let system = format!(
                    "You are a text length normalizer. Adjust the following text to be approximately {} words (current: {}). \
                     Maintain the original style and story progression. Do not add filler content.",
                    target_words, current_words
                );
                let user = format!("Adjust this text to {} words:\n\n{}", target_words, content);
                let response = self.chat(ctx, &system, &user).await?;
                let normalized_words = utils::count_words(&response.content, language);
                Ok(NormalizeOutput {
                    content: response.content,
                    word_count: normalized_words,
                    applied: true,
                })
            },
            LengthCheck::OutsideSoft => {
                // Within hard range but outside soft range - accept as is
                Ok(NormalizeOutput {
                    content: content.to_string(),
                    word_count: current_words,
                    applied: false,
                })
            },
        }
    }
}

#[async_trait]
impl BaseAgent for LengthNormalizerAgent {
    fn role(&self) -> AgentRole {
        AgentRole::LengthNormalizer
    }

    fn name(&self) -> &str {
        "length-normalizer"
    }
}

pub struct NormalizeOutput {
    pub content: String,
    pub word_count: u32,
    pub applied: bool,
}

pub struct LengthSpec {
    pub target: u32,
    pub soft_min: u32,
    pub soft_max: u32,
    pub hard_min: u32,
    pub hard_max: u32,
}

impl LengthSpec {
    pub fn from_chapter_words(words: u32) -> Self {
        let variance = (words as f64 * 0.15) as u32;
        let hard_variance = (words as f64 * 0.3) as u32;
        Self {
            target: words,
            soft_min: words.saturating_sub(variance),
            soft_max: words + variance,
            hard_min: words.saturating_sub(hard_variance),
            hard_max: words + hard_variance,
        }
    }

    pub fn check(&self, word_count: u32) -> LengthCheck {
        if word_count < self.hard_min {
            LengthCheck::TooShort
        } else if word_count > self.hard_max {
            LengthCheck::TooLong
        } else if word_count < self.soft_min || word_count > self.soft_max {
            LengthCheck::OutsideSoft
        } else {
            LengthCheck::Ok
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LengthCheck {
    Ok,
    OutsideSoft,
    TooShort,
    TooLong,
}
