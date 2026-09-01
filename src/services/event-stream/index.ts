/**
 * ═══════════════════════════════════════════════════════════════════════════
 * EventStream - 实时事件流服务
 * ═══════════════════════════════════════════════════════════════════════════
 */

export { useEventStream } from "./useEventStream";
export { EventStreamManager } from "./manager";
export type {
  EventPayload,
  EventSubscription,
  EventStreamConfig,
  EventStreamState,
} from "./types";