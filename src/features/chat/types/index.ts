// Session & Agent 事件类型已上移到 @/types/session（打破 chat ↔ agent 循环依赖）。
// 此处显式 re-export 保持 barrel 向后兼容：chat 内部用 `@/features/chat/types`
// 仍可拿到这些类型。新代码（尤其是 agent 模块）应直接从 `@/types` 导入。
export type {
  Session,
  Message,
  AgentEvent,
  PendingConfirmation,
} from "@/types/session";
export * from "./attachment";
export * from "./audit";
