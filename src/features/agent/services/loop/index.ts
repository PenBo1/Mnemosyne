// Loop-Engineering 模块导出

export * from "./loop-patterns";
export * from "./loop-state";
export * from "./loop-runner";
export * from "./loop-tools";

// ── LoopPanel 面板 service（封装 loop_* IPC） ──────────────────
//
// 将 LoopPanel 组件内直调的 loop_list / loop_create / loop_update / loop_stop /
// loop_delete IPC 下沉到此。组件改为调用以下 service 函数。
// 类型从 @/features/loop/types 导入（面板展示类型），与上方 loop-patterns 重导出
// 的后端 DTO 类型（同名 LoopState/LoopStatus）不同，故此处用别名区分。

import { ipc, ipcVoid } from "@/services/ipc";
import type {
  PanelLoopState,
  TriggerType as PanelTriggerType,
} from "@/features/loop/types";

export interface LoopTriggerConfig {
  intervalMs?: number;
  eventType?: string;
  eventFilter?: string;
}

export async function listLoops(sessionId: string): Promise<PanelLoopState[]> {
  return ipc<PanelLoopState[]>("loop_list", { sessionId });
}

export type CreateLoopParams = {
  sessionId: string;
  name: string;
  triggerType: PanelTriggerType;
  triggerConfig: LoopTriggerConfig;
  maxIterations: number;
};

export async function createLoop(params: CreateLoopParams): Promise<void> {
  await ipcVoid("loop_create", params);
}

export type UpdateLoopParams = {
  sessionId: string;
  loopId: string;
  name: string;
  triggerType: PanelTriggerType;
  triggerConfig: LoopTriggerConfig;
  maxIterations: number;
};

export async function updateLoop(params: UpdateLoopParams): Promise<void> {
  await ipcVoid("loop_update", params);
}

export async function stopLoop(sessionId: string, loopId: string): Promise<void> {
  await ipcVoid("loop_stop", { sessionId, loopId });
}

export async function deleteLoop(sessionId: string, loopId: string): Promise<void> {
  await ipcVoid("loop_delete", { sessionId, loopId });
}
