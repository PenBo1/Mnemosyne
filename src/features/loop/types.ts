// Loop 类型聚合 —— 同时导出引擎类型（与后端 DTO 对齐）与 LoopPanel 面板展示类型。
//
// 引擎类型（LoopState/LoopPattern/LoopRunLog/CreateLoopStateRequest 等）来自
// ./types/loop，与后端 application/loop_engine/types.rs 的
// #[serde(rename_all = "camelCase")] DTO 对齐。
//
// 面板展示类型（PanelLoopState/TriggerType）仅供 LoopPanel 与 loop panel service
// 使用，是简化的展示结构；与引擎 LoopState 同名会冲突，故以 PanelLoopState 区分。

export * from "./types/loop";
import type { LoopStatus } from "./types/loop";

export type TriggerType = "timer" | "event";

export interface PanelLoopState {
  id: string;
  name: string;
  triggerType: TriggerType;
  triggerConfig: {
    intervalMs?: number;
    eventType?: string;
    eventFilter?: string;
  };
  status: LoopStatus;
  nextWakeTime?: string;
  lastRunTime?: string;
  iterations: number;
  maxIterations?: number;
  createdAt: string;
}
