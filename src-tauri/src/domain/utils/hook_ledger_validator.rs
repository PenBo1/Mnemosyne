//! Hook ledger validator -- validates that chapter draft acts on hook ledger.

use crate::domain::story::AuditSeverity;

pub struct HookLedgerViolation {
    pub severity: AuditSeverity,
    pub category: String,
    pub description: String,
    pub suggestion: String,
}

/// Validate that the draft acts on the hook ledger declared in the memo.
pub fn validate_hook_ledger(memo_body: &str, draft_content: &str) -> Vec<HookLedgerViolation> {
    let ledger = parse_hook_ledger(memo_body);
    let mut violations = Vec::new();

    let committed: Vec<&HookLedgerEntry> = ledger.advance.iter()
        .chain(ledger.resolve.iter())
        .collect();

    for entry in &committed {
        if !draft_echoes_entry(draft_content, entry) {
            violations.push(HookLedgerViolation {
                severity: AuditSeverity::Warning,
                category: "hook 账需语义复核".to_string(),
                description: format!(
                    "memo 在 advance/resolve 里声明要处理 {}，但确定性关键词检查没有找到对应落点",
                    entry.id
                ),
                suggestion: format!(
                    "复核正文是否已经用动作、对话、物件或信息变化推进了 {}；若没有，请补具体场景",
                    entry.id
                ),
            });
        }
    }

    let resolved_count = ledger.resolve.len();
    let opened_count = ledger.open.len() + ledger.new_open_count as usize;
    if resolved_count > 0 && opened_count < resolved_count {
        violations.push(HookLedgerViolation {
            severity: AuditSeverity::Critical,
            category: "hook 账揭 1 埋 1 违规".to_string(),
            description: format!(
                "本章 resolve 了 {} 个钩子，但 open 只有 {} 个新钩子",
                resolved_count, opened_count
            ),
            suggestion: "在 memo 的 open 段下至少再埋与已揭钩子相关的新钩子".to_string(),
        });
    }

    violations
}

pub struct HookLedger {
    pub open: Vec<HookLedgerEntry>,
    pub advance: Vec<HookLedgerEntry>,
    pub resolve: Vec<HookLedgerEntry>,
    pub defer: Vec<HookLedgerEntry>,
    pub new_open_count: u32,
}

pub struct HookLedgerEntry {
    pub id: String,
    pub descriptor: String,
    pub keywords: Vec<String>,
}

fn parse_hook_ledger(memo_body: &str) -> HookLedger {
    let section = extract_ledger_section(memo_body);
    let empty = HookLedger {
        open: vec![],
        advance: vec![],
        resolve: vec![],
        defer: vec![],
        new_open_count: 0,
    };

    let section = match section {
        Some(s) => s,
        None => return empty,
    };

    let mut ledger = empty;
    let mut current: Option<String> = None;

    let sub_heading_re = regex::Regex::new(r"(?i)^(open|advance|resolve|defer)\s*[:：]?\s*$").unwrap();

    for raw_line in section.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(caps) = sub_heading_re.captures(line) {
            current = Some(caps[1].to_lowercase());
            continue;
        }

        if current.is_none() || !line.starts_with('-') {
            continue;
        }

        let cleaned = line.trim_start_matches(['-', ' ']);

        if current.as_deref() == Some("open") && cleaned.to_lowercase().starts_with("[new]") {
            ledger.new_open_count += 1;
            continue;
        }

        if let Some(entry) = extract_ledger_entry(cleaned) {
            match current.as_deref() {
                Some("open") => ledger.open.push(entry),
                Some("advance") => ledger.advance.push(entry),
                Some("resolve") => ledger.resolve.push(entry),
                Some("defer") => ledger.defer.push(entry),
                _ => {}
            }
        }
    }

    ledger
}

fn extract_ledger_section(memo_body: &str) -> Option<String> {
    let patterns = [
        regex::Regex::new(r"(?mi)^#{2,3}\s*本章\s*hook\s*账\s*$").unwrap(),
        regex::Regex::new(r"(?mi)^#{2,3}\s*Hook\s+ledger\s+for\s+this\s+chapter\s*$").unwrap(),
    ];
    let next_heading_re = regex::Regex::new(r"\n#{2,3}\s").unwrap();

    for pattern in &patterns {
        if let Some(caps) = pattern.captures(memo_body) {
            let start = caps.get(0).unwrap().end();
            let rest = &memo_body[start..];
            let end = next_heading_re.find(rest).map(|m| m.start()).unwrap_or(rest.len());
            return Some(rest[..end].to_string());
        }
    }
    None
}

fn extract_ledger_entry(line: &str) -> Option<HookLedgerEntry> {
    let cleaned = line.trim();
    if cleaned.starts_with("[new]") || cleaned.starts_with("[NEW]") {
        return None;
    }

    let placeholder = regex::Regex::new(r"(?i)^(无|空|none|nil|null|暂无|n/a|na|tbd|todo|待定)$").unwrap();
    let first_word = cleaned.split_whitespace().next().unwrap_or("");
    if placeholder.is_match(first_word) {
        return None;
    }

    let id_match = regex::Regex::new(r"^([A-Za-z\u{4e00}-\u{9fff}][A-Za-z0-9_\-\u{4e00}-\u{9fff}]{0,19})").unwrap();
    let caps = id_match.captures(cleaned)?;
    let candidate = caps.get(1)?.as_str();

    let subsection_words = regex::Regex::new(r"(?i)^(open|advance|resolve|defer|new)$").unwrap();
    if subsection_words.is_match(candidate) || placeholder.is_match(candidate) {
        return None;
    }

    let descriptor = cleaned[candidate.len()..].trim().to_string();
    let keywords = extract_keywords(&descriptor);

    Some(HookLedgerEntry {
        id: candidate.to_string(),
        descriptor,
        keywords,
    })
}

fn extract_keywords(descriptor: &str) -> Vec<String> {
    if descriptor.is_empty() {
        return vec![];
    }

    let quoted = regex::Regex::new(r#""([^"\n]+)""#).unwrap();
    let source = quoted.captures(descriptor)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .unwrap_or(descriptor.split("->").next().unwrap_or(descriptor));

    let cjk_runs = regex::Regex::new(r"[\u{4e00}-\u{9fff}]{2,}").unwrap();
    let mut tokens: Vec<String> = Vec::new();

    for m in cjk_runs.find_iter(source) {
        let run = m.as_str();
        tokens.push(run.to_string());
        if run.len() >= 3 {
            for i in 0..run.len() - 1 {
                tokens.push(run[i..i + 2].to_string());
            }
        }
    }

    let ascii_words = regex::Regex::new(r"[A-Za-z]{3,}").unwrap();
    for m in ascii_words.find_iter(source) {
        tokens.push(m.as_str().to_lowercase());
    }

    tokens.sort();
    tokens.dedup();
    tokens
}

fn draft_echoes_entry(draft: &str, entry: &HookLedgerEntry) -> bool {
    if !entry.keywords.is_empty() {
        let draft_lower = draft.to_lowercase();
        return entry.keywords.iter().any(|kw| {
            if kw.starts_with(|c: char| c.is_ascii_alphabetic()) {
                draft_lower.contains(kw)
            } else {
                draft.contains(kw)
            }
        });
    }
    if regex::Regex::new(r"^[A-Za-z0-9_-]+$").unwrap().is_match(&entry.id) {
        let pattern = regex::Regex::new(&format!(r"\b{}\b", regex::escape(&entry.id))).unwrap();
        return pattern.is_match(draft);
    }
    draft.contains(&entry.id)
}
