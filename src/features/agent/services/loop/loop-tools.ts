// Loop 工具定义 —— Agent 可调用的 loop 工具。
//
// 工具列表:
// - schedule_wakeup: 设置定时唤醒
// - arm_monitor: 监听事件
// - task_stop: 停止 Loop
//
// 这些工具会被 Rust 侧的 agent engine 调用。

export interface LoopToolSchema {
  name: string;
  description: string;
  parameters: {
    type: "object";
    properties: Record<string, unknown>;
    required?: string[];
  };
}

export const LOOP_TOOLS: LoopToolSchema[] = [
  {
    name: "schedule_wakeup",
    description: "Schedule a timed wakeup for a loop. The loop will tick at the specified interval.",
    parameters: {
      type: "object",
      properties: {
        stateId: {
          type: "string",
          description: "The loop state ID to schedule",
        },
        intervalMs: {
          type: "number",
          description: "Interval in milliseconds between ticks",
        },
        maxIterations: {
          type: "number",
          description: "Maximum number of iterations before stopping",
        },
      },
      required: ["stateId", "intervalMs"],
    },
  },
  {
    name: "arm_monitor",
    description: "Arm an event monitor for a loop. The loop will tick when the specified event occurs.",
    parameters: {
      type: "object",
      properties: {
        stateId: {
          type: "string",
          description: "The loop state ID to monitor",
        },
        eventType: {
          type: "string",
          enum: ["file_change", "git_commit", "chapter_complete"],
          description: "The type of event to monitor",
        },
        filter: {
          type: "object",
          description: "Optional filter criteria for the event",
        },
      },
      required: ["stateId", "eventType"],
    },
  },
  {
    name: "task_stop",
    description: "Stop a running loop. The loop will be paused and can be resumed later.",
    parameters: {
      type: "object",
      properties: {
        stateId: {
          type: "string",
          description: "The loop state ID to stop",
        },
        reason: {
          type: "string",
          description: "Optional reason for stopping",
        },
      },
      required: ["stateId"],
    },
  },
];

export async function executeLoopTool(
  name: string,
  args: Record<string, unknown>
): Promise<string> {
  const { scheduleWakeup, armMonitor, taskStop } = await import("./loop-runner");

  switch (name) {
    case "schedule_wakeup": {
      const result = await scheduleWakeup({
        stateId: args.stateId as string,
        intervalMs: args.intervalMs as number,
        maxIterations: args.maxIterations as number | undefined,
      });
      return JSON.stringify(result);
    }
    case "arm_monitor": {
      const result = await armMonitor({
        stateId: args.stateId as string,
        eventType: args.eventType as "file_change" | "git_commit" | "chapter_complete",
        filter: args.filter as Record<string, unknown> | undefined,
      });
      return JSON.stringify(result);
    }
    case "task_stop": {
      await taskStop({
        stateId: args.stateId as string,
        reason: args.reason as string | undefined,
      });
      return JSON.stringify({ success: true });
    }
    default:
      throw new Error(`Unknown loop tool: ${name}`);
  }
}