// Loop Skill —— Loop 工程技能，用于周期性任务执行和事件监听。
//
// Frontmatter 定义:
// - name: loop
// - description: Periodic task execution with event-gated triggers
// - when_to_use: 检查状态、周期性审查、事件驱动任务
//
// Phases:
// - Phase 0: Execute current check (执行当前检查)
// - Phase 1: Arm monitor if event-gated (如需事件触发，启动 Monitor)
// - Phase 2: Call ScheduleWakeup (调用定时唤醒)
//
// Output: 返回执行结果

import type { SkillDefinition } from "../types";

export const LOOP_SKILL_MARKDOWN = `---
name: loop
description: Periodic task execution with event-gated triggers. Supports timer-based, event-based, and hybrid scheduling.
when_to_use:
  - Check system state periodically
  - Monitor file changes or git events
  - Execute tasks on schedule
  - React to specific events
output_format: markdown
version: "1.0.0"
author: Mnemosyne Team
tags:
  - automation
  - scheduling
  - monitoring
parameters:
  patternId:
    type: string
    description: The loop pattern ID to execute
    required: true
  triggerType:
    type: string
    description: Type of trigger (timer, event, hybrid)
    required: true
    enum: [timer, event, hybrid]
  intervalMs:
    type: number
    description: Interval in milliseconds for timer triggers
    required: false
    min: 1000
    max: 86400000
  eventType:
    type: string
    description: Event type to monitor for event triggers
    required: false
    enum: [file_change, git_commit, chapter_complete]
---

# Loop Skill

This skill enables periodic task execution with flexible triggering mechanisms.

## Phase 0: Execute Current Check

Execute the current check defined by the loop pattern:

1. Load the loop pattern by \`patternId\`
2. Evaluate current system state
3. Determine if action is needed
4. If action needed, proceed to Phase 1
5. If no action needed, skip to Phase 2

\`\`\`
Loop Pattern: {{patternId}}
Session: {{sessionId}}
\`\`\`

## Phase 1: Arm Monitor (Event-gated Only)

If the trigger type is \`event\` or \`hybrid\`, arm the event monitor:

1. Call \`arm_monitor\` tool with event type and filter
2. Set up event listener for specified event
3. Monitor will trigger loop tick on event

\`\`\`
Event Type: {{eventType}}
Filter: {{eventFilter}}
\`\`\`

## Phase 2: Schedule Wakeup

Schedule the next loop execution:

1. Call \`schedule_wakeup\` tool with interval
2. Register timed wakeup with scheduler
3. Loop will tick at scheduled time

\`\`\`
Interval: {{intervalMs}}ms
Max Iterations: {{maxIterations}}
\`\`\`

## Output

The skill returns a markdown report with:

- **Status**: success/partial/failed
- **Check Result**: Result of current check
- **Monitor Status**: Armed/Disarmed (for event-gated)
- **Next Wakeup**: Scheduled time for next tick
- **Recommendations**: Any suggested actions

## Usage

\`\`\`typescript
// Timer-based loop (every 5 minutes)
await run_skill({
  skill: "loop",
  params: {
    patternId: "chapter-write-loop",
    triggerType: "timer",
    intervalMs: 300000
  }
});

// Event-based loop (file changes)
await run_skill({
  skill: "loop",
  params: {
    patternId: "audit-revise-loop",
    triggerType: "event",
    eventType: "file_change"
  }
});

// Hybrid loop (timer + event)
await run_skill({
  skill: "loop",
  params: {
    patternId: "observation-loop",
    triggerType: "hybrid",
    intervalMs: 600000,
    eventType: "git_commit"
  }
});
\`\`\`
`;

export const LOOP_SKILL_DEFINITION: SkillDefinition = {
  metadata: {
    name: "loop",
    description:
      "Periodic task execution with event-gated triggers. Supports timer-based, event-based, and hybrid scheduling.",
    whenToUse: [
      "Check system state periodically",
      "Monitor file changes or git events",
      "Execute tasks on schedule",
      "React to specific events",
    ],
    version: "1.0.0",
    author: "Mnemosyne Team",
    tags: ["automation", "scheduling", "monitoring"],
  },
  phases: [
    {
      name: "Execute Current Check",
      instructions: `Execute the current check defined by the loop pattern:

1. Load the loop pattern by {{patternId}}
2. Evaluate current system state
3. Determine if action is needed
4. If action needed, proceed to Phase 1
5. If no action needed, skip to Phase 2

Session: {{sessionId}}
Workspace: {{workspaceId}}`,
      type: "discover",
      timeoutMs: 30000,
    },
    {
      name: "Arm Monitor (Event-gated Only)",
      instructions: `If the trigger type is 'event' or 'hybrid', arm the event monitor:

1. Call arm_monitor tool with event type and filter
2. Set up event listener for specified event
3. Monitor will trigger loop tick on event

Event Type: {{eventType}}
Filter: {{eventFilter}}`,
      type: "deliver",
      timeoutMs: 10000,
    },
    {
      name: "Schedule Wakeup",
      instructions: `Schedule the next loop execution:

1. Call schedule_wakeup tool with interval
2. Register timed wakeup with scheduler
3. Loop will tick at scheduled time

Interval: {{intervalMs}}ms
Max Iterations: {{maxIterations}}`,
      type: "schedule",
      timeoutMs: 5000,
    },
  ],
  outputFormat: "markdown",
  parameters: {
    patternId: {
      type: "string",
      description: "The loop pattern ID to execute",
      required: true,
    },
    triggerType: {
      type: "string",
      description: "Type of trigger (timer, event, hybrid)",
      required: true,
      enum: ["timer", "event", "hybrid"],
    },
    intervalMs: {
      type: "number",
      description: "Interval in milliseconds for timer triggers",
      required: false,
      min: 1000,
      max: 86400000,
    },
    eventType: {
      type: "string",
      description: "Event type to monitor for event triggers",
      required: false,
      enum: ["file_change", "git_commit", "chapter_complete"],
    },
  },
};

export function getLoopSkillDefinition(): SkillDefinition {
  return LOOP_SKILL_DEFINITION;
}