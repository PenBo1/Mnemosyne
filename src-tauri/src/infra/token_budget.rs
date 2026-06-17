use serde::{Deserialize, Serialize};

/// Approximate token count (1 token ≈ 4 chars for English, 1.5 chars for CJK).
pub fn estimate_tokens(text: &str) -> u32 {
    let mut count = 0u32;
    for ch in text.chars() {
        if ch.is_ascii() {
            count += 1;
        } else {
            // CJK and other multi-byte chars ≈ 2 tokens per char
            count += 2;
        }
    }
    // Rough division: 4 chars per token
    (count + 3) / 4
}

/// Token budget allocation for a context window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBudget {
    pub total: u32,
    pub system: u32,
    pub history: u32,
    pub tools: u32,
    pub response: u32,
    pub reserved: u32,
}

impl ContextBudget {
    /// Create a budget for a given context window size.
    /// Allocates: 10% system, 60% history, 5% tools, 20% response, 5% reserved.
    pub fn for_window(window_size: u32) -> Self {
        Self {
            total: window_size,
            system: window_size / 10,
            history: (window_size * 6) / 10,
            tools: window_size / 20,
            response: window_size / 5,
            reserved: window_size / 20,
        }
    }

    /// Available tokens for history after system prompt.
    pub fn available_for_history(&self, system_tokens: u32, tool_tokens: u32) -> u32 {
        self.total
            .saturating_sub(self.response)
            .saturating_sub(self.reserved)
            .saturating_sub(system_tokens)
            .saturating_sub(tool_tokens)
    }

    /// Check if compaction is needed.
    pub fn needs_compaction(&self, used_tokens: u32) -> bool {
        used_tokens > (self.total * 85) / 100
    }

    /// Max messages to keep after compaction (rough estimate).
    pub fn max_messages_after_compact(&self, avg_msg_tokens: u32) -> usize {
        if avg_msg_tokens == 0 {
            return 20;
        }
        let budget = self.available_for_history(self.system, self.tools);
        (budget / avg_msg_tokens) as usize
    }
}

impl Default for ContextBudget {
    fn default() -> Self {
        Self::for_window(128_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens_ascii() {
        assert_eq!(estimate_tokens("hello"), 2); // 5 chars / 4 = 1.25 -> 2
    }

    #[test]
    fn test_estimate_tokens_cjk() {
        let tokens = estimate_tokens("你好世界");
        assert!(tokens > 0);
    }

    #[test]
    fn test_context_budget() {
        let budget = ContextBudget::for_window(100_000);
        assert_eq!(budget.total, 100_000);
        assert!(budget.system > 0);
        assert!(budget.history > 0);
        assert!(budget.response > 0);
    }

    #[test]
    fn test_needs_compaction() {
        let budget = ContextBudget::for_window(100_000);
        assert!(!budget.needs_compaction(50_000));
        assert!(budget.needs_compaction(90_000));
    }
}
