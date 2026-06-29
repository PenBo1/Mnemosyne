/// Draft directive parser for parsing user instructions
pub struct DraftDirectiveParser;

impl DraftDirectiveParser {
    /// Parse a draft directive from user input
    pub fn parse_directive(input: &str) -> DraftDirective {
        let trimmed = input.trim();

        // Check for slash commands
        if let Some(cmd) = trimmed.strip_prefix('/') {
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            if let Some(first) = parts.first() {
                match *first {
                    "write" | "续写" => return DraftDirective::WriteNext,
                    "audit" | "审计" => return DraftDirective::AuditChapter,
                    "revise" | "修订" => return DraftDirective::ReviseChapter,
                    "create" | "创建" => return DraftDirective::CreateBook,
                    "status" | "状态" => return DraftDirective::ShowStatus,
                    "help" | "帮助" => return DraftDirective::ShowHelp,
                    _ => return DraftDirective::Unknown(cmd.to_string()),
                }
            }
        }

        // Check for Chinese keywords
        if trimmed.contains("续写") || trimmed.contains("下一章") || trimmed.contains("继续写") {
            return DraftDirective::WriteNext;
        }
        if trimmed.contains("审计") || trimmed.contains("检查") {
            return DraftDirective::AuditChapter;
        }
        if trimmed.contains("修订") || trimmed.contains("修改") {
            return DraftDirective::ReviseChapter;
        }
        if trimmed.contains("创建") && (trimmed.contains("书") || trimmed.contains("小说")) {
            return DraftDirective::CreateBook;
        }

        DraftDirective::Chat(trimmed.to_string())
    }
}

#[derive(Debug, Clone)]
pub enum DraftDirective {
    WriteNext,
    AuditChapter,
    ReviseChapter,
    CreateBook,
    ShowStatus,
    ShowHelp,
    Chat(String),
    Unknown(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_slash_commands() {
        assert!(matches!(DraftDirectiveParser::parse_directive("/write"), DraftDirective::WriteNext));
        assert!(matches!(DraftDirectiveParser::parse_directive("/audit"), DraftDirective::AuditChapter));
        assert!(matches!(DraftDirectiveParser::parse_directive("/revise"), DraftDirective::ReviseChapter));
    }

    #[test]
    fn test_parse_chinese_keywords() {
        assert!(matches!(DraftDirectiveParser::parse_directive("续写下一章"), DraftDirective::WriteNext));
        assert!(matches!(DraftDirectiveParser::parse_directive("审计这一章"), DraftDirective::AuditChapter));
    }

    #[test]
    fn test_parse_chat() {
        assert!(matches!(DraftDirectiveParser::parse_directive("hello world"), DraftDirective::Chat(_)));
    }
}
