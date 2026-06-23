import { useState, useRef, useEffect, useCallback } from "react";
import { cn } from "@/lib/utils";
import { useI18n } from "@/lib/i18n";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { ChatComposer } from "./composer";
import { ThreadMessageList } from "./thread/ThreadMessageList";
import { WelcomeIntro } from "./intro/WelcomeIntro";
import { useAgent } from "@/hooks/useAgent";
import { useAgentStore } from "@/stores/agent";
import { PlusIcon, ArrowUpIcon } from "lucide-react";
import type { ChatContainerProps } from "@/types/chat";
import type { ChatMessage } from "@/types/chat";

export function ChatContainer({
  novelId,
  className,
  autoFocus,
}: ChatContainerProps) {
  const { t } = useI18n();
  const scrollRef = useRef<HTMLDivElement>(null);
  const [input, setInput] = useState("");

  // Get session state from store
  const sessions = useAgentStore((s) => s.sessions);
  const currentSessionId = useAgentStore((s) => s.currentSessionId);
  const createSession = useAgentStore((s) => s.createSession);
  const switchSession = useAgentStore((s) => s.switchSession);

  // Get agent state
  const { messages, streaming, streamingContent, loading, sendMessage, cancel } =
    useAgent(currentSessionId);

  // Convert messages to ChatMessage format
  const chatMessages: ChatMessage[] = messages.map((msg) => ({
    ...msg,
    status: streaming ? "streaming" : "complete",
  }));

  // Auto-scroll to bottom on new messages
  useEffect(() => {
    if (scrollRef.current && streaming) {
      const el = scrollRef.current;
      requestAnimationFrame(() => {
        el.scrollTop = el.scrollHeight;
      });
    }
  }, [messages, streamingContent, streaming]);

  // Handle send message
  const handleSend = useCallback(async () => {
    if (!input.trim() || streaming) return;

    const content = input.trim();
    setInput("");

    // Create new session if needed
    let sessionId = currentSessionId;
    if (!sessionId) {
      const session = await createSession(novelId, t.agentChat.newChat);
      sessionId = session.id;
      // Wait a bit for session to be ready
      setTimeout(() => sendMessage(content), 100);
    } else {
      await sendMessage(content);
    }
  }, [input, streaming, currentSessionId, novelId, createSession, sendMessage, t.agentChat.newChat]);

  // Handle cancel
  const handleCancel = useCallback(() => {
    cancel();
  }, [cancel]);

  // Handle new session
  const handleNewSession = useCallback(async () => {
    await createSession(novelId, t.agentChat.newChat);
  }, [createSession, novelId, t.agentChat.newChat]);

  // Handle session switch
  const handleSwitchSession = useCallback(
    async (sessionId: string) => {
      await switchSession(sessionId);
    },
    [switchSession]
  );

  // Current session info
  const currentSession = sessions.find((s) => s.id === currentSessionId);

  // Empty state - no session selected
  const isEmpty = !currentSessionId || messages.length === 0;

  return (
    <div className={cn("flex flex-col h-full", className)}>
      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b">
        <div className="flex items-center gap-2">
          <h3 className="text-sm font-medium">{t.agentChat.title}</h3>
          {currentSession && (
            <span className="text-xs text-muted-foreground truncate max-w-[200px]">
              {currentSession.title || t.agentChat.unnamedSession}
            </span>
          )}
        </div>
        <div className="flex items-center gap-1">
          <Button variant="ghost" size="sm" onClick={handleNewSession}>
            <PlusIcon className="size-4" />
            {t.agentChat.newChat}
          </Button>
          {sessions.length > 0 && (
            <select
              className="text-xs border rounded px-2 py-1 bg-background"
              value={currentSessionId || ""}
              onChange={(e) => handleSwitchSession(e.target.value)}
            >
              <option value="" disabled>
                {t.agentChat.unnamed}
              </option>
              {sessions.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.title || t.agentChat.unnamed} ({s.message_count} {t.agentChat.messageCount})
                </option>
              ))}
            </select>
          )}
        </div>
      </div>

      {/* Content area */}
      <ScrollArea className="flex-1" ref={scrollRef}>
        {isEmpty && !streaming ? (
          <div className="flex flex-col h-full min-h-[400px]">
            {/* Welcome intro */}
            <div className="flex-1 flex items-center justify-center">
              <WelcomeIntro />
            </div>

            {/* Large input box for empty state */}
            <div className="p-4">
              <div className="relative border rounded-2xl bg-muted/50 shadow-sm">
                <textarea
                  ref={(el) => {
                    if (autoFocus && el) el.focus();
                  }}
                  value={input}
                  onChange={(e) => setInput(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && !e.shiftKey) {
                      e.preventDefault();
                      handleSend();
                    }
                  }}
                  placeholder={t.agentChat.placeholder}
                  rows={3}
                  className="w-full resize-none rounded-2xl bg-transparent px-4 pt-4 pb-14 text-base focus:outline-none placeholder:text-muted-foreground"
                />
                <div className="absolute bottom-3 left-3 right-3 flex items-center justify-between">
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    className="text-muted-foreground"
                  >
                    <PlusIcon className="size-5" />
                  </Button>
                  <Button
                    size="icon"
                    className="rounded-full bg-foreground text-background hover:bg-foreground/90"
                    onClick={handleSend}
                    disabled={!input.trim()}
                  >
                    <ArrowUpIcon className="size-5" />
                  </Button>
                </div>
              </div>
            </div>
          </div>
        ) : (
          <div className="p-3">
            <ThreadMessageList
              messages={chatMessages}
              streamingContent={streamingContent}
              isStreaming={streaming}
            />
          </div>
        )}
      </ScrollArea>

      {/* Input area (when there are messages) */}
      {!isEmpty && (
        <div className="p-3 border-t">
          <ChatComposer
            value={input}
            onChange={setInput}
            onSubmit={handleSend}
            onCancel={handleCancel}
            streaming={streaming}
            disabled={loading}
            placeholder={t.agentChat.placeholderFollowUp}
            autoFocus={autoFocus}
          />
        </div>
      )}
    </div>
  );
}