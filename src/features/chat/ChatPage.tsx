/**
 * ═══════════════════════════════════════════════════════════════════════════
 * ChatPage - Agent 聊天主页面
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { listen } from "@tauri-apps/api/event";
import { useChat } from "@/features/chat/hooks/useChat";
import { useAgentStore } from "@/features/chat/store";
import { useI18n } from "@/locales/i18n";
import { ChatHeader } from "@/features/chat/components/chat-header";
import { MessageList } from "@/features/chat/components/message-list";
import { ChatInput } from "@/features/chat/components/chat-input";
import { ContextPanel } from "@/features/chat/components/context-panel";
import { ApprovalCard } from "@/features/agent/components/ApprovalCard";
import { PlanDiffReview } from "@/features/agent/components/PlanDiffReview";
import { MemoryPanel } from "@/features/agent/components/MemoryPanel";
import { LoopPanel } from "@/features/agent/components/LoopPanel";
import { FailureReport, type FailurePattern } from "@/features/agent/components/FailureReport";
import { SLASH_COMMANDS, type SlashCommand } from "@/features/chat/components/slash-commands";
import { ConversationTimeline } from "@/features/chat/components/conversation-timeline";
import { optimizePromptDirect } from "@/features/chat/services/prompt-optimizer";
import { loadSettings, type AiModelConfig } from "@/services/settings";
import type { AttachmentSpec } from "@/features/chat/types";

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * Agent 聊天主页面，整合消息列表、输入框、侧边面板等组件
 */
