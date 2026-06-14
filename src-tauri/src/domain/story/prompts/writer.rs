use super::types::{BookRules, GenreConfig, Language, LengthSpec, WriterMode};

pub struct WriterPromptBuilder {
    language: Language,
    genre: GenreConfig,
    book_rules: BookRules,
    chapter_number: Option<u32>,
    mode: WriterMode,
    length_spec: LengthSpec,
}

impl WriterPromptBuilder {
    pub fn new(language: Language, genre: GenreConfig, chapter_words: u32) -> Self {
        let length_spec = LengthSpec::from_chapter_words(chapter_words, &language);
        Self {
            language,
            genre,
            book_rules: BookRules::default(),
            chapter_number: None,
            mode: WriterMode::Full,
            length_spec,
        }
    }

    pub fn with_book_rules(mut self, rules: BookRules) -> Self {
        self.book_rules = rules;
        self
    }

    pub fn with_chapter_number(mut self, chapter: u32) -> Self {
        self.chapter_number = Some(chapter);
        self
    }

    pub fn with_mode(mut self, mode: WriterMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn build(&self) -> String {
        match self.language {
            Language::Zh => super::writer_zh::build_chinese(
                &self.genre.name,
                &self.book_rules,
                self.chapter_number,
                &self.mode,
                &self.length_spec,
            ),
            Language::En => super::writer_en::build_english(
                &self.genre.name,
                &self.book_rules,
                self.chapter_number,
                &self.mode,
                &self.length_spec,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_writer_prompt_builder() {
        let genre = GenreConfig::default();
        let builder = WriterPromptBuilder::new(Language::Zh, genre, 3000)
            .with_chapter_number(1);
        let prompt = builder.build();
        assert!(prompt.contains("核心规则"));
        assert!(prompt.contains("黄金三章"));
    }
}
