//! ═══════════════════════════════════════════════════════════════════════════
//! Thinking Scrubber - 思考块剥离器
//! ═══════════════════════════════════════════════════════════════════════════

/// 支持的思考标签列表
const THINKING_TAGS: &[&str] = &[
    "thinking",
    "think",
    "reasoning",
    "thought",
    "REASONING_SCRATCHPAD",
];

/// 思考块结构
pub struct ThinkingBlock {
    /// 块内容
    pub content: String,
    /// 标签名称
    pub tag: String,
}

// ── 流式剥离器 ──────────────────────────────────────────────────────────────

/// 流式思考块剥离器
/// 
/// 用于在流式输出过程中剥离 thinking 标签块。
pub struct StreamingThinkScrubber {
    /// 是否在块中
    in_block: bool,
    /// 当前标签
    current_tag: Option<String>,
    /// 缓冲区
    buffer: String,
    /// 块缓冲区
    block_buffer: String,
}

impl Default for StreamingThinkScrubber {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingThinkScrubber {
    /// 创建剥离器实例
    pub fn new() -> Self {
        Self {
            in_block: false,
            current_tag: None,
            buffer: String::new(),
            block_buffer: String::new(),
        }
    }

    /// 填充数据并返回可见内容
    pub fn feed(&mut self, delta: &str) -> Option<String> {
        self.buffer.push_str(delta);
        self.process_buffer()
    }

    /// 处理缓冲区
    fn process_buffer(&mut self) -> Option<String> {
        let mut visible_output = String::new();

        while !self.buffer.is_empty() {
            if self.in_block {
                if let Some(end_pos) = self.find_closing_tag() {
                    let content_before = &self.buffer[..end_pos];
                    self.block_buffer.push_str(content_before);
                    
                    let tag = self.current_tag.clone().unwrap();
                    let close_tag_len = format!("</{}>", tag).len();
                    self.buffer = self.buffer[end_pos + close_tag_len..].to_string();
                    
                    tracing::trace!(
                        tag = %tag,
                        block_len = self.block_buffer.len(),
                        "[thinking] thinking block ended"
                    );
                    
                    self.in_block = false;
                    self.current_tag = None;
                    self.block_buffer.clear();
                } else {
                    self.block_buffer.push_str(&self.buffer);
                    self.buffer.clear();
                    break;
                }
            } else {
                let mut found_tag = false;
                for tag in THINKING_TAGS {
                    let open_tag = format!("<{}>", tag);
                    if let Some(pos) = self.find_tag_position(&open_tag) {
                        if pos == 0 || self.buffer[..pos].chars().all(|c| c.is_whitespace() || c == '\n') {
                            visible_output.push_str(&self.buffer[..pos]);
                            self.buffer = self.buffer[pos + open_tag.len()..].to_string();
                            self.in_block = true;
                            self.current_tag = Some(tag.to_string());
                            self.block_buffer.clear();
                            
                            tracing::trace!(tag = %tag, "[thinking] thinking block started");
                            found_tag = true;
                            break;
                        }
                    }
                }
                
                if !found_tag {
                    let safe_len = self.safe_visible_length();
                    visible_output.push_str(&self.buffer[..safe_len]);
                    self.buffer = self.buffer[safe_len..].to_string();
                }
            }
        }

        if visible_output.is_empty() {
            None
        } else {
            Some(visible_output)
        }
    }

    /// 查找标签位置
    fn find_tag_position(&self, tag: &str) -> Option<usize> {
        self.buffer.find(tag)
    }

    /// 查找闭合标签位置
    fn find_closing_tag(&self) -> Option<usize> {
        if let Some(ref tag) = self.current_tag {
            let close_tag = format!("</{}>", tag);
            self.buffer.find(&close_tag)
        } else {
            None
        }
    }

    /// 计算安全可见长度
    fn safe_visible_length(&self) -> usize {
        let buffer = &self.buffer;
        
        for tag in THINKING_TAGS {
            let open_tag = format!("<{}>", tag);
            if let Some(pos) = buffer.find(&open_tag) {
                let prefix = &buffer[..pos];
                let last_newline = prefix.chars().rev().position(|c| c == '\n');
                if let Some(newline_offset) = last_newline {
                    return pos - newline_offset;
                }
                return pos;
            }
        }
        
        if buffer.len() > 20 {
            let max_partial_tag_len = 15;
            buffer.len().saturating_sub(max_partial_tag_len)
        } else {
            buffer.len()
        }
    }

