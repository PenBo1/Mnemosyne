//! ═══════════════════════════════════════════════════════════════════════════
//! Scrubber - StreamingContextScrubber 跨 chunk 边界的 memory-context span 解析器
//! ═══════════════════════════════════════════════════════════════════════════

const OPEN_TAG: &str = "<memory-context>";
const CLOSE_TAG: &str = "</memory-context>";

// ── MemoryContextScrubber ──────────────────────────────────────────────────

/// memory-context span 的流式 scrubber
pub struct MemoryContextScrubber {
    /// 待处理缓冲（可能含跨 chunk 的标签片段）
    pending: String,
    /// 是否在 memory-context 块内
    in_block: bool,
}

impl Default for MemoryContextScrubber {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryContextScrubber {
    pub fn new() -> Self {
        Self {
            pending: String::new(),
            in_block: false,
        }
    }

    /// 输入一个 chunk，返回此 chunk 产生的可见输出（可能为空）
    ///
    /// 若返回 None，表示此 chunk 未产生可见输出（内容被缓冲或被剥离）。
    pub fn feed(&mut self, delta: &str) -> Option<String> {
        self.pending.push_str(delta);
        self.process_buffer()
    }

    /// 流结束时调用，返回缓冲区中的剩余可见内容
    ///
    /// 若 scrubber 仍处于 in_block 状态，说明模型输出了未闭合的 memory-context 块，
    /// 此时丢弃块内容（不输出），并记录警告。
    pub fn flush(&mut self) -> Option<String> {
        if self.in_block {
            tracing::warn!(
                pending_len = self.pending.len(),
                "[memory-scrubber] flushing unclosed <memory-context> block, content discarded"
            );
            self.in_block = false;
            self.pending.clear();
            return None;
        }

        if self.pending.is_empty() {
            None
        } else {
            let remaining = std::mem::take(&mut self.pending);
            Some(remaining)
        }
    }

    /// 当前是否在 memory-context 块内
    pub fn is_in_block(&self) -> bool {
        self.in_block
    }

    fn process_buffer(&mut self) -> Option<String> {
        let mut visible_output = String::new();

        loop {
            if self.pending.is_empty() {
                break;
            }

            if self.in_block {
                // 在块内，查找结束标签
                match self.pending.find(CLOSE_TAG) {
                    Some(pos) => {
                        // 找到结束标签，丢弃块内容 + 标签
                        let after = pos + CLOSE_TAG.len();
                        self.pending = self.pending[after..].to_string();
                        self.in_block = false;
                    }
                    None => {
                        // 未找到结束标签，可能是标签跨 chunk
                        // 从最后一个 `<` 处分割：之前的是块内容（丢弃），之后的是可能的标签前缀（保留）
                        let split = self.last_tag_start();
                        if let Some(idx) = split {
                            // 丢弃 [0..idx]，保留 [idx..]
                            self.pending = self.pending[idx..].to_string();
                        } else {
                            // 无 `<`，全部是块内容，丢弃
                            self.pending.clear();
                        }
                        break;
                    }
                }
            } else {
                // 在块外，查找开始标签
                match self.pending.find(OPEN_TAG) {
                    Some(pos) => {
                        // 找到开始标签，输出标签前的内容
                        visible_output.push_str(&self.pending[..pos]);
                        let after = pos + OPEN_TAG.len();
                        self.pending = self.pending[after..].to_string();
                        self.in_block = true;
                    }
                    None => {
                        // 未找到开始标签，可能是标签跨 chunk
                        // 从最后一个 `<` 处分割：之前的是安全输出，之后的是可能的标签前缀（保留）
                        let split = self.last_tag_start();
                        if let Some(idx) = split {
                            visible_output.push_str(&self.pending[..idx]);
                            self.pending = self.pending[idx..].to_string();
                        } else {
                            // 无 `<`，全部安全输出
                            visible_output.push_str(&self.pending);
                            self.pending.clear();
                        }
                        break;
                    }
                }
            }
        }

        if visible_output.is_empty() {
            None
        } else {
            Some(visible_output)
        }
    }

    /// 找到 pending 中最后一个 `<` 的字节位置
    ///
    /// `<` 是单字节 ASCII，`rfind('<')` 返回字节位置，可直接用于切片。
    /// 用于处理标签跨 chunk 边界的情况：从最后一个 `<` 开始可能是标签开头。
    fn last_tag_start(&self) -> Option<usize> {
        self.pending.rfind('<')
    }
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // 测试 1：无 memory-context 标签，原样输出
    #[test]
    fn no_tags_passes_through() {
        let mut scrubber = MemoryContextScrubber::new();
        assert_eq!(scrubber.feed("Hello world").as_deref(), Some("Hello world"));
    }

    // 测试 2：完整块在一个 chunk 内，被剥离
    #[test]
    fn complete_block_in_one_chunk_stripped() {
        let mut scrubber = MemoryContextScrubber::new();
        let input = "before<memory-context>secret data</memory-context>after";
        let output = scrubber.feed(input);
        assert_eq!(output.as_deref(), Some("beforeafter"));
    }

