// 跨 chunk 行缓冲区，用于 SSE / NDJSON 流式解析。
//
// 问题：HTTP 流式响应按 chunk 到达，一个 chunk 可能：
// 1. 包含多行（最后一个换行符之后是不完整行）
// 2. 在多字节 UTF-8 字符中间被切断
// 3. 在 SSE `data: {...}\n` 的 JSON 中间被切断
//
// 直接用 `String::from_utf8_lossy(&bytes).lines()` 会导致：
// - 不完整行在下个 chunk 到达时丢失（每 chunk 独立解析，无状态）
// - 多字节字符切断处出现 U+FFFD 替换字符（lossy 解码无法恢复）
//
// 解决：按 bytes 累积，仅在遇到 `\n` 时把完整行 `from_utf8` 解码返回。

/// 跨 chunk 行缓冲区
///
/// 累积原始字节，按 `\n` 分割。最后一段不含 `\n` 的字节保留到下次 push，
/// 直到补齐完整行再返回。多字节 UTF-8 字符被 chunk 边界切断时不会产生 U+FFFD。
pub struct SseLineBuffer {
    buf: Vec<u8>,
}

impl SseLineBuffer {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// 推入一个 chunk 的字节，返回所有**完整行**（去掉行尾 `\n` / `\r\n`）。
    ///
    /// 最后一段不含 `\n` 的字节会保留在内部缓冲区，等下次 push 补齐后再返回。
    pub fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        self.buf.extend_from_slice(chunk);
        let mut lines = Vec::new();
        while let Some(idx) = self.buf.iter().position(|&b| b == b'\n') {
            let mut line_bytes: Vec<u8> = self.buf.drain(..=idx).collect();
            // 去掉行尾的 \n
            line_bytes.pop();
            // 去掉行尾的 \r（CRLF）
            if line_bytes.last() == Some(&b'\r') {
                line_bytes.pop();
            }
            let line = match std::str::from_utf8(&line_bytes) {
                Ok(s) => s.to_string(),
                Err(_) => String::from_utf8_lossy(&line_bytes).into_owned(),
            };
            lines.push(line);
        }
        lines
    }
}

impl Default for SseLineBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_lines_returned_immediately() {
        let mut buf = SseLineBuffer::new();
        let lines = buf.push(b"data: {\"a\":1}\ndata: {\"a\":2}\n");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "data: {\"a\":1}");
        assert_eq!(lines[1], "data: {\"a\":2}");
    }

    #[test]
    fn partial_line_retained_across_chunks() {
        let mut buf = SseLineBuffer::new();
        let lines = buf.push(b"data: {\"a\"");
        assert!(lines.is_empty());
        let lines = buf.push(b":1}\n");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "data: {\"a\":1}");
    }

    #[test]
    fn multibyte_utf8_not_corrupted_at_chunk_boundary() {
        // "中文" in UTF-8: e4 b8 ad e6 96 87
        let full = "中文\n";
        let bytes = full.as_bytes();
        let mid = 3; // 在 "中"(e4 b8 ad) 之后、"文"(e6 96 87) 之前切断
        let mut buf = SseLineBuffer::new();
        let lines1 = buf.push(&bytes[..mid]);
        assert!(lines1.is_empty(), "切断的多字节字符不应产出残缺行");
        let lines2 = buf.push(&bytes[mid..]);
        assert_eq!(lines2.len(), 1);
        assert_eq!(lines2[0], "中文");
    }

    #[test]
    fn crlf_line_endings_stripped() {
        let mut buf = SseLineBuffer::new();
        let lines = buf.push(b"line1\r\nline2\r\n");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "line1");
        assert_eq!(lines[1], "line2");
    }

    #[test]
    fn empty_chunk_returns_nothing() {
        let mut buf = SseLineBuffer::new();
        let lines = buf.push(b"");
        assert!(lines.is_empty());
    }

    #[test]
    fn no_trailing_newline_retains_all() {
        let mut buf = SseLineBuffer::new();
        let lines = buf.push(b"incomplete line without newline");
        assert!(lines.is_empty());
    }
}
