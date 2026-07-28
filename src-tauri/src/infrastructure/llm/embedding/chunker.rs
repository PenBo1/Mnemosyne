//! ═══════════════════════════════════════════════════════════════════════════
//! 文本切分 - 长文本分块
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 策略：优先按双换行分段，单段过长再按 max_chars 切分，保留段落边界语义。

const DEFAULT_MAX_CHARS: usize = 800;

/// 将长文本切分为多个块。每块不超过 max_chars(默认 800 字符)。
pub fn split_text(text: &str, max_chars: Option<usize>) -> Vec<String> {
    let max = max_chars.unwrap_or(DEFAULT_MAX_CHARS).max(100);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    for paragraph in trimmed.split("\n\n") {
        let p = paragraph.trim();
        if p.is_empty() {
            continue;
        }
        if p.chars().count() <= max {
            chunks.push(p.to_string());
        } else {
            // 段落过长,按字符切分
            let mut buf = String::new();
            for ch in p.chars() {
                buf.push(ch);
                if buf.chars().count() >= max {
                    chunks.push(std::mem::take(&mut buf));
                }
            }
            if !buf.is_empty() {
                chunks.push(buf);
            }
        }
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_returns_single_chunk() {
        let chunks = split_text("hello world", None);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "hello world");
    }

    #[test]
    fn empty_text_returns_empty() {
        assert!(split_text("   \n\n  ", None).is_empty());
    }

    #[test]
    fn splits_long_paragraph_by_chars() {
        let long = "a".repeat(2000);
        let chunks = split_text(&long, Some(500));
        assert!(chunks.len() >= 4);
        for c in &chunks {
            assert!(c.chars().count() <= 500);
        }
    }

    #[test]
    fn splits_by_paragraph() {
        let text = "para1\n\npara2\n\npara3";
        let chunks = split_text(text, None);
        assert_eq!(chunks.len(), 3);
    }
}
