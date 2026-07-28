// Code-Review Skill —— 代码审查技能，基于努力级别的多角度审查。
//
// Frontmatter 定义:
// - name: code-review
// - description: Multi-angle code review with effort-based depth
// - when_to_use: 代码审查、代码质量检查、安全审查
//
// Effort Levels:
// - low: 快速审查（1-2 个角度）
// - medium: 标准审查（3-5 个角度）
// - high: 深度审查（6-8 个角度）
// - xhigh: 扩展审查（9-10 个角度）
// - max: 完整审查（所有角度）
//
// Phases:
// - Phase 0: Gather the diff (获取代码变更)
// - Phase 1: Find candidates (多角度查找问题)
// - Phase 2: Verify (验证投票)
//
// Output: JSON 格式审查结果

import type { SkillDefinition, EffortLevel } from "../types";

export const CODE_REVIEW_SKILL_MARKDOWN = `---
name: code-review
description: Multi-angle code review with effort-based depth. Supports 5 effort levels for flexible review scope.
when_to_use:
  - Review code changes before commit
  - Check code quality and maintainability
  - Identify potential bugs and security issues
  - Ensure coding standards compliance
output_format: json
version: "1.0.0"
author: Mnemosyne Team
tags:
  - code-quality
  - review
  - security
parameters:
  target:
    type: string
    description: Target files or directories to review
    required: true
  effort:
    type: string
    description: Effort level (low/medium/high/xhigh/max)
    required: false
    default: medium
    enum: [low, medium, high, xhigh, max]
  focus:
    type: array
    description: Specific areas to focus on
    required: false
---

# Code-Review Skill

This skill performs multi-angle code review with configurable depth.

## Effort Levels

| Level | Finder Angles | Description |
|-------|---------------|-------------|
| low | 1-2 | Quick review, critical issues only |
| medium | 3-5 | Standard review, common issues |
| high | 6-8 | Deep review, detailed analysis |
| xhigh | 9-10 | Extended review, comprehensive coverage |
| max | All | Complete review, all angles |

## Phase 0: Gather the Diff

Collect the code changes to review:

1. Identify changed files via \`git diff\`
2. Read file contents for context
3. Parse diff hunks for review
4. Prepare review scope

\`\`\`
Target: {{target}}
Branch: {{branch}}
\`\`\`

## Phase 1: Find Candidates

Apply multiple finder angles to identify issues:

### Finder Angles (A-J)

- **A. Correctness**: Logic errors, edge cases, race conditions
- **B. Security**: Injection, auth, data validation, secrets
- **C. Performance**: N+1 queries, memory leaks, unnecessary work
- **D. Maintainability**: Naming, structure, duplication, complexity
- **E. Readability**: Comments, formatting, consistency
- **F. Testing**: Coverage, edge cases, mocks, assertions
- **G. Documentation**: API docs, README, comments
- **H. Compatibility**: Version constraints, breaking changes
- **I. Error Handling**: Edge cases, error messages, recovery
- **J. Architecture**: Coupling, cohesion, patterns

Based on effort level, select N finder angles:

\`\`\`
Effort: {{effort}}
Selected Angles: {{selectedAngles}}
\`\`\`

## Phase 2: Verify

Validate findings through voting:

1. Each finding reviewed by 3 reviewers
2. At least 2 votes required to confirm
3. Confirmed findings added to report
4. False positives filtered out

\`\`\`
Voting Strategy: 3-vote adversarial
Threshold: 2/3 votes required
\`\`\`

## Output Format

Returns JSON with review results:

\`\`\`json
{
  "summary": {
    "totalFindings": 5,
    "critical": 1,
    "high": 2,
    "medium": 2,
    "low": 0
  },
  "findings": [
    {
      "id": "F001",
      "file": "src/utils/parser.ts",
      "line": 42,
      "angle": "Correctness",
      "severity": "critical",
      "message": "Null pointer dereference when input is empty",
      "suggestion": "Add null check before processing"
    }
  ]
}
\`\`\`

## Usage

\`\`\`typescript
// Standard review
await run_skill({
  skill: "code-review",
  params: {
    target: "src/",
    effort: "medium"
  }
});

// Deep review with focus
await run_skill({
  skill: "code-review",
  params: {
    target: "src/auth/",
    effort: "high",
    focus: ["security", "error-handling"]
  }
});

// Complete review
await run_skill({
  skill: "code-review",
  params: {
    target: ".",
    effort: "max"
  }
});
\`\`\`
`;

