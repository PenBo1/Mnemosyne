import { useCallback, useMemo } from "react";
import { useMainAgentStore } from "@/stores/agent";
import { useEventSubscription, EventChannels } from "@/infrastructure/event_bus";
import type { MainAgentEvent } from "@/shared/types/main-agent";

/**
 * 自主 Agent hook。
 *
 * 替代此前 MainAgentPage 直接 import store + 手写事件监听的违规（R3）。
 *
 * 职责：
 * - 暴露 store 的状态和 actions 给页面
 * - 内部用 lib/events 统一订阅 main-agent:progress 事件（替代 services/main-agent
 *   里的 listenToMainAgentEvents，消除其动态 import + 竞态泄漏）
 *
 * 性能：使用选择性订阅（selector），避免流式事件触发整树重渲染。
 * 页面禁止直接 import @/stores/agent，必须通过本 hook。
 */
export function useMainAgent() {
  // 状态字段：每个字段独立订阅，避免无关字段变更触发重渲染
  const sessions = useMainAgentStore((s) => s.sessions);
  const activeSessionId = useMainAgentStore((s) => s.activeSessionId);
  const loading = useMainAgentStore((s) => s.loading);

  // actions：Zustand 中 action 引用稳定，独立订阅不会触发重渲染
  const handleEvent = useMainAgentStore((s) => s.handleEvent);
  const startExecution = useMainAgentStore((s) => s.startExecution);
  const respondToConfirmation = useMainAgentStore((s) => s.respondToConfirmation);
  const cancelExecution = useMainAgentStore((s) => s.cancelExecution);
  const setActiveSession = useMainAgentStore((s) => s.setActiveSession);

  // 事件订阅：流式 Progress 事件通过统一事件总线分发到 store
  useEventSubscription<MainAgentEvent>(
    EventChannels.MainAgentProgress,
    (event) => {
      handleEvent(event);
    },
  );

  // 派生：当前活动会话。仅当 sessions 或 activeSessionId 变化时重算
  const activeSession = useMemo(
    () => (activeSessionId ? sessions[activeSessionId] ?? null : null),
    [sessions, activeSessionId],
  );

  // 包装 actions：保持稳定引用，避免传递给子组件时触发重渲染
  const start = useCallback((goal: string) => startExecution(goal), [startExecution]);
  const respond = useCallback(
    (approved: boolean, modifiedArgs?: string) =>
      respondToConfirmation(approved, modifiedArgs),
    [respondToConfirmation],
  );
  const cancel = useCallback(() => cancelExecution(), [cancelExecution]);
  const setActive = useCallback((id: string | null) => setActiveSession(id), [setActiveSession]);

  return {
    sessions,
    activeSessionId,
    loading,
    activeSession,
    startExecution: start,
    respondToConfirmation: respond,
    cancelExecution: cancel,
    setActiveSession: setActive,
  };
}
