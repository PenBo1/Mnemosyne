// 流式响应协议 —— 纯逻辑内核，无 React/Tauri/IPC 依赖
// 被 hooks/agent 和 stores/agent 共同依赖，统一两套 agent 系统的流式语义

import type { AgentEvent } from "@/shared/types";

/**
 * 流式事件归一化 reducer：将原始 AgentEvent 序列归约为会话状态。
 * 纯函数，可单测。
 */
export interface StreamState {
  // TODO: 待实现的流式状态
}

export function streamReducer(state: StreamState, _event: AgentEvent): StreamState {
  // TODO: 按 TurnStarted/StreamDelta/ToolCallBegin/ToolCallEnd/TurnCompleted/Error 归约
  return state;
}
