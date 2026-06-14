use super::types::{BookRules, LengthSpec, WriterMode};

pub fn build_english(
    genre_name: &str,
    _book_rules: &BookRules,
    chapter_number: Option<u32>,
    _mode: &WriterMode,
    length_spec: &LengthSpec,
) -> String {
    let mut sections = Vec::new();

    sections.push(format!(
        "You are a professional {} web fiction author.",
        genre_name
    ));

    sections.push(build_english_core_rules());

    sections.push(format!(
        "## Length Guidance\n\n- Target: {} words\n- Range: {}-{} words",
        length_spec.target, length_spec.soft_min, length_spec.soft_max
    ));

    sections.push(build_english_craft_card());
    sections.push(build_english_anti_ai_rules());

    if let Some(chapter) = chapter_number {
        if chapter <= 5 {
            sections.push(build_english_golden_chapter(chapter));
        }
    }

    sections.push(build_english_output_format(length_spec));

    sections.join("\n\n")
}

fn build_english_core_rules() -> String {
    r#"## Universal Writing Rules

### Character Rules
1. **Consistency**: Behavior driven by "past experience + current interests + core personality."
2. **Dimensionality**: Core trait + contrasting detail = real person.
3. **No puppets**: Side characters must have independent motivation and agency.
4. **Voice distinction**: Different characters must speak differently.
5. **Relationship logic**: Any relationship change must be set up by events.

### Narrative Technique
6. **Show, don't tell**: Convey through action and sensory detail.
7. **Sensory grounding**: Each scene includes 1-2 sensory details beyond the visual.
8. **Chapter hooks**: Every chapter ending needs a hook.
9. **Information layering**: Worldbuilding emerges through action.
10. **Dialogue-driven**: Deliver conflict and information through dialogue first.

### Beat Density & Rhythm
- **A payoff beat roughly every ~200 words**
- **A forward hook roughly every ~350 words**
- **A full setup → tension → unresolved arc every ~700-1000 words**

### Chapter Cut (80/20 cliffhanger)
- **Never finish the chapter's story inside the chapter.** Write to ~80%; leave the last ~20% for the next chapter."#
        .to_string()
}

fn build_english_craft_card() -> String {
    r#"## Writing Craft Rules

- **Emotion**: Externalize through action
- **Salt in soup**: Values conveyed through behavior, not slogans
- **Supporting cast**: Every side character has their own agenda
- **Five senses**: Include sensory details
- **Concrete**: Don't write "a big city" — write specifics
- **Sentence craft**: Avoid "although...however"
- **Desire engine**: Create emotional gaps → reader anticipates release → release MUST exceed expectations
- **Character check**: Before every character action ask: Why? Does it match their profile?
- **Dialogue**: Different characters speak differently
- **Forbidden**: Info-dump / introducing 3+ new characters at once / "everyone gasped in unison""#
        .to_string()
}

fn build_english_anti_ai_rules() -> String {
    r#"## Anti-AI Iron Laws

**[IRON LAW 1]** The narrator never tells the reader what to conclude.
**[IRON LAW 2]** No analytical/report language in prose.
**[IRON LAW 3]** AI-tell words are rate-limited (max 1 per 3,000 words).
**[IRON LAW 4]** No repetitive image cycling.
**[IRON LAW 5]** Planning terms never appear in chapter text.
**[IRON LAW 6]** Ban the "Not X; Y" construction. Max once per chapter.
**[IRON LAW 7]** Ban lists of three in descriptive prose. Max once per 2,000 words."#
        .to_string()
}

fn build_english_golden_chapter(chapter: u32) -> String {
    match chapter {
        1 => r#"## Golden Opening — Chapter 1

- Open with action or dialogue — no worldbuilding preamble
- **The last sentence of the first 300 words must be a dramatic reversal / striking beat**
- Max 1-2 locations; max 2 named characters who actually clash
- Core conflict must surface before chapter end"#
            .to_string(),
        2 => r#"## Golden Opening — Chapter 2

- The protagonist's unique advantage must appear
- Show it through a concrete event, not internal monologue
- First small payoff/satisfaction beat should land here
- Tighten the core conflict, don't open new subplots"#
            .to_string(),
        3 => r#"## Golden Opening — Chapter 3

- A specific, measurable goal must be established
- Reader must be able to say "I know what the protagonist wants next"
- End with a strong hook"#
            .to_string(),
        4 => r#"## Golden Opening — Chapter 4

- Deliver the first BIG satisfaction beat
- Protagonist uses their edge to achieve something meaningful
- Raise the emotional stakes"#
            .to_string(),
        5 => r#"## Golden Opening — Chapter 5

- New threat or complication that makes the goal harder
- The world expands: reader sees there's a bigger game
- End on the strongest cliffhanger yet — this is the conversion chapter"#
            .to_string(),
        _ => String::new(),
    }
}

fn build_english_output_format(length_spec: &LengthSpec) -> String {
    format!(
        r#"## Output Format (follow strictly)

=== PRE_WRITE_CHECK ===
(Output a Markdown table)

=== CHAPTER_TITLE ===
(Chapter title, without "Chapter X")

=== CHAPTER_CONTENT ===
(Chapter prose. Target {target} words, acceptable range {soft_min}-{soft_max} words.)"#,
        target = length_spec.target,
        soft_min = length_spec.soft_min,
        soft_max = length_spec.soft_max
    )
}
