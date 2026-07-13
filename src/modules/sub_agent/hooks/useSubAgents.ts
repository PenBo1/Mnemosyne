import { create } from "zustand";
import { toast } from "sonner";
import * as subAgentService from "@/features/sub_agent/services";
import type { SubAgentInfo, SubAgentResult } from "@/shared/types";

/**
 * 子 Agent 状态管理（Zustand store）。
 *
 * 设计参考 `useGit.ts`：
 * - 通过 service 层调用 IPC，不直接 `invoke`
 * - 错误用 sonner toast 反馈
 * - `loading` 仅覆盖显式加载动作，轮询不触发 loading 闪烁
 *
 * 实时更新策略：当前后端未推送子 Agent 状态变更事件，`subscribe()` 采用轮询
 * （每 2s 刷新一次）。若后续后端补充 `sub_agent:changed` 事件，可切换为 listen。
 */

interface SubAgentsState {
  /** 当前会话的所有子 Agent（按 startedAt 倒序，最新在前）。 */
  subAgents: SubAgentInfo[];
  loading: boolean;
  /** 当前选中的 task_id（用于详情面板）。 */
  selectedTaskId: string | null;
  /**
   * 当前选中子 Agent 的结果（output/artifacts/error/durationMs）。
   *
   * 注意：当前 IPC 命令不直接返回 `SubAgentResult`（结果走工具回传给主 Agent），
   * 此字段保留为类型锚点，目前恒为 null，待后续后端暴露结果查询命令后填充。
   */
  selectedResult: SubAgentResult | null;
  /** 轮询定时器 ID（用于在 unsubscribe 时清理）。 */
  _pollTimer: ReturnType<typeof setInterval> | null;
  /** 当前轮询绑定的 session_id，用于在切换会话时重建定时器。 */
  _pollingSessionId: string | null;
  error: string | null;

  /** 加载指定会话的子 Agent 列表。 */
  loadAgents: (sessionId: string) => Promise<void>;
  /** 选中一个子 Agent（taskId 为 null 表示取消选中）。 */
  selectAgent: (taskId: string | null) => void;
  /** 取消运行中的子 Agent。 */
  cancelAgent: (taskId: string) => Promise<boolean>;
  /** 刷新当前列表（需要已加载过的 sessionId）。 */
  refresh: () => Promise<void>;
  /** 订阅指定会话的实时更新（轮询）。返回 unsubscribe 函数。 */
  subscribe: (sessionId: string) => () => void;
  /** 重置 store（切换会话或卸载时调用）。 */
  reset: () => void;
}

function getErrorText(err: unknown, fallback: string): string {
  return err instanceof Error ? err.message : fallback;
}

/** 按启动时间倒序排序（最新 spawn 的子 Agent 排在最前）。 */
function sortByStartedAtDesc(items: SubAgentInfo[]): SubAgentInfo[] {
  return [...items].sort((a, b) => b.startedAt.localeCompare(a.startedAt));
}

const POLL_INTERVAL_MS = 2000;

export const useSubAgents = create<SubAgentsState>((set, get) => ({
  subAgents: [],
  loading: false,
  selectedTaskId: null,
  selectedResult: null,
  _pollTimer: null,
  _pollingSessionId: null,
  error: null,

  loadAgents: async (sessionId: string) => {
    set({ loading: true, error: null });
    try {
      const agents = await subAgentService.listSubAgents(sessionId);
      set({ subAgents: sortByStartedAtDesc(agents), loading: false });
    } catch (err) {
      const msg = getErrorText(err, "Failed to load sub-agents");
      set({ loading: false, error: msg });
      toast.error(msg);
    }
  },

  selectAgent: (taskId: string | null) => {
    set({ selectedTaskId: taskId, selectedResult: null });
  },

  cancelAgent: async (taskId: string) => {
    set({ loading: true, error: null });
    try {
      await subAgentService.cancelSubAgent(taskId);
      // 立即刷新本地列表以反映状态变化（Cancelled）
      await get().refresh();
      return true;
    } catch (err) {
      const msg = getErrorText(err, "Failed to cancel sub-agent");
      set({ loading: false, error: msg });
      toast.error(msg);
      return false;
    }
  },

  refresh: async () => {
    const sessionId = get()._pollingSessionId;
    if (!sessionId) return;
    try {
      const agents = await subAgentService.listSubAgents(sessionId);
      set({ subAgents: sortByStartedAtDesc(agents) });
    } catch (err) {
      // 轮询失败不弹 toast（避免噪音），仅写入 error 供调试
      set({ error: getErrorText(err, "Failed to refresh sub-agents") });
    }
  },

  subscribe: (sessionId: string) => {
    // 切换会话时清理旧定时器
    const prevTimer = get()._pollTimer;
    if (prevTimer) {
      clearInterval(prevTimer);
    }

    // 首次订阅立即加载一次
    void get().loadAgents(sessionId);

    const timer = setInterval(() => {
      void get().refresh();
    }, POLL_INTERVAL_MS);

    set({ _pollTimer: timer, _pollingSessionId: sessionId });

    return () => {
      const current = get()._pollTimer;
      if (current) {
        clearInterval(current);
      }
      set({ _pollTimer: null, _pollingSessionId: null });
    };
  },

  reset: () => {
    const timer = get()._pollTimer;
    if (timer) {
      clearInterval(timer);
    }
    set({
      subAgents: [],
      loading: false,
      selectedTaskId: null,
      selectedResult: null,
      _pollTimer: null,
      _pollingSessionId: null,
      error: null,
    });
  },
}));
