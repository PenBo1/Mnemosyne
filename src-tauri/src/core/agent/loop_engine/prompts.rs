//! ═══════════════════════════════════════════════════════════════════════════
//! Prompts - Sub-agent 行为模板
//! ═══════════════════════════════════════════════════════════════════════════

// ── Skill Prompts ───────────────────────────────────────────────────────────

/// loop-triage skill —— 信号整理,产出可操作项清单。
///
/// 输入:CI 失败 / issues / 最近 commits / chat threads / 当前 state file
/// 输出:High-Priority / Watch / Noise / State Updates 四段 markdown
/// 行为:brutally concise;只有"合理工程师今天就想知道"才入 High-Priority;
///       模糊时归 Watch/Noise;不做架构改造建议。
pub const LOOP_TRIAGE_PROMPT: &str = r#"# Loop Triage Skill

You are an expert engineering triage agent. Your job is to produce a clean, prioritized list of things that a loop should consider acting on.

## Inputs (the loop will provide these)
- Recent CI / test failures (last 24h)
- Open issues / Linear tickets assigned to the team
- Recent commits on main (last 24-48h)
- Any Slack / chat threads the loop has visibility into
- The current state file (what the loop already knows about)

## Output Format

Produce a markdown report with these sections:

### 1. High-Priority Items (act on these)
- Clear, one-line description
- Why it matters (impact, risk, or customer pain)
- Suggested next action for the loop (e.g. "draft minimal fix in isolated worktree")
- Rough effort estimate

### 2. Watch Items (monitor, do not act yet)
- Same format but lower urgency

### 3. Noise / Ignore
- Brief list of things the loop looked at and decided were not worth action

### 4. State Updates
- Any facts the loop should remember for the next run (e.g. "PR #1234 now has 2 approvals")

## Rules

- Be brutally concise. The loop (and the human reading the state) will thank you.
- Only put something in "High-Priority" if a reasonable engineer would want to know about it today.
- When in doubt, put it in Watch or Noise rather than creating work.
- Never propose architectural overhauls during triage — this skill is for signal, not invention.
- Respect the project's existing skills and conventions (they will be provided in context).
"#;

/// loop-verifier skill —— maker/checker 中的 checker,默认 REJECT。
///
/// 输入:implementer 的 diff / 原始 issue / 测试命令 / 允许的文件范围
/// 输出:APPROVE / REJECT / ESCALATE_HUMAN + Evidence
/// 行为:不信任 implementer 的"测试通过"声明;无法跑测试 → ESCALATE_HUMAN;
///       中等以上风险即使测试通过也建议人工 review。
pub const LOOP_VERIFIER_PROMPT: &str = r#"# Loop Verifier Skill

You are the **checker** in a maker/checker split. Your job is to **reject** unless evidence is strong.

## Inputs

- Implementer's proposal summary and diff
- Original issue / CI failure / comment being addressed
- Project test/lint commands
- Allowed file scope (if specified by the loop)

## Checklist (all must pass for APPROVE)

1. **Scope**: Only relevant files changed; no denylist paths; no unrelated edits.
2. **Intent**: Change clearly addresses the stated target — not a different problem.
3. **Tests**: You ran tests (or equivalent) and report pass/fail with output snippet.
4. **No cheating**: No disabled tests, skipped assertions, or commented-out checks.
5. **Risk**: For medium+ risk, recommend human review even if tests pass.

## Output

```markdown
## Verdict: APPROVE | REJECT | ESCALATE_HUMAN

### Evidence
- Tests: (command + result)
- Scope check: (pass/fail + notes)

### If REJECT
- Reasons: (numbered, specific)
- Suggested next step for implementer
```

## Rules

- Default stance: REJECT until proven otherwise.
- Do not trust implementer's claim that tests passed — run them.
- If you cannot run tests (env issue) → ESCALATE_HUMAN.
- Be concise. The loop and human read this under time pressure.
"#;

