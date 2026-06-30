// 流式响应协议 —— 纯逻辑内核，无 React/Tauri/IPC 依赖
// 被 hooks/agent 和 stores/agent 共同依赖，统一两套 agent 系统的流式语义
//
// 归约规则：
// - TurnStarted：重置 turn 累积文本，记录 token 预算
// - StreamDelta：累积到当前 turn 的 content
// - ToolCallBegin：新增一个 pending 工具调用
// - ToolCallEnd：把对应工具调用标记为 succeeded/failed
// - TurnCompleted：固化本 turn 结果，累加 token 用量
// - Error：记录错误并结束当前 turn
// - CompactionTriggered：标记上下文已压缩

import type { AgentEvent } from "@/shared/types";
import type { ToolCallState } from "./tool-protocol";

export interface StreamState {
  /** 当前会话累计的文本内容（所有已完成 turn + 当前 turn） */
  content: string;
  /** 当前 turn 正在流式累积的文本（TurnCompleted 后合并到 content） */
  currentTurnContent: string;
  /** 工具调用列表（按触发顺序） */
  toolCalls: ToolCallState[];
  /** 累计 token 用量 */
  totalInputTokens: number;
  totalOutputTokens: number;
  /** 最近一次错误（Error 事件） */
  lastError: string | null;
  /** 是否正在流式输出中（TurnStarted 后、TurnCompleted/Error 前） */
  isStreaming: boolean;
  /** 上下文是否已压缩 */
  compacted: boolean;
}

export const initialStreamState: StreamState = {
  content: "",
  currentTurnContent: "",
  toolCalls: [],
  totalInputTokens: 0,
  totalOutputTokens: 0,
  lastError: null,
  isStreaming: false,
  compacted: false,
};

/**
 * 流式事件归一化 reducer：将原始 AgentEvent 序列归约为会话状态。
 * 纯函数，可单测。
 */
export function streamReducer(state: StreamState, event: AgentEvent): StreamState {
  switch (event.type) {
    case "TurnStarted":
      return {
        ...state,
        currentTurnContent: "",
        isStreaming: true,
        lastError: null,
      };

    case "StreamDelta": {
      if (!state.isStreaming) return state;
      const delta = event.content ?? "";
      return {
        ...state,
        currentTurnContent: state.currentTurnContent + delta,
      };
    }

    case "ToolCallBegin": {
      const toolCall: ToolCallState = {
        id: event.tool_call_id ?? `tool-${state.toolCalls.length}`,
        name: event.tool ?? "unknown",
        status: "running",
        args: event.args,
        updatedAt: Date.now(),
      };
      return {
        ...state,
        toolCalls: [...state.toolCalls, toolCall],
      };
    }

    case "ToolCallEnd": {
      const id = event.tool_call_id;
      const isError = event.is_error === true;
      const toolCalls = state.toolCalls.map((tc) => {
        if (tc.id !== id) return tc;
        return isError
          ? { ...tc, status: "failed" as const, error: event.output ?? "Tool failed", updatedAt: Date.now() }
          : { ...tc, status: "succeeded" as const, result: event.output, updatedAt: Date.now() };
      });
      return { ...state, toolCalls };
    }

    case "TurnCompleted": {
      const turnContent = state.currentTurnContent;
      return {
        ...state,
        content: state.content + turnContent,
        currentTurnContent: "",
        isStreaming: false,
        totalInputTokens: state.totalInputTokens + (event.input_tokens ?? 0),
        totalOutputTokens: state.totalOutputTokens + (event.output_tokens ?? 0),
      };
    }

    case "Error":
      return {
        ...state,
        isStreaming: false,
        lastError: event.error ?? event.content ?? "Unknown error",
      };

    case "CompactionTriggered":
      return {
        ...state,
        compacted: true,
      };

    default:
      return state;
  }
}
