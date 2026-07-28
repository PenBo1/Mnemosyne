/**
 * @deprecated 新代码请从 @/types/session 导入，本文件仅为兼容保留。
 *
 * Session & Agent 事件类型已上移到 @/types/session（打破 chat ↔ agent 与
 * chat ↔ session 循环依赖）。此处显式 re-export 保持 barrel 向后兼容：
 * chat 内部用 `@/features/chat/types` 仍可拿到这些类型，后续将被移除。
 */

/** @deprecated 请改从 @/types/session 导入 */
export type {
  Session,
  Message,
  AgentEvent,
  PendingConfirmation,
} from "@/types/session";

/** @deprecated 请改从 @/features/chat/types/attachment 导入 */
export * from "./attachment";

/** @deprecated 请改从 @/features/chat/types/audit 导入 */
export * from "./audit";
