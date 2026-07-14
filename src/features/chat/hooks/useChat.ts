import { useCallback, useEffect, useRef } from "react";
import { toast } from "sonner";
import { useAgentStore } from "@/features/chat/store";
import { useWorkspaceStore } from "@/features/workspace/store/workspace";
import { readFile } from "@/services/storage/fs";
import { getWikiEntry } from "@/features/wiki/services";
import { getCustomInstructions } from "@/services/settings";
import {
  sendMessage as chatSendMessage,
  setCurrentWorkspacePath,
  stopChat,
} from "@/features/agent/services/llm/chat-runtime";
import { respondApproval } from "@/features/agent/services/approval";
import { useAgentsStore } from "@/features/agent/store/agents-store";
import { usePlanStore } from "@/features/agent/store/plan-store";
import type { Message, Session, AttachmentSpec } from "@/features/chat/types";

/** 解析附件为注入 LLM 上下文的文本块。
 *  - file: 读取文件内容
 *  - wiki: 按 entry_id 查询 wiki 条目内容
 *  - chapter: 按 <workspace>/chapters/<n>.md 读取章节文件
 *  - text: 直接取 content 字段
 *  解析失败的附件跳过并 toast 提示，不阻断发送。 */
async function resolveAttachments(
  attachments: AttachmentSpec[],
  workspacePath: string | null,
): Promise<string | undefined> {
  const blocks: string[] = [];
  for (const att of attachments) {
    try {
      let text: string;
      let kindLabel: string;
      switch (att.kind) {
        case "file":
          text = await readFile(att.ref);
          kindLabel = "文件";
          break;
        case "wiki": {
          const entry = await getWikiEntry(att.ref);
          text = entry?.content ?? "";
          kindLabel = "Wiki";
          break;
        }
        case "chapter": {
          if (!workspacePath) {
            toast.error(`无法解析章节附件：当前无活动工作区`);
            continue;
          }
          const filePath = `${workspacePath}\\chapters\\${att.ref}.md`;
          text = await readFile(filePath);
          kindLabel = "章节";
          break;
        }
        case "text":
          text = att.content ?? "";
          kindLabel = "文本";
          break;
        default:
          continue;
      }
      if (text.trim()) {
        blocks.push(`【${kindLabel}：${att.label}】\n${text}`);
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : "未知错误";
      toast.error(`附件「${att.label}」解析失败：${msg}`);
    }
  }
  if (blocks.length === 0) return undefined;
  return `以下是用户提供的参考上下文，请结合这些内容回答用户问题：\n\n${blocks.join("\n\n")}`;
}

/**
 * 页面级 Chat hook：组合会话列表 + 当前会话消息 + 流式发消息流程。
 *
 * - 会话列表 / 当前 sessionId / messages / streaming 均从 useAgentStore 读
 * - chat-runtime 直接更新 store（streamText + fullStream 迭代），无 Rust agent-event 订阅
 * - sendMessage 自行实现，处理"无 session 时先创建"的流程，绑定 active workspaceId
 */
export function useChat() {
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

  // 监听 activeWorkspaceId 变化：重载该工作区下的会话列表
  useEffect(() => {
    loadSessions(undefined, activeWorkspaceId ?? undefined);
  }, [activeWorkspaceId, loadSessions]);

  // 读取 pendingClear 标志（用户主动清空会话时为 true）
  const pendingClear = useAgentStore((s) => s.pendingClear);

  // 初始挂载时若已有会话但未选中，自动选第一个
  // 但如果用户主动清空（pendingClear=true），则跳过自动选择
  const initialAutoSelectDone = useRef(false);
  useEffect(() => {
    if (!initialAutoSelectDone.current && !pendingClear && sessions.length > 0 && !currentSessionId) {
      initialAutoSelectDone.current = true;
      switchSession(sessions[0].id);
    }
  }, [sessions, currentSessionId, pendingClear, switchSession]);

  const sendMessage = useCallback(
    async (content: string, attachments?: AttachmentSpec[]) => {
      const trimmed = content.trim();
      if (!trimmed) return;

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
        ? await resolveAttachments(attachments, workspacePath)
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
    [createSession, appendMessage, clearStreamingContent, setStreaming, setError, workspacePath],
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
  };
}
