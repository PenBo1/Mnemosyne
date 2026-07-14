// 工具执行上限配置 —— 单次对话中工具调用的安全阀。
//
// 后端命令:
// - tool_limits_get → 当前配置
// - tool_limits_update(config) → 更新配置
// - tool_limits_reset → 重置为默认值

import { ipc, ipcVoid } from "@/services/ipc";

export interface ToolLimitsConfig {
  /** 单次对话中工具调用总次数上限(0 = 不限制,默认 50) */
  maxToolCallsPerDialog: number;
  /** 连续失败次数上限(0 = 不限制,默认 5) */
  maxConsecutiveFailures: number;
  /** 单次工具调用超时毫秒(0 = 不限制,默认 60000) */
  timeoutPerCallMs: number;
}

export const DEFAULT_TOOL_LIMITS: ToolLimitsConfig = {
  maxToolCallsPerDialog: 50,
  maxConsecutiveFailures: 5,
  timeoutPerCallMs: 60_000,
};

/** 硬上限(对齐 Rust 的 validate()) */
export const TOOL_LIMITS_BOUNDS = {
  maxToolCallsPerDialog: { min: 0, max: 1000 },
  maxConsecutiveFailures: { min: 0, max: 100 },
  timeoutPerCallMs: { min: 0, max: 600_000 },
} as const;

/** 读取当前配置 */
export function getToolLimits(): Promise<ToolLimitsConfig> {
  return ipc<ToolLimitsConfig>("tool_limits_get");
}

/** 更新配置(写盘 + 刷新缓存) */
export async function updateToolLimits(config: ToolLimitsConfig): Promise<void> {
  await ipcVoid("tool_limits_update", { config });
}

/** 重置为默认值 */
export async function resetToolLimits(): Promise<void> {
  await ipcVoid("tool_limits_reset");
}

/** 校验配置项是否在合理范围内(对齐 Rust 端 validate) */
export function validateToolLimits(config: ToolLimitsConfig): string | null {
  const { maxToolCallsPerDialog, maxConsecutiveFailures, timeoutPerCallMs } = config;
  const b = TOOL_LIMITS_BOUNDS;
  if (maxToolCallsPerDialog > b.maxToolCallsPerDialog.max) {
    return `maxToolCallsPerDialog exceeds hard limit (> ${b.maxToolCallsPerDialog.max})`;
  }
  if (maxConsecutiveFailures > b.maxConsecutiveFailures.max) {
    return `maxConsecutiveFailures exceeds hard limit (> ${b.maxConsecutiveFailures.max})`;
  }
  if (timeoutPerCallMs > b.timeoutPerCallMs.max) {
    return `timeoutPerCallMs exceeds hard limit (> ${b.timeoutPerCallMs.max})`;
  }
  return null;
}