    // 测试 3：多个块
    #[test]
    fn multiple_blocks_stripped() {
        let mut scrubber = MemoryContextScrubber::new();
        let input = "a<memory-context>b</memory-context>c<memory-context>d</memory-context>e";
        let output = scrubber.feed(input);
        assert_eq!(output.as_deref(), Some("ace"));
    }

    // 测试 4：开始标签跨 chunk 边界
    #[test]
    fn open_tag_split_across_chunks() {
        let mut scrubber = MemoryContextScrubber::new();

        // 第一 chunk：包含开始标签的前半部分
        let out1 = scrubber.feed("before<memory-con");
        // 应输出 "before"，保留 "<memory-con" 作为缓冲
        assert_eq!(out1.as_deref(), Some("before"));

        // 第二 chunk：补全开始标签 + 块内容 + 结束标签
        let out2 = scrubber.feed("text>secret</memory-context>after");
        // 应输出 "after"
        assert_eq!(out2.as_deref(), Some("after"));
    }

    // 测试 5：结束标签跨 chunk 边界
    #[test]
    fn close_tag_split_across_chunks() {
        let mut scrubber = MemoryContextScrubber::new();

        // 开始标签 + 块内容 + 结束标签前半
        let out1 = scrubber.feed("<memory-context>secret</memory-cont");
        // 应无输出（进入块内，缓冲结束标签前半）
        assert_eq!(out1, None);

        // 补全结束标签
        let out2 = scrubber.feed("ext>after");
        assert_eq!(out2.as_deref(), Some("after"));
    }

    // 测试 6：块内容跨多个 chunk
    #[test]
    fn block_content_across_multiple_chunks() {
        let mut scrubber = MemoryContextScrubber::new();

        let out1 = scrubber.feed("<memory-context>");
        assert_eq!(out1, None);

        let out2 = scrubber.feed("part1 ");
        assert_eq!(out2, None);

        let out3 = scrubber.feed("part2 ");
        assert_eq!(out3, None);

        let out4 = scrubber.feed("</memory-context>visible");
        assert_eq!(out4.as_deref(), Some("visible"));
    }

    // 测试 7：未闭合的块，flush 时丢弃内容
    #[test]
    fn unclosed_block_flush_discards_content() {
        let mut scrubber = MemoryContextScrubber::new();

        let out1 = scrubber.feed("visible<memory-context>secret");
        assert_eq!(out1.as_deref(), Some("visible"));

        // flush 时块未闭合，丢弃块内容
        let flushed = scrubber.flush();
        assert_eq!(flushed, None);
        assert!(!scrubber.is_in_block());
    }

    // 测试 8：flush 返回块外的剩余缓冲（以 `<` 结尾的输入会缓冲 `<` 作为可能的标签前缀）
    #[test]
    fn flush_returns_remaining_outside_block() {
        let mut scrubber = MemoryContextScrubber::new();

        // "ab<" 中 "ab" 安全输出，"<" 可能是标签开头，缓冲
        let out1 = scrubber.feed("ab<");
        assert_eq!(out1.as_deref(), Some("ab"));

        // flush 应返回缓冲的 "<"
        let flushed = scrubber.flush();
        assert_eq!(flushed.as_deref(), Some("<"));
    }

    // 测试 9：空 delta 返回 None
    #[test]
    fn empty_delta_returns_none() {
        let mut scrubber = MemoryContextScrubber::new();
        assert_eq!(scrubber.feed(""), None);
    }

    // 测试 10：is_in_block 状态正确
    #[test]
    fn is_in_block_reflects_state() {
        let mut scrubber = MemoryContextScrubber::new();
        assert!(!scrubber.is_in_block());

        scrubber.feed("<memory-context>");
        assert!(scrubber.is_in_block());

        scrubber.feed("</memory-context>");
        assert!(!scrubber.is_in_block());
    }

    // 测试 11：中文内容正常处理
    #[test]
    fn chinese_content_handled() {
        let mut scrubber = MemoryContextScrubber::new();
        let input = "你好<memory-context>记忆内容</memory-context>世界";
        let output = scrubber.feed(input);
        assert_eq!(output.as_deref(), Some("你好世界"));
    }

    // 测试 12：开始标签后立即结束（空块）
    #[test]
    fn empty_block_stripped() {
        let mut scrubber = MemoryContextScrubber::new();
        let input = "a<memory-context></memory-context>b";
        let output = scrubber.feed(input);
        assert_eq!(output.as_deref(), Some("ab"));
    }

    // 测试 13：标签逐字符输入（极端跨 chunk）
    #[test]
    fn tag_input_char_by_char() {
        let mut scrubber = MemoryContextScrubber::new();
        let input = "<memory-context>x</memory-context>";
        let mut output = String::new();

        for ch in input.chars() {
            if let Some(visible) = scrubber.feed(&ch.to_string()) {
                output.push_str(&visible);
            }
        }
        if let Some(visible) = scrubber.flush() {
            output.push_str(&visible);
        }

        assert_eq!(output, "", "逐字符输入应完全剥离块");
    }
}
