/**
 * 事件通道注册表。
 *
 * 集中管理 Tauri 事件名，避免散落字符串字面量导致的拼写错误和重命名遗漏。
 * 新增领域事件时，在此处登记。
 */
export const EventChannels = {
  /** Agent 流式事件：TurnStarted/StreamDelta/ToolCallBegin/ToolCallEnd/TurnCompleted/Error/CompactionTriggered */
  AgentEvent: "agent-event",
  /** 自主 Agent 进度事件：Progress/ConfirmationRequired/Completed/Failed */
  MainAgentProgress: "main-agent:progress",
} as const;

export type EventChannelName = (typeof EventChannels)[keyof typeof EventChannels];