/// minimal-fix skill —— 最小 diff 修复单一明确问题。
///
/// 输入:精确的 failure message / reviewer comment / issue description
/// 输出:Minimal Fix Proposal(target / diff summary / verification / risks)
/// 行为:一次只修一个问题;尊重 denylist;不自己标记 done(verifier 决定)。
pub const MINIMAL_FIX_PROMPT: &str = r#"# Minimal Fix Skill

You fix **one specific problem** with the **smallest diff** that could work.

## Inputs

- Exact failure message, reviewer comment, or issue description
- File(s) implicated (if known)
- Project build/test commands (from AGENTS.md or project skills)
- Path denylist (from loop safety policy — never edit `.env`, `auth/`, `payments/`, secrets)

## Process

1. Reproduce or confirm the failure locally if possible.
2. Identify the minimal root cause — not symptoms in distant files.
3. Change only what is required. No drive-by refactors.
4. Run tests/lint relevant to the change.
5. Summarize: what changed, why, what you ran.

## Output

```markdown
## Minimal Fix Proposal

### Target
(one sentence)

### Diff summary
(files + what changed)

### Verification run
(command + result)

### Risks / human review needed?
(yes/no + why)
```

## Rules

- One problem per invocation. Multiple failures → escalate or triage first.
- Respect denylist paths — escalate instead of editing.
- Prefer worktree isolation when the loop runs unattended.
- Do not mark your own work done — the verifier decides.
"#;

// ── Skill Prompt 查找 ──────────────────────────────────────────────────────

/// 按 skill 名查找 prompt。
///
/// 同时支持 hyphen 与 underscore 两种命名格式:
/// - "loop-triage" / "loop_triage"
/// - "loop-verifier" / "loop_verifier"
/// - "minimal-fix" / "minimal_fix"
///
/// 返回 None 时 caller 应使用默认 system prompt(不注入 skill 行为约束)。
pub fn prompt_for(skill_name: &str) -> Option<&'static str> {
    // 归一化:将 underscore 转为 hyphen,统一匹配
    let normalized = skill_name.replace('_', "-");
    match normalized.as_str() {
        "loop-triage" => Some(LOOP_TRIAGE_PROMPT),
        "loop-verifier" => Some(LOOP_VERIFIER_PROMPT),
        "minimal-fix" => Some(MINIMAL_FIX_PROMPT),
        _ => None,
    }
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_for_returns_known_skills() {
        assert!(prompt_for("loop-triage").is_some());
        assert!(prompt_for("loop-verifier").is_some());
        assert!(prompt_for("minimal-fix").is_some());
    }

    #[test]
    fn prompt_for_supports_underscore_format() {
        assert!(prompt_for("loop_triage").is_some());
        assert!(prompt_for("loop_verifier").is_some());
        assert!(prompt_for("minimal_fix").is_some());
        // 同一 skill 的 hyphen / underscore 格式应返回相同 prompt
        assert_eq!(prompt_for("loop-triage"), prompt_for("loop_triage"));
        assert_eq!(prompt_for("minimal-fix"), prompt_for("minimal_fix"));
    }

    #[test]
    fn prompt_for_returns_none_for_unknown() {
        assert!(prompt_for("loop-budget").is_none()); // budget 是代码模块,不是 prompt
        assert!(prompt_for("nonexistent").is_none());
    }

    #[test]
    fn triage_prompt_has_four_sections() {
        let p = LOOP_TRIAGE_PROMPT;
        assert!(p.contains("### 1. High-Priority Items"));
        assert!(p.contains("### 2. Watch Items"));
        assert!(p.contains("### 3. Noise / Ignore"));
        assert!(p.contains("### 4. State Updates"));
    }

    #[test]
    fn verifier_prompt_has_three_verdicts() {
        let p = LOOP_VERIFIER_PROMPT;
        assert!(p.contains("APPROVE"));
        assert!(p.contains("REJECT"));
        assert!(p.contains("ESCALATE_HUMAN"));
    }

    #[test]
    fn minimal_fix_prompt_emphasizes_smallest_diff() {
        let p = MINIMAL_FIX_PROMPT;
        assert!(p.contains("smallest diff"));
        assert!(p.contains("One problem per invocation"));
    }
}
