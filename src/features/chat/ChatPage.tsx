import { useState, useEffect } from "react";
import { toast } from "sonner";
import { useChat } from "@/features/chat/hooks/useChat";
import { useAgentStore } from "@/features/chat/store";
import { useI18n } from "@/locales/i18n";
import { ChatHeader } from "@/features/chat/components/chat-header";
import { MessageList } from "@/features/chat/components/message-list";
import { ChatInput } from "@/features/chat/components/chat-input";
import { ContextPanel } from "@/features/chat/components/context-panel";
import { ApprovalCard } from "@/features/agent/components/ApprovalCard";
import { PlanDiffReview } from "@/features/agent/components/PlanDiffReview";
import { SLASH_COMMANDS, type SlashCommand } from "@/features/chat/components/slash-commands";
import type { AttachmentSpec } from "@/features/chat/types";

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

  const [input, setInput] = useState("");
  const [attachments, setAttachments] = useState<AttachmentSpec[]>([]);
  const [panelOpen, setPanelOpen] = useState(false);
  const [activeCommand, setActiveCommand] = useState<SlashCommand | null>(null);

  const title = activeSession?.title || t.agentChat.title;

  useEffect(() => {
    const handler = (e: Event) => {
      const detail = (e as CustomEvent<string>).detail;
      if (detail) setInput(detail);
    };
    window.addEventListener("chat:prompt", handler);
    return () => window.removeEventListener("chat:prompt", handler);
  }, []);

  const executeCommand = (stem: string, args?: string) => {
    switch (stem) {
      case "/new":
        void handleNewSession();
        return true;
      case "/clear":
        useAgentStore.getState().replaceMessages([]);
        return true;
      case "/write":
        if (!args) {
          toast.warning(t.agentChat.slashWriteNoArgs);
        } else {
          toast.info(t.agentChat.slashWriteWIP);
        }
        return true;
      case "/help":
        toast.info("/new - 新建会话\n/clear - 清空消息\n/write - 写作模式\n/help - 显示帮助\n/status - 查看状态\n/depth - 设置思考深度\n/wiki - 打开 Wiki\n/memory - 打开记忆");
        return true;
      case "/status":
        toast.info("状态功能开发中");
        return true;
      case "/depth":
        toast.info(`思考深度设置功能开发中，参数: ${args || "未提供"}`);
        return true;
      case "/character":
      case "/world":
      case "/plot":
        toast.info(`${stem.slice(1)} 模式功能开发中`);
        return true;
      case "/wiki":
      case "/memory":
        toast.info(`导航功能开发中: ${stem}`);
        return true;
      case "/export":
        toast.info(`导出功能开发中，参数: ${args || "未提供"}`);
        return true;
      default:
        toast.error(`命令 ${stem} 未实现`);
        return true;
    }
  };

  const handleSubmit = () => {
    const trimmed = input.trim();
    if ((!trimmed && !activeCommand) || streaming) return;

    if (activeCommand) {
      const success = executeCommand(activeCommand.stem, trimmed);
      if (success) {
        setActiveCommand(null);
        setInput("");
        setAttachments([]);
      }
      return;
    }

    if (trimmed.startsWith("/")) {
      const stemMatch = trimmed.match(/^\/\S+/);
      const stem = stemMatch?.[0] ?? trimmed;
      const args = trimmed.slice(stem.length).trim();

      const matchedCommand = SLASH_COMMANDS.find((cmd) => cmd.stem === stem);

      if (matchedCommand) {
        executeCommand(stem, args);
      } else {
        toast.error(`未知命令: ${stem}\n输入 /help 查看可用命令`);
      }
      setInput("");
      setAttachments([]);
      return;
    }

    setInput("");
    setAttachments([]);
    void sendMessage(trimmed, attachments.length > 0 ? attachments : undefined);
  };

  const handleActiveCommandChange = (cmd: SlashCommand | null) => {
    if (cmd && !cmd.hasArgs) {
      executeCommand(cmd.stem);
      setActiveCommand(null);
    } else {
      setActiveCommand(cmd);
    }
  };

  const handleAttachFile = (filePath: string) => {
    const parts = filePath.split(/[\\/]/);
    const label = parts[parts.length - 1] || filePath;
    setAttachments((prev) => [...prev, { kind: "file", ref: filePath, label }]);
  };

  const handleRemoveAttachment = (index: number) => {
    setAttachments((prev) => prev.filter((_, i) => i !== index));
  };

  const totalTokens = (activeSession?.input_tokens ?? 0) + (activeSession?.output_tokens ?? 0);

  return (
    <div className="flex h-full bg-background">
      <main className="flex min-w-0 flex-1 flex-col">
        <ChatHeader
          title={title}
          streaming={streaming}
          hasSession={!!activeSession}
          planModeActive={planModeActive}
          panelOpen={panelOpen}
          onNewSession={handleNewSession}
          onDeleteSession={handleDeleteSession}
          onTogglePanel={() => setPanelOpen((v) => !v)}
          onTogglePlanMode={togglePlanMode}
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
        />
      </main>

      <ContextPanel
        open={panelOpen}
        workspacePath={workspacePath}
        sessionId={activeSession?.id ?? null}
        totalTokens={totalTokens}
      />

      {planQueue.length > 0 && (
        <PlanDiffReview
          queue={planQueue}
          onApplyAll={async () => { await planApplyAll(); }}
          onRejectOne={planRemoveOne}
          onDiscardAll={planClear}
        />
      )}
    </div>
  );
}