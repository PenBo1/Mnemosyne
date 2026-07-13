import { useState, useEffect } from "react";
import { useChat } from "@/features/chat/hooks/useChat";
import { useAgentStore } from "@/features/chat/store";
import { useI18n } from "@/locales/i18n";
import { ChatHeader } from "@/features/chat/components/chat-header";
import { MessageList } from "@/features/chat/components/message-list";
import { ChatInput } from "@/features/chat/components/chat-input";
import { ContextPanel } from "@/features/chat/components/context-panel";
import { ApprovalCard } from "@/features/agent/components/ApprovalCard";
import { PlanDiffReview } from "@/features/agent/components/PlanDiffReview";
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

  const title = activeSession?.title || t.agentChat.title;

  // 监听 EmptyState 卡片点击 —— 填入输入框
  useEffect(() => {
    const handler = (e: Event) => {
      const detail = (e as CustomEvent<string>).detail;
      if (detail) setInput(detail);
    };
    window.addEventListener("chat:prompt", handler);
    return () => window.removeEventListener("chat:prompt", handler);
  }, []);

  const handleSubmit = () => {
    const trimmed = input.trim();
    if (!trimmed || streaming) return;
    setInput("");
    const atts = attachments.length > 0 ? attachments : undefined;
    setAttachments([]);

    if (trimmed === "/new") {
      void handleNewSession();
      return;
    }
    if (trimmed === "/clear") {
      useAgentStore.getState().replaceMessages([]);
      return;
    }

    void sendMessage(trimmed, atts);
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
