import { useEffect, useRef, useState } from "react";
import { PanelRightOpen, PanelRightClose } from "lucide-react";
import { useChat } from "@/features/chat/hooks/useChat";
import { fetchWorkspaces } from "@/features/workspace/services";
import { useI18n } from "@/shared/i18n";
import { cn } from "@/shared/utils";
import { ChatTopBar } from "@/features/chat/components/ChatTopBar";
import { MessageList } from "@/features/chat/components/MessageList";
import { ChatInput } from "@/features/chat/components/ChatInput";
import { ContextPanel } from "@/features/chat/components/ContextPanel";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

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
  } = useChat();

  const [input, setInput] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);
  const [panelOpen, setPanelOpen] = useState(false);
  const [workspacePath, setWorkspacePath] = useState<string | null>(null);

  const title = activeSession?.title || t.agentChat.title;

  // Load workspace path from first workspace on mount
  useEffect(() => {
    void (async () => {
      try {
        const workspaces = await fetchWorkspaces();
        if (workspaces.length > 0) {
          setWorkspacePath(workspaces[0].path);
        }
      } catch {
        // No workspaces available -- panel file browser will show empty state
      }
    })();
  }, []);

  const handleSubmit = () => {
    const trimmed = input.trim();
    if (!trimmed || streaming) return;
    setInput("");
    void sendMessage(trimmed);
  };

  const handleAttachFile = (_filePath: string) => {
    // Placeholder: in future, attach file to context or message
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
          onAttachFile={handleAttachFile}
        />
      </main>

      {/* Right context panel (inline, NOT a Sheet) */}
      <ContextPanel
        open={panelOpen}
        onToggle={() => setPanelOpen((prev) => !prev)}
        workspacePath={workspacePath}
      />
    </div>
  );
}