export default function ChatPage() {
  const { t } = useI18n();
  const {
    activeSession,
    messages,
    streaming,
    error,
    sendMessage,
    cancel,
    regenerate,
    handleNewSession,
    handleDeleteSession,
    workspacePath,
    pendingConfirmation,
    submittingConfirmation,
    respondApproval,
    planModeActive,
    planQueue,
    togglePlanMode,
    planApplyAll,
    planRemoveOne,
    planClear,
  } = useChat();

  // ── 状态管理 ──────────────────────────────────────────────────────────────

  const [input, setInput] = useState("");
  const [attachments, setAttachments] = useState<AttachmentSpec[]>([]);
  const [panelOpen, setPanelOpen] = useState(false);
  const [memoryPanelOpen, setMemoryPanelOpen] = useState(false);
  const [loopPanelOpen, setLoopPanelOpen] = useState(false);
  const [activeCommand, setActiveCommand] = useState<SlashCommand | null>(null);
  const [failures, setFailures] = useState<FailurePattern[]>([]);
  
  // ── 事件监听 ──────────────────────────────────────────────────────────────

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    listen<FailurePattern[]>("agent:failure", (event) => {
      setFailures((prev) => [...prev, ...event.payload].slice(-50));
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlisten = fn;
      }
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const title = activeSession?.title || t.agentChat.title;

  useEffect(() => {
    const handler = (e: Event) => {
      const detail = (e as CustomEvent<string>).detail;
      if (detail) setInput(detail);
    };
    window.addEventListener("chat:prompt", handler);
    return () => window.removeEventListener("chat:prompt", handler);
  }, []);

  // ── 命令处理 ──────────────────────────────────────────────────────────────

  /**
   * 执行本地 slash 命令
   * 返回 true 表示已在本地处理完成，返回 false 表示需要发送给 AI agent
   */
  const executeCommand = useCallback((stem: string, _args?: string): boolean => {
    switch (stem) {
      case "/new":
        void handleNewSession();
        return true;
      case "/clear":
        useAgentStore.getState().replaceMessages([]);
        return true;
      case "/help":
        toast.info(t.agentChat.slashHelpText);
        return true;
      case "/status":
        toast.info(t.agentChat.slashStatusWip);
        return true;
      case "/wiki":
      case "/memory":
        toast.info(t.agentChat.slashNavWip.replace("{command}", stem));
        return true;
      case "/write":
      case "/character":
      case "/world":
      case "/plot":
      case "/depth":
      case "/export":
        return false;
      default:
        toast.error(t.agentChat.slashNotImplemented.replace("{command}", stem));
        return true;
    }
  }, [t, handleNewSession]);

  // ── 消息提交 ──────────────────────────────────────────────────────────────

  const handleSubmit = useCallback(() => {
    const trimmed = input.trim();
    if ((!trimmed && !activeCommand) || streaming) return;

    let messageText = trimmed;
    let localAttachments = attachments;

    if (activeCommand) {
      // 有 activeCommand 时，组合命令词干 + 参数作为消息
      const stem = activeCommand.stem;
      const args = trimmed;
      const handled = executeCommand(stem, args);
      if (handled) {
        // 本地命令已处理，不需要发送给 AI
        setActiveCommand(null);
        setInput("");
        setAttachments([]);
        return;
      }
      // 需要发送给 AI：组合消息内容
      messageText = args ? `${stem} ${args}` : stem;
      setActiveCommand(null);
      setInput("");
      setAttachments([]);
      void sendMessage(messageText, localAttachments.length > 0 ? localAttachments : undefined);
      return;
    }

    if (trimmed.startsWith("/")) {
      const stemMatch = trimmed.match(/^\/\S+/);
      const stem = stemMatch?.[0] ?? trimmed;
      const args = trimmed.slice(stem.length).trim();

      const matchedCommand = SLASH_COMMANDS.find((cmd) => cmd.stem === stem);

      if (matchedCommand) {
        const handled = executeCommand(stem, args);
        if (handled) {
          // 本地命令已处理
          setInput("");
          setAttachments([]);
          return;
        }
        // 需要发送给 AI：使用原始输入
        messageText = trimmed;
      } else {
        // 未知命令，直接作为普通消息发送给 AI
        messageText = trimmed;
      }
    }

    setInput("");
    setAttachments([]);
    void sendMessage(messageText, localAttachments.length > 0 ? localAttachments : undefined);
  }, [input, activeCommand, streaming, attachments, executeCommand, sendMessage]);

  const handleActiveCommandChange = useCallback((cmd: SlashCommand | null) => {
    if (cmd && !cmd.hasArgs) {
      const handled = executeCommand(cmd.stem);
      if (handled) {
        setActiveCommand(null);
      } else {
        // 命令需要发送给 AI，设置 activeCommand 让用户输入参数
        setActiveCommand(cmd);
      }
    } else {
      setActiveCommand(cmd);
    }
  }, [executeCommand]);

  // ── 附件处理 ──────────────────────────────────────────────────────────────

  const handleAttachFile = useCallback((filePath: string) => {
    const parts = filePath.split(/[\\/]/);
    const label = parts[parts.length - 1] || filePath;
    setAttachments((prev) => [...prev, { kind: "file", ref: filePath, label }]);
  }, []);

  const handleRemoveAttachment = useCallback((index: number) => {
    setAttachments((prev) => prev.filter((_, i) => i !== index));
  }, []);

  // ── 提示词优化 ────────────────────────────────────────────────────────────

  /**
   * 优化当前输入的提示词
   */
  const handleOptimizePrompt = useCallback(async (): Promise<string | null> => {
    try {
      // 加载设置获取当前模型配置
      const settings = await loadSettings();
      const models = settings.ai.models;
      const activeModelId = settings.ai.active_model_id;

      if (!activeModelId) {
        toast.error(t.chat.errors.noModelConfigured);
        return null;
      }

      const modelConfig = models.find((m: AiModelConfig) => m.id === activeModelId);
      if (!modelConfig) {
        toast.error(t.chat.errors.noModelConfigured);
        return null;
      }

      // 调用优化服务
      const optimized = await optimizePromptDirect(input, {
        provider: modelConfig.provider,
        model: modelConfig.model,
        api_key: modelConfig.api_key,
        base_url: modelConfig.base_url,
      });

      if (optimized) {
        toast.success(t.agentChat.promptOptimized);
        return optimized;
      } else {
        toast.error(t.agentChat.optimizeFailed);
        return null;
      }
    } catch (error) {
      console.error("Prompt optimization error:", error);
      toast.error(t.agentChat.optimizeFailed);
      return null;
    }
  }, [input, t]);

  // ── 面板控制 ──────────────────────────────────────────────────────────────

  const togglePanel = useCallback(() => setPanelOpen((v) => !v), []);
  const toggleMemoryPanel = useCallback(() => setMemoryPanelOpen((v) => !v), []);
  const toggleLoopPanel = useCallback(() => setLoopPanelOpen((v) => !v), []);
  const closeMemoryPanel = useCallback(() => setMemoryPanelOpen(false), []);
  const closeLoopPanel = useCallback(() => setLoopPanelOpen(false), []);
  const dismissFailures = useCallback(() => setFailures([]), []);
  const handleApplyAll = useCallback(async () => { await planApplyAll(); }, [planApplyAll]);

  // 时间线节点点击：滚动到对应消息
  const handleTimelineNodeClick = useCallback((messageIndex: number) => {
    const event = new CustomEvent("timeline-scroll-to", { detail: { messageIndex } });
    window.dispatchEvent(event);
  }, []);

  const totalTokens = (activeSession?.input_tokens ?? 0) + (activeSession?.output_tokens ?? 0);

  // ── 渲染 ──────────────────────────────────────────────────────────────────

  return (
    <div className="relative flex h-full bg-background">
      {/* 时间线悬浮层 */}
      <ConversationTimeline
        messages={messages}
        streaming={streaming}
        onNodeClick={handleTimelineNodeClick}
      />

      <main className="flex min-w-0 flex-1 flex-col">
        <ChatHeader
          title={title}
          streaming={streaming}
          hasSession={!!activeSession}
          planModeActive={planModeActive}
          panelOpen={panelOpen}
          memoryPanelOpen={memoryPanelOpen}
          loopPanelOpen={loopPanelOpen}
          onNewSession={handleNewSession}
          onDeleteSession={handleDeleteSession}
          onTogglePanel={togglePanel}
          onTogglePlanMode={togglePlanMode}
          onToggleMemoryPanel={toggleMemoryPanel}
          onToggleLoopPanel={toggleLoopPanel}
        />

        <MessageList
          messages={messages}
          streaming={streaming}
          error={error}
          onRegenerate={regenerate}
        />

        {pendingConfirmation && (
          <div className="px-4 pb-2">
            <ApprovalCard
              confirmation={pendingConfirmation}
              submitting={submittingConfirmation}
              onRespond={respondApproval}
            />
          </div>
        )}

        {failures.length > 0 && (
          <div className="px-4 pb-2">
            <FailureReport
              failures={failures}
              onDismiss={dismissFailures}
            />
          </div>
        )}

        <ChatInput
          value={input}
          onChange={setInput}
          onSubmit={handleSubmit}
          onCancel={cancel}
          streaming={streaming}
          attachments={attachments}
          onAttachFile={handleAttachFile}
          onRemoveAttachment={handleRemoveAttachment}
          activeCommand={activeCommand}
          onActiveCommandChange={handleActiveCommandChange}
          onOptimizePrompt={handleOptimizePrompt}
        />
      </main>

      <ContextPanel
        open={panelOpen}
        workspacePath={workspacePath}
        sessionId={activeSession?.id ?? null}
        totalTokens={totalTokens}
      />

      <MemoryPanel
        open={memoryPanelOpen}
        sessionId={activeSession?.id ?? null}
        onClose={closeMemoryPanel}
      />

      <LoopPanel
        open={loopPanelOpen}
        sessionId={activeSession?.id ?? null}
        onClose={closeLoopPanel}
      />

      {planQueue.length > 0 && (
        <PlanDiffReview
          queue={planQueue}
          onApplyAll={handleApplyAll}
          onRejectOne={planRemoveOne}
          onDiscardAll={planClear}
        />
      )}
    </div>
  );
}