export const FINDER_ANGLES = {
  A: {
    name: "Correctness",
    description: "Logic errors, edge cases, race conditions",
    effortWeight: 1,
  },
  B: {
    name: "Security",
    description: "Injection, auth, data validation, secrets",
    effortWeight: 1,
  },
  C: {
    name: "Performance",
    description: "N+1 queries, memory leaks, unnecessary work",
    effortWeight: 1,
  },
  D: {
    name: "Maintainability",
    description: "Naming, structure, duplication, complexity",
    effortWeight: 1,
  },
  E: {
    name: "Readability",
    description: "Comments, formatting, consistency",
    effortWeight: 1,
  },
  F: {
    name: "Testing",
    description: "Coverage, edge cases, mocks, assertions",
    effortWeight: 1,
  },
  G: {
    name: "Documentation",
    description: "API docs, README, comments",
    effortWeight: 1,
  },
  H: {
    name: "Compatibility",
    description: "Version constraints, breaking changes",
    effortWeight: 1,
  },
  I: {
    name: "Error Handling",
    description: "Edge cases, error messages, recovery",
    effortWeight: 1,
  },
  J: {
    name: "Architecture",
    description: "Coupling, cohesion, patterns",
    effortWeight: 1,
  },
} as const;

export const EFFORT_ANGLE_COUNT: Record<EffortLevel, number> = {
  low: 2,
  medium: 5,
  high: 8,
  xhigh: 10,
  max: 10,
};

export function selectFinderAngles(effort: EffortLevel): string[] {
  const count = EFFORT_ANGLE_COUNT[effort];
  return Object.keys(FINDER_ANGLES).slice(0, count);
}

export const CODE_REVIEW_SKILL_DEFINITION: SkillDefinition = {
  metadata: {
    name: "code-review",
    description:
      "Multi-angle code review with effort-based depth. Supports 5 effort levels for flexible review scope.",
    whenToUse: [
      "Review code changes before commit",
      "Check code quality and maintainability",
      "Identify potential bugs and security issues",
      "Ensure coding standards compliance",
    ],
    version: "1.0.0",
    author: "Mnemosyne Team",
    tags: ["code-quality", "review", "security"],
  },
  phases: [
    {
      name: "Gather the Diff",
      instructions: `Collect the code changes to review:

1. Identify changed files via git diff
2. Read file contents for context
3. Parse diff hunks for review
4. Prepare review scope

Target: {{target}}
Workspace: {{workspaceId}}`,
      type: "discover",
      timeoutMs: 30000,
    },
    {
      name: "Find Candidates",
      instructions: `Apply multiple finder angles to identify issues:

Finder Angles (A-J):
- A. Correctness: Logic errors, edge cases, race conditions
- B. Security: Injection, auth, data validation, secrets
- C. Performance: N+1 queries, memory leaks, unnecessary work
- D. Maintainability: Naming, structure, duplication, complexity
- E. Readability: Comments, formatting, consistency
- F. Testing: Coverage, edge cases, mocks, assertions
- G. Documentation: API docs, README, comments
- H. Compatibility: Version constraints, breaking changes
- I. Error Handling: Edge cases, error messages, recovery
- J. Architecture: Coupling, cohesion, patterns

Effort: {{effort}}
Selected Angles: {{selectedAngles}}`,
      type: "deliver",
      timeoutMs: 60000,
    },
    {
      name: "Verify",
      instructions: `Validate findings through voting:

1. Each finding reviewed by 3 reviewers
2. At least 2 votes required to confirm
3. Confirmed findings added to report
4. False positives filtered out

Voting Strategy: 3-vote adversarial
Threshold: 2/3 votes required`,
      type: "verify",
      timeoutMs: 30000,
    },
  ],
  outputFormat: "json",
  parameters: {
    target: {
      type: "string",
      description: "Target files or directories to review",
      required: true,
    },
    effort: {
      type: "string",
      description: "Effort level (low/medium/high/xhigh/max)",
      required: false,
      default: "medium",
      enum: ["low", "medium", "high", "xhigh", "max"],
    },
    focus: {
      type: "array",
      description: "Specific areas to focus on",
      required: false,
    },
  },
};

export function getCodeReviewSkillDefinition(): SkillDefinition {
  return CODE_REVIEW_SKILL_DEFINITION;
}