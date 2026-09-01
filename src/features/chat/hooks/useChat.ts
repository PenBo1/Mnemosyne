import { useCallback, useEffect, useRef } from "react";
import { toast } from "sonner";
import { useAgentStore } from "@/features/chat/store";
import { useWorkspaceStore } from "@/features/workspace/store/workspace";
import { readFile } from "@/services/storage/fs";
import { getWikiEntry } from "@/features/wiki/services";
import { getCustomInstructions, getActiveModel } from "@/services/settings";
import {
  sendMessage as chatSendMessage,
  setCurrentWorkspacePath,
  stopChat,
} from "@/features/agent/services/llm/chat-runtime";
import { respondApproval } from "@/features/agent/services/approval";
import { useAgentsStore } from "@/features/agent/store/agents-store";
import { usePlanStore } from "@/features/agent/store/plan-store";
import type { Message, Session, AttachmentSpec } from "@/features/chat/types";
import {
  createInitialPipelineState,
  updatePipelineStep,
  advanceToNextStep,
  type PipelinePhase,
} from "@/features/agent/components/PipelineProgress";
import { listen } from "@tauri-apps/api/event";
import { useI18n, type Translations } from "@/locales/i18n";

/** 解析附件为注入 LLM 上下文的文本块。
 *  - file: 读取文件内容
 *  - wiki: 按 entry_id 查询 wiki 条目内容
 *  - chapter: 按 <workspace>/chapters/<n>.md 读取章节文件
 *  - text: 直接取 content 字段
 *  解析失败的附件跳过并 toast 提示，不阻断发送。 */
