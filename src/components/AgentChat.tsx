import { useState, useRef, useEffect } from "react";
import { useAgent } from "@/hooks/useAgent";
import { useSession } from "@/hooks/useSession";
import { useI18n } from "@/lib/i18n";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Spinner } from "@/components/ui/spinner";

interface AgentChatProps {
  novelId?: string;
}

export function AgentChat({ novelId }: AgentChatProps) {
  const { t } = useI18n();
  const { sessions, currentSession, create, switch: switchSession } = useSession(novelId);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const { messages, streaming, streamingContent, loading, sendMessage, cancel } = useAgent(sessionId);
  const [input, setInput] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Sync sessionId with currentSession
  useEffect(() => {
    if (currentSession) {
      setSessionId(currentSession.id);
    } else {
      setSessionId(null);
    }
  }, [currentSession]);

  // Auto-scroll to bottom
  useEffect(() => {
    if (scrollRef.current) {
      const el = scrollRef.current;
      // Use requestAnimationFrame to ensure DOM has updated
      requestAnimationFrame(() => {
        el.scrollTop = el.scrollHeight;
      });
    }
  }, [messages, streamingContent]);

  // Focus input when not streaming
  useEffect(() => {
    if (!streaming && inputRef.current) {
      inputRef.current.focus();
    }
  }, [streaming]);

  const handleSend = async () => {
    if (!input.trim() || streaming) return;
    const content = input.trim();
    setInput("");
    await sendMessage(content);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const handleNewSession = async () => {
    const session = await create(t.agentChat.newChat);
    setSessionId(session.id);
  };

  const handleSwitchSession = async (newSessionId: string) => {
    setSessionId(newSessionId);
    await switchSession(newSessionId);
  };

  // Empty state - no session selected
  if (!sessionId) {
    return (
      <div className="flex flex-col h-full">
        <div className="flex items-center justify-between p-3 border-b">
          <h3 className="text-sm font-medium">{t.agentChat.title}</h3>
          <Button onClick={handleNewSession} size="sm">
            {t.agentChat.newChat}
          </Button>
        </div>
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center text-muted-foreground">
            <p className="text-sm">{t.agentChat.emptyTitle}</p>
            <p className="text-xs mt-1">{t.agentChat.emptyDesc}</p>
            <Button onClick={handleNewSession} className="mt-4" size="sm">
              {t.agentChat.newChat}
            </Button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b">
        <div className="flex items-center gap-2">
          <h3 className="text-sm font-medium">{t.agentChat.title}</h3>
          {currentSession && (
            <span className="text-xs text-muted-foreground">
              {currentSession.title || t.agentChat.unnamedSession}
            </span>
          )}
        </div>
        <div className="flex items-center gap-1">
          <Button variant="ghost" size="sm" onClick={handleNewSession}>
            {t.agentChat.newChat}
          </Button>
          {sessions.length > 0 && (
            <select
              className="text-xs border rounded px-2 py-1"
              value={sessionId}
              onChange={(e) => handleSwitchSession(e.target.value)}
            >
              {sessions.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.title || t.agentChat.unnamed} ({s.message_count} {t.agentChat.messageCount})
                </option>
              ))}
            </select>
          )}
        </div>
      </div>

      {/* Messages */}
      <ScrollArea className="flex-1 p-3" ref={scrollRef}>
        <div className="space-y-3">
          {loading && (
            <div className="flex justify-center py-4">
              <Spinner className="size-4" />
            </div>
          )}

          {!loading && messages.length === 0 && !streaming && (
            <div className="text-center text-muted-foreground py-8">
              <p className="text-sm">{t.agentChat.emptyTitle}</p>
              <p className="text-xs mt-1">{t.agentChat.emptyDesc}</p>
            </div>
          )}

          {messages.map((msg) => (
            <div
              key={msg.id}
              className={`flex ${msg.role === "user" ? "justify-end" : "justify-start"}`}
            >
              <div
                className={`max-w-[80%] rounded-lg px-3 py-2 text-sm ${
                  msg.role === "user"
                    ? "bg-primary text-primary-foreground"
                    : msg.role === "assistant"
                    ? "bg-muted"
                    : msg.role === "tool"
                    ? "bg-muted/50 text-xs font-mono"
                    : ""
                }`}
              >
                {msg.role === "tool" ? (
                  <div className="opacity-70">
                    <span className="font-medium">{t.agentChat.toolResult}</span> {msg.content.slice(0, 200)}
                    {msg.content.length > 200 && "..."}
                  </div>
                ) : (
                  <div className="whitespace-pre-wrap">{msg.content}</div>
                )}
                {msg.tool_calls && (
                  <div className="mt-1 text-xs opacity-60">
                    {t.agentChat.toolCalls}{" "}
                    {(() => {
                      try {
                        return JSON.parse(msg.tool_calls)
                          .map((tc: { name: string }) => tc.name)
                          .join(", ");
                      } catch {
                        return msg.tool_calls;
                      }
                    })()}
                  </div>
                )}
              </div>
            </div>
          ))}

          {/* Streaming response */}
          {streaming && streamingContent && (
            <div className="flex justify-start">
              <div className="max-w-[80%] rounded-lg px-3 py-2 text-sm bg-muted">
                <div className="whitespace-pre-wrap">{streamingContent}</div>
                <span className="animate-pulse">|</span>
              </div>
            </div>
          )}

          {/* Loading indicator when streaming but no content yet */}
          {streaming && !streamingContent && (
            <div className="flex justify-start">
              <div className="max-w-[80%] rounded-lg px-3 py-2 text-sm bg-muted">
                <div className="flex items-center gap-2 text-muted-foreground">
                  <Spinner className="size-3" />
                  <span>{t.agentChat.thinking}</span>
                </div>
              </div>
            </div>
          )}
        </div>
      </ScrollArea>

      {/* Input */}
      <div className="p-3 border-t">
        <div className="flex gap-2">
          <Input
            ref={inputRef}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={t.agentChat.placeholder}
            disabled={streaming}
            className="flex-1"
          />
          {streaming ? (
            <Button variant="outline" size="sm" onClick={cancel}>
              {t.agentChat.stop}
            </Button>
          ) : (
            <Button size="sm" onClick={handleSend} disabled={!input.trim()}>
              {t.agentChat.send}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
