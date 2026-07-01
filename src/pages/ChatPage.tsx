import { useRef, useState } from "react";
import { PanelRightOpen, PanelRightClose } from "lucide-react";
import { useChat } from "@/features/chat/hooks/useChat";
import { useWorkspaceStore } from "@/stores/workspace";
import { useI18n } from "@/shared/i18n";
import { cn } from "@/shared/utils";
import { ChatTopBar } from "@/features/chat/components/ChatTopBar";
import { MessageList } from "@/features/chat/components/MessageList";
import { ChatInput } from "@/features/chat/components/ChatInput";
import { ContextPanel } from "@/features/chat/components/ContextPanel";
import { ConfirmationDialog } from "@/features/chat/components/ConfirmationDialog";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { AttachmentSpec } from "@/shared/types";

export default function ChatPage() {
  const { t } = useI18n();
  const {
    activeSession,
    messages,
    streaming,
    streamingContent,
    streamingReasoning,
    error,
    sendMessage,
    cancel,
    handleNewSession,
    handleDeleteSession,
    pendingConfirmation,
    submittingConfirmation,
    respondConfirmation,
  } = useChat();

  const [input, setInput] = useState("");
  const [attachments, setAttachments] = useState<AttachmentSpec[]>([]);
  const scrollRef = useRef<HTMLDivElement>(null);
  const [panelOpen, setPanelOpen] = useState(false);

  // 工作区信息：从 workspace store 拿 active workspace，派生 path
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspaceId);
  const workspaces = useWorkspaceStore((s) => s.workspaces);
  const activeWorkspace = workspaces.find((w) => w.id === activeWorkspaceId) ?? null;
  const workspacePath = activeWorkspace?.path ?? null;

  const title = activeSession?.title || t.agentChat.title;

  const handleSubmit = () => {
    const trimmed = input.trim();
    if (!trimmed || streaming) return;
    setInput("");
    // 发送时带上附件，发送后清空附件列表
    const atts = attachments.length > 0 ? attachments : undefined;
    setAttachments([]);
    void sendMessage(trimmed, atts);
  };

  // 文件选择器回调：从绝对路径派生文件名作为 label
  const handleAttachFile = (filePath: string) => {
    const parts = filePath.split(/[\\/]/);
    const label = parts[parts.length - 1] || filePath;
    setAttachments((prev) => [
      ...prev,
      { kind: "file", ref: filePath, label },
    ]);
  };

  const handleAddAttachment = (att: AttachmentSpec) => {
    setAttachments((prev) => {
      // 去重：相同 kind+ref 不重复添加
      if (prev.some((a) => a.kind === att.kind && a.ref === att.ref)) {
        return prev;
      }
      return [...prev, att];
    });
  };

  const handleRemoveAttachment = (index: number) => {
    setAttachments((prev) => prev.filter((_, i) => i !== index));
  };

  return (
    <div className="flex h-full bg-[var(--bg-base-default)]">
      {/* Main chat area */}
      <main className="flex min-w-0 flex-1 flex-col">
        <ChatTopBar
          title={title}
          streaming={streaming}
          hasSession={!!activeSession}
          onNewSession={handleNewSession}
          onDeleteSession={handleDeleteSession}
        />

        {/* Inline panel toggle button below the top bar */}
        <div className="flex items-center border-b border-[var(--border-neutral-l1)] px-2 py-0.5">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon-xs"
                onClick={() => setPanelOpen((prev) => !prev)}
                aria-label={t.agentChat.contextPanel}
                className={cn(
                  "text-[var(--text-tertiary)]",
                  panelOpen && "text-[var(--text-secondary)]"
                )}
              >
                {panelOpen ? (
                  <PanelRightClose className="size-3.5" />
                ) : (
                  <PanelRightOpen className="size-3.5" />
                )}
              </Button>
            </TooltipTrigger>
            <TooltipContent>{t.agentChat.contextPanel}</TooltipContent>
          </Tooltip>
        </div>

        <MessageList
          messages={messages}
          streaming={streaming}
          streamingContent={streamingContent}
          streamingReasoning={streamingReasoning}
          error={error}
          scrollRef={scrollRef}
        />
        <ChatInput
          value={input}
          onChange={setInput}
          onSubmit={handleSubmit}
          onCancel={cancel}
          streaming={streaming}
          attachments={attachments}
          onAttachFile={handleAttachFile}
          onAddAttachment={handleAddAttachment}
          onRemoveAttachment={handleRemoveAttachment}
          workspaceId={activeWorkspaceId}
          workspacePath={workspacePath}
        />
      </main>

      {/* Right context panel (inline, NOT a Sheet) */}
      <ContextPanel
        open={panelOpen}
        onToggle={() => setPanelOpen((prev) => !prev)}
        workspacePath={workspacePath}
        sessionId={activeSession?.id ?? null}
        totalTokens={(activeSession?.input_tokens ?? 0) + (activeSession?.output_tokens ?? 0)}
      />

      {/* SafetyGate 确认对话框：高风险工具调用前请求用户授权 */}
      <ConfirmationDialog
        pending={pendingConfirmation}
        submitting={submittingConfirmation}
        onRespond={respondConfirmation}
      />

      {/* TODO(integration): 在此处集成子 Agent 面板。
          推荐方式 — 当存在活跃 session 时渲染为右侧第二栏或 ContextPanel 内的一个分区：
            <SubAgentPanel sessionId={activeSession?.id ?? null} />
          详情面板（选中后）可叠加为右侧 Sheet 或在 ContextPanel 内切换：
            <SubAgentDetail />
          组件位于 @/features/sub_agent/components。
          需要先确认 activeSession.id 即主 Agent 的 session_id（与后端
          SubAgentControl.list_children(parent_thread_id=session_id) 对齐）。 */}
    </div>
  );
}