async function resolveAttachments(
  attachments: AttachmentSpec[],
  workspacePath: string | null,
  t: Translations,
): Promise<string | undefined> {
  const blocks: string[] = [];
  for (const att of attachments) {
    try {
      let text: string;
      let kindLabel: string;
      switch (att.kind) {
        case "file":
          text = await readFile(att.ref);
          kindLabel = t.chat.attachment.kind.file;
          break;
        case "wiki": {
          const entry = await getWikiEntry(att.ref);
          text = entry?.content ?? "";
          kindLabel = t.chat.attachment.kind.wiki;
          break;
        }
        case "chapter": {
          if (!workspacePath) {
            toast.error(t.chat.attachment.noWorkspace);
            continue;
          }
          const filePath = `${workspacePath}/chapters/${att.ref}.md`;
          text = await readFile(filePath);
          kindLabel = t.chat.attachment.kind.chapter;
          break;
        }
        case "text":
          text = att.content ?? "";
          kindLabel = t.chat.attachment.kind.text;
          break;
        default:
          continue;
      }
      if (text.trim()) {
        blocks.push(`【${kindLabel}：${att.label}】\n${text}`);
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : t.chat.errors.unknown;
      toast.error(
        t.chat.attachment.parseFailed
          .replace("{label}", att.label)
          .replace("{error}", msg),
      );
    }
  }
  if (blocks.length === 0) return undefined;
  return `${t.chat.attachment.referenceContext}\n\n${blocks.join("\n\n")}`;
}

/**
 * 页面级 Chat hook：组合会话列表 + 当前会话消息 + 流式发消息流程。
 *
 * - 会话列表 / 当前 sessionId / messages / streaming 均从 useAgentStore 读
 * - chat-runtime 直接更新 store（streamText + fullStream 迭代），无 Rust agent-event 订阅
 * - sendMessage 自行实现，处理"无 session 时先创建"的流程，绑定 active workspaceId
 */
export function useChat() {
  const { t } = useI18n();
  const sessions = useAgentStore((s) => s.sessions);
  const currentSessionId = useAgentStore((s) => s.currentSessionId);
  const loading = useAgentStore((s) => s.loading);
  const loadSessions = useAgentStore((s) => s.loadSessions);
  const switchSession = useAgentStore((s) => s.switchSession);
  const createSession = useAgentStore((s) => s.createSession);
  const deleteSession = useAgentStore((s) => s.deleteSession);
  const clearCurrentSession = useAgentStore((s) => s.clearCurrentSession);
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspaceId);
  const workspaces = useWorkspaceStore((s) => s.workspaces);

  // chat-runtime 直接更新 store，这里直接读 store 即可（无需事件订阅层）
  const messages = useAgentStore((s) => s.messages);
  const streaming = useAgentStore((s) => s.streaming);
  // streamingContent/streamingReasoning 不在此订阅 —— 移到 MessageList 内部订阅，
  // 避免 ChatPage 因每帧 delta 变化重渲染（ChatTopBar/ChatInput 等不需要重渲染）。
  const error = useAgentStore((s) => s.error);

  // 写操作 action（通过 selector，避免整树订阅）
  const appendMessage = useAgentStore((s) => s.appendMessage);
  const setStreaming = useAgentStore((s) => s.setStreaming);
  const clearStreamingContent = useAgentStore((s) => s.clearStreamingContent);
  const setError = useAgentStore((s) => s.setError);

  // 跟踪 latest currentSessionId，避免 sendMessage 闭包陈旧
  const currentSessionIdRef = useRef(currentSessionId);
  currentSessionIdRef.current = currentSessionId;

  // 跟踪 latest activeWorkspaceId，避免闭包陈旧
  const activeWorkspaceIdRef = useRef(activeWorkspaceId);
  activeWorkspaceIdRef.current = activeWorkspaceId;

  // 当前活动工作区路径 (用于解析 chapter 附件)
  const activeWorkspace = workspaces.find((w) => w.id === activeWorkspaceId) ?? null;
  const workspacePath = activeWorkspace?.path ?? null;

  // 首次挂载加载会话列表（绑定当前活动工作区）
  useEffect(() => {
    loadSessions(undefined, activeWorkspaceId ?? undefined);
    // P1 阶段 3: hydrate custom agents store（builtin + custom + activeId）
    void useAgentsStore.getState().hydrate();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 监听 Pipeline 子 Agent 步骤事件
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    (async () => {
      const fn = await listen<{
        phase: PipelinePhase;
        subagentName: string;
        chapter: number;
        status: "running" | "completed" | "error";
      }>("subagent-step", (event) => {
        const { phase, subagentName, chapter, status } = event.payload;
        const currentState = useAgentStore.getState().pipelineState;

        if (!currentState || currentState.currentChapter !== chapter) {
          const newState = createInitialPipelineState(chapter);
          newState.steps[0] = {
            phase,
            subagentName,
            status,
            startedAt: status === "running" ? Date.now() : undefined,
            completedAt: status === "completed" ? Date.now() : undefined,
          };
          newState.currentStepIndex = 0;
          useAgentStore.getState().setPipelineState(newState);
        } else {
          const stepIndex = currentState.steps.findIndex((s) => s.phase === phase);
          if (stepIndex >= 0) {
            const updated = updatePipelineStep(currentState, stepIndex, {
              status,
              subagentName,
              completedAt: status === "completed" ? Date.now() : undefined,
            });
            if (status === "completed") {
              const next = advanceToNextStep(updated);
              useAgentStore.getState().setPipelineState(next);
            } else {
              useAgentStore.getState().setPipelineState(updated);
            }
          }
        }
      });
      if (cancelled) {
        fn();
        return;
      }
      unlisten = fn;
    })();

    return () => {
      cancelled = true;
      if (unlisten) unlisten();
    };
  }, []);

  // 监听 activeWorkspaceId 变化：重载该工作区下的会话列表
  useEffect(() => {
    loadSessions(undefined, activeWorkspaceId ?? undefined);
  }, [activeWorkspaceId, loadSessions]);

  // 用户期望：默认打开空白对话页面，不自动加载历史会话
  // 直接输入内容会自动创建新会话，或点击侧边栏选择历史会话
  // 因此保持 currentSessionId = null，不自动选择历史会话

  const sendMessage = useCallback(
    async (content: string, attachments?: AttachmentSpec[]) => {
      const trimmed = content.trim();
      if (!trimmed) return;

      const activeModel = await getActiveModel();
      if (!activeModel) {
        const msg = t.chat.errors.noModelConfigured;
        toast.error(msg, {
          action: {
            label: t.chat.actions.goToSettings,
            onClick: () => {
              window.location.href = "#/settings?tab=ai";
            },
          },
        });
        return;
      }

      // 无 session 时先创建（拿到真实 id 后再发），绑定 active workspace
      let sid = currentSessionIdRef.current;
      if (!sid) {
        try {
          const session = await createSession(undefined, undefined, activeWorkspaceIdRef.current ?? undefined);
          sid = session.id;
        } catch {
          return; // createSession 已通过 toast 报错
        }
      }

      // 解析附件为上下文文本 (失败不阻断发送)
      const contextText = attachments && attachments.length > 0
        ? await resolveAttachments(attachments, workspacePath, t)
        : undefined;

      setError(null);
      clearStreamingContent();
      setStreaming(true);

      // 乐观追加 user 消息
      const userMessage: Message = {
        id: `temp-${Date.now()}`,
        session_id: sid,
        role: "user",
        content: trimmed,
        tool_calls: null,
        tool_results: null,
        token_count: null,
        created_at: new Date().toISOString(),
      };
      appendMessage(userMessage);

      try {
        // P1: 注入工作区路径（chat-runtime 用于 Rust 端 AGENTS.md + env）
        setCurrentWorkspacePath(workspacePath);
        const customInstructions = await getCustomInstructions();
        // P2.8: 读取当前激活 agent 的工具白名单，随 IPC request 传递给后端。
        // 后端 SendMessageRequest 暂未声明 tool_whitelist 字段（serde 忽略未知字段），
        // 当前为协议预声明 —— 后端补字段后即可启用 per-agent 工具限制，前端无需再改。
        // filterToolsByWhitelist 辅助函数（agents.ts）用于前端工具列表展示场景，
        // 此处仅传递白名单，实际过滤由后端 agent engine 执行。
        const toolWhitelist = useAgentsStore.getState().active().toolWhitelist;
        await chatSendMessage(sid, trimmed, contextText, customInstructions, toolWhitelist);
      } catch (err) {
        setStreaming(false);
        const msg = err instanceof Error ? err.message : "Failed to send message";
        setError(msg);
        toast.error(msg);
      }
    },
    [createSession, appendMessage, clearStreamingContent, setStreaming, setError, workspacePath, t],
  );

  const handleNewSession = useCallback(() => {
    // 仅进入空白页，不创建会话（输入消息时才创建）
    clearCurrentSession();
  }, [clearCurrentSession]);

  const handleDeleteSession = useCallback(async () => {
    if (!currentSessionId) return;
    await deleteSession(currentSessionId);
  }, [currentSessionId, deleteSession]);

  // cancel — 对接 chat-runtime.stopChat（中断 streamText 流）
  const cancel = useCallback(() => {
    if (!currentSessionId) return;
    stopChat(currentSessionId);
  }, [currentSessionId]);

  // 重新生成：找到最后一条 user 消息，作为新 turn 重发 (追加新回复，无孤立数据)
  const regenerate = useCallback(() => {
    if (streaming) return;
    for (let i = messages.length - 1; i >= 0; i--) {
      if (messages[i].role === "user") {
        void sendMessage(messages[i].content);
        return;
      }
    }
  }, [messages, streaming, sendMessage]);

  const activeSession: Session | null =
    sessions.find((s) => s.id === currentSessionId) ?? null;

  // P1 阶段 3: approval + plan mode 状态
  const pendingConfirmation = useAgentStore((s) => s.pendingConfirmation);
  const submittingConfirmation = useAgentStore((s) => s.submittingConfirmation);
  const planModeActive = usePlanStore((s) => s.active);
  const planQueue = usePlanStore((s) => s.queue);
  const togglePlanMode = usePlanStore((s) => s.toggle);
  const planApplyAll = usePlanStore((s) => s.applyAll);
  const planRemoveOne = usePlanStore((s) => s.removeOne);
  const planClear = usePlanStore((s) => s.clear);

  // Pipeline 进度状态
  const pipelineState = useAgentStore((s) => s.pipelineState);
  const setPipelineState = useAgentStore((s) => s.setPipelineState);
  const clearPipelineState = useAgentStore((s) => s.clearPipelineState);

  // P1 阶段 3: approval 响应 —— UI Approve/Deny 按钮调此方法
  const handleRespondApproval = useCallback(
    (approvalId: string, approved: boolean) => {
      useAgentStore.getState().setSubmittingConfirmation(true);
      const sid = currentSessionIdRef.current;
      if (sid) {
        respondApproval(sid, approvalId, approved);
      }
    },
    [],
  );

  // 清空当前会话消息列表
  const clearMessages = useCallback(() => {
    useAgentStore.getState().replaceMessages([]);
  }, []);

  return {
    sessions,
    currentSessionId,
    activeSession,
    messages,
    streaming,
    error,
    loading,
    workspacePath,
    sendMessage,
    cancel,
    regenerate,
    switchSession,
    handleNewSession,
    handleDeleteSession,
    clearMessages,
    // P1 阶段 3: approval + plan mode
    pendingConfirmation,
    submittingConfirmation,
    respondApproval: handleRespondApproval,
    planModeActive,
    planQueue,
    togglePlanMode,
    planApplyAll,
    planRemoveOne,
    planClear,
    // Pipeline 进度
    pipelineState,
    setPipelineState,
    clearPipelineState,
  };
}
