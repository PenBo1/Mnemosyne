// 工具调用协议 —— 纯逻辑内核
// 工具调用状态机：pending → approved/rejected → running → succeeded/failed
//
// 纯函数实现，无 React/Tauri/IPC 依赖。被 hooks/agent 和 stores/agent 共用。

export type ToolCallStatus = "pending" | "approved" | "rejected" | "running" | "succeeded" | "failed";

export interface ToolCallState {
  id: string;
  name: string;
  status: ToolCallStatus;
  /** 工具调用参数（序列化后的字符串，便于展示与持久化） */
  args?: string;
  /** 工具执行结果（仅 succeeded 状态有值） */
  result?: unknown;
  /** 工具执行错误（仅 failed 状态有值） */
  error?: string;
  /** 上一次状态变更的时间戳（ms） */
  updatedAt: number;
}

export type ToolCallAction =
  | { type: "approve" }
  | { type: "reject" }
  | { type: "start" }
  | { type: "succeed"; result: unknown }
  | { type: "fail"; error: string };

/**
 * 工具调用状态转移函数（纯函数）
 *
 * 非法转移（如 succeeded → approved）返回原状态，不抛错，
 * 让调用方在 UI 层决定如何提示用户（避免纯逻辑层抛错）。
 */
export function transitionToolCall(current: ToolCallState, action: ToolCallAction): ToolCallState {
  const now = Date.now();

  switch (action.type) {
    case "approve":
      // 仅 pending 可批准
      if (current.status !== "pending") return current;
      return { ...current, status: "approved", updatedAt: now };

    case "reject":
      // 仅 pending 可拒绝
      if (current.status !== "pending") return current;
      return { ...current, status: "rejected", updatedAt: now };

    case "start":
      // 仅 approved 可启动（自动批准的场景由调用方先 approve 再 start）
      if (current.status !== "approved") return current;
      return { ...current, status: "running", updatedAt: now };

    case "succeed":
      // 仅 running 可成功
      if (current.status !== "running") return current;
      return { ...current, status: "succeeded", result: action.result, error: undefined, updatedAt: now };

    case "fail":
      // 仅 running 可失败
      if (current.status !== "running") return current;
      return { ...current, status: "failed", error: action.error, result: undefined, updatedAt: now };

    default: {
      // exhaustiveness check — 若新增 action 类型未处理，编译期报错
      const _exhaustive: never = action;
      void _exhaustive;
      return current;
    }
  }
}

/** 创建初始 ToolCallState */
export function createToolCallState(id: string, name: string, args?: string): ToolCallState {
  return {
    id,
    name,
    status: "pending",
    args,
    updatedAt: Date.now(),
  };
}