    /// 刷新并返回剩余内容
    pub fn flush(&mut self) -> Option<String> {
        if self.in_block && !self.block_buffer.is_empty() {
            tracing::debug!(
                tag = %self.current_tag.as_deref().unwrap_or("unknown"),
                block_len = self.block_buffer.len(),
                "[thinking] flushing unclosed thinking block"
            );
        }
        
        if !self.buffer.is_empty() {
            let remaining = self.buffer.clone();
            self.buffer.clear();
            Some(remaining)
        } else {
            None
        }
    }

    /// 检查是否在块中
    pub fn is_in_block(&self) -> bool {
        self.in_block
    }

    /// 获取当前块内容
    pub fn current_block_content(&self) -> Option<&str> {
        if self.in_block {
            Some(&self.block_buffer)
        } else {
            None
        }
    }

    /// 取出并清空块缓冲区
    pub fn take_block_buffer(&mut self) -> Option<String> {
        if self.block_buffer.is_empty() {
            None
        } else {
            let buffer = std::mem::take(&mut self.block_buffer);
            Some(buffer)
        }
    }
}

// ── 批量提取函数 ────────────────────────────────────────────────────────────

/// 从文本中提取思考块
/// 
/// # 参数
/// - `text`: 输入文本
/// 
/// # 返回值
/// 返回 (思考块列表, 可见文本)
pub fn extract_thinking_blocks(text: &str) -> (Vec<ThinkingBlock>, String) {
    let mut blocks = Vec::new();
    let mut visible_text = String::new();
    let mut remaining = text;
    
    while !remaining.is_empty() {
        let mut found = false;
        
        for tag in THINKING_TAGS {
            let open_tag = format!("<{}>", tag);
            let close_tag = format!("</{}>", tag);
            
            if let Some(start) = remaining.find(&open_tag) {
                let valid_start = start == 0 || remaining[..start].chars().all(|c| c.is_whitespace() || c == '\n');
                
                if valid_start {
                    if let Some(content_start) = remaining.find(&open_tag) {
                        let after_open = &remaining[content_start + open_tag.len()..];
                        if let Some(end) = after_open.find(&close_tag) {
                            let content = after_open[..end].trim().to_string();
                            if !content.is_empty() {
                                blocks.push(ThinkingBlock {
                                    content,
                                    tag: tag.to_string(),
                                });
                            }
                            remaining = &after_open[end + close_tag.len()..];
                            found = true;
                            break;
                        }
                    }
                }
            }
        }
        
        if !found {
            if let Some(next_tag_pos) = remaining.find('<') {
                visible_text.push_str(&remaining[..next_tag_pos]);
                remaining = &remaining[next_tag_pos..];
            } else {
                visible_text.push_str(remaining);
                break;
            }
        }
    }
    
    (blocks, visible_text.trim().to_string())
}

// ── 测试用例 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_thinking_blocks() {
        let text = r#"Let me think about this.
<thinking>
I need to analyze the request first.
Then consider the options.
</thinking>
Here is my response."#;
        
        let (blocks, visible) = extract_thinking_blocks(text);
        
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].tag, "thinking");
        assert!(blocks[0].content.contains("analyze the request"));
        assert!(visible.contains("Let me think about this."));
        assert!(visible.contains("Here is my response."));
    }

    #[test]
    fn test_streaming_scrubber() {
        let mut scrubber = StreamingThinkScrubber::new();
        
        let output = scrubber.feed("Hello ");
        assert_eq!(output, Some("Hello ".to_string()));
        
        let output = scrubber.feed("<thinking>");
        assert_eq!(output, None);
        
        let output = scrubber.feed("Thinking content here...");
        assert_eq!(output, None);
        
        let output = scrubber.feed("</thinking>");
        assert_eq!(output, None);
        
        let output = scrubber.feed(" World");
        assert_eq!(output, Some(" World".to_string()));
    }
}