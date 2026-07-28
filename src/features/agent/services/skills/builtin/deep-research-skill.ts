// Deep-Research Skill —— 深度研究技能，5 阶段并行研究流程。
//
// Frontmatter 定义:
// - name: deep-research
// - description: Multi-phase deep research with parallel search and adversarial verification
// - when_to_use: 深度研究、技术调研、竞品分析
//
// Phases:
// - Phase 1: Scope (分解问题为 5 个搜索角度)
// - Phase 2: Search (5 个并行 WebSearch Agent)
// - Phase 3: Fetch (URL 去重，获取前 15 个源)
// - Phase 4: Verify (3 票对抗式验证)
// - Phase 5: Synthesize (合并结果)
//
// Output: Markdown 格式研究报告

import type { SkillDefinition } from "../types";

export const DEEP_RESEARCH_SKILL_MARKDOWN = `---
name: deep-research
description: Multi-phase deep research with parallel search and adversarial verification. Generates comprehensive research reports.
when_to_use:
  - Deep research on technical topics
  - Competitive analysis
  - Technology trend investigation
  - Comprehensive literature review
output_format: markdown
version: "1.0.0"
author: Mnemosyne Team
tags:
  - research
  - analysis
  - web-search
parameters:
  query:
    type: string
    description: Research query or question
    required: true
  depth:
    type: number
    description: Search depth (1-5)
    required: false
    default: 3
    min: 1
    max: 5
  sources:
    type: number
    description: Maximum number of sources to include
    required: false
    default: 15
    min: 5
    max: 50
---

# Deep-Research Skill

This skill performs comprehensive research through a 5-phase parallel pipeline.

## Phase 1: Scope

Decompose the research question into 5 search angles:

1. Analyze the query for key concepts
2. Identify 5 distinct search angles:
   - Angle 1: Broad overview
   - Angle 2: Technical details
   - Angle 3: Use cases and examples
   - Angle 4: Alternatives and comparisons
   - Angle 5: Future trends and developments
3. Generate search queries for each angle

\`\`\`
Query: {{query}}
Depth: {{depth}}
\`\`\`

## Phase 2: Search

Execute 5 parallel WebSearch Agents:

1. Launch 5 WebSearch Agents simultaneously
2. Each agent searches one angle
3. Collect top 10 results per angle
4. Aggregate results

\`\`\`
Agents: 5 parallel WebSearch Agents
Results: 50 initial results
\`\`\`

## Phase 3: Fetch

Fetch and deduplicate sources:

1. Deduplicate URLs across all angles
2. Filter by relevance and authority
3. Fetch top {{sources}} sources (default: 15)
4. Extract key content from each source

\`\`\`
Max Sources: {{sources}}
Strategy: Relevance + Authority ranking
\`\`\`

## Phase 4: Verify

Adversarial verification with 3-vote system:

1. Launch 3 verification agents
2. Each agent validates findings
3. Only confirmed facts (2/3 votes) included
4. Contradictions flagged for review

\`\`\`
Voting: 3-vote adversarial
Threshold: 2/3 agreement required
\`\`\`

## Phase 5: Synthesize

Merge results into final report:

1. Organize findings by theme
2. Structure report with sections:
   - Executive Summary
   - Key Findings
   - Detailed Analysis
   - Sources
   - Limitations
3. Apply markdown formatting

\`\`\`
Format: Markdown report
Sections: Summary, Findings, Analysis, Sources, Limitations
\`\`\`

## Output Format

Returns a markdown report:

\`\`\`markdown
# Research Report: [Query]

## Executive Summary

[1-2 paragraph summary of key findings]

## Key Findings

1. [Finding 1 with citation]
2. [Finding 2 with citation]
...

## Detailed Analysis

### [Topic 1]

[Detailed analysis with citations]

### [Topic 2]

[Detailed analysis with citations]

## Sources

1. [Source 1 title](url)
2. [Source 2 title](url)
...

## Limitations

- [Limitation 1]
- [Limitation 2]
\`\`\`

## Usage

\`\`\`typescript
// Standard research
await run_skill({
  skill: "deep-research",
  params: {
    query: "What are the best practices for implementing RAG in LLM applications?",
    depth: 3,
    sources: 15
  }
});

// Deep research with more sources
await run_skill({
  skill: "deep-research",
  params: {
    query: "Compare React, Vue, and Svelte performance characteristics",
    depth: 5,
    sources: 30
  }
});
\`\`\`
`;

export const DEEP_RESEARCH_SKILL_DEFINITION: SkillDefinition = {
  metadata: {
    name: "deep-research",
    description:
      "Multi-phase deep research with parallel search and adversarial verification. Generates comprehensive research reports.",
    whenToUse: [
      "Deep research on technical topics",
      "Competitive analysis",
      "Technology trend investigation",
      "Comprehensive literature review",
    ],
    version: "1.0.0",
    author: "Mnemosyne Team",
    tags: ["research", "analysis", "web-search"],
  },
  phases: [
    {
      name: "Scope",
      instructions: `Decompose the research question into 5 search angles:

1. Analyze the query for key concepts
2. Identify 5 distinct search angles:
   - Angle 1: Broad overview
   - Angle 2: Technical details
   - Angle 3: Use cases and examples
   - Angle 4: Alternatives and comparisons
   - Angle 5: Future trends and developments
3. Generate search queries for each angle

Query: {{query}}
Depth: {{depth}}`,
      type: "discover",
      timeoutMs: 30000,
    },
    {
      name: "Search",
      instructions: `Execute 5 parallel WebSearch Agents:

1. Launch 5 WebSearch Agents simultaneously
2. Each agent searches one angle
3. Collect top 10 results per angle
4. Aggregate results

Agents: 5 parallel WebSearch Agents
Results: 50 initial results`,
      type: "deliver",
      timeoutMs: 60000,
    },
    {
      name: "Fetch",
      instructions: `Fetch and deduplicate sources:

1. Deduplicate URLs across all angles
2. Filter by relevance and authority
3. Fetch top {{sources}} sources (default: 15)
4. Extract key content from each source

Max Sources: {{sources}}
Strategy: Relevance + Authority ranking`,
      type: "deliver",
      timeoutMs: 90000,
    },
    {
      name: "Verify",
      instructions: `Adversarial verification with 3-vote system:

1. Launch 3 verification agents
2. Each agent validates findings
3. Only confirmed facts (2/3 votes) included
4. Contradictions flagged for review

Voting: 3-vote adversarial
Threshold: 2/3 agreement required`,
      type: "verify",
      timeoutMs: 60000,
    },
    {
      name: "Synthesize",
      instructions: `Merge results into final report:

1. Organize findings by theme
2. Structure report with sections:
   - Executive Summary
   - Key Findings
   - Detailed Analysis
   - Sources
   - Limitations
3. Apply markdown formatting

Format: Markdown report
Sections: Summary, Findings, Analysis, Sources, Limitations`,
      type: "persist",
      timeoutMs: 30000,
    },
  ],
  outputFormat: "markdown",
  parameters: {
    query: {
      type: "string",
      description: "Research query or question",
      required: true,
    },
    depth: {
      type: "number",
      description: "Search depth (1-5)",
      required: false,
      default: 3,
      min: 1,
      max: 5,
    },
    sources: {
      type: "number",
      description: "Maximum number of sources to include",
      required: false,
      default: 15,
      min: 5,
      max: 50,
    },
  },
};

export function getDeepResearchSkillDefinition(): SkillDefinition {
  return DEEP_RESEARCH_SKILL_DEFINITION;
}