import { useState, useRef, useEffect } from "react";
import { useAgent } from "@/hooks/useAgent";
import { useSession } from "@/hooks/useSession";
import { useI18n } from "@/lib/i18n";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Spinner } from "@/components/ui/spinner";
import { useCharacters } from "@/hooks/useCharacters";
import { useWorldSettings } from "@/hooks/useWorldSettings";
import { usePlotPoints } from "@/hooks/usePlotPoints";
import { useTimelineEvents } from "@/hooks/useTimelineEvents";
import { useResearchItems } from "@/hooks/useResearchItems";
import { useWorkspaceStore } from "@/stores/workspace";
import {
  UsersIcon,
  GlobeIcon,
  GitBranchIcon,
  ClockIcon,
  BookMarkedIcon,
  PanelRightOpenIcon,
  PanelRightCloseIcon,
  PlusIcon,
  ArrowUpIcon,
} from "lucide-react";

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
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const [showMaterials, setShowMaterials] = useState(false);

  const { activeWorkspaceId } = useWorkspaceStore();
  const { characters } = useCharacters(activeWorkspaceId);
  const { items: worldSettings } = useWorldSettings(activeWorkspaceId);
  const { points: plotPoints } = usePlotPoints(activeWorkspaceId);
  const { events: timelineEvents } = useTimelineEvents(activeWorkspaceId);
  const { items: researchItems } = useResearchItems(activeWorkspaceId);

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

    // Create a new session if there's no current session
    if (!sessionId) {
      const session = await create(t.agentChat.newChat);
      setSessionId(session.id);
      // Send the message after session is created
      setTimeout(() => sendMessage(content), 100);
    } else {
      await sendMessage(content);
    }
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
        {/* Header */}
        <div className="flex items-center justify-between p-3 border-b">
          <div className="flex items-center gap-2">
            <h3 className="text-sm font-medium">{t.agentChat.title}</h3>
          </div>
          <div className="flex items-center gap-1">
            <Button variant="ghost" size="sm" onClick={handleNewSession}>
              {t.agentChat.newChat}
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={() => setShowMaterials(!showMaterials)}
              title={showMaterials ? t.agentChat.hideMaterials : t.agentChat.showMaterials}
            >
              {showMaterials ? (
                <PanelRightCloseIcon className="size-4" />
              ) : (
                <PanelRightOpenIcon className="size-4" />
              )}
            </Button>
          </div>
        </div>

        {/* Welcome Landing */}
        <div className="flex-1 flex flex-col items-center justify-center px-4">
          <div className="text-center mb-8">
            <h1 className="text-2xl font-semibold mb-2">{t.agentChat.welcomeTitle}</h1>
            <p className="text-muted-foreground text-sm">{t.agentChat.welcomeHint}</p>
          </div>

          <div className="w-full max-w-2xl">
            <div className="relative border rounded-2xl bg-muted/50 shadow-sm">
              <textarea
                ref={inputRef}
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder={t.agentChat.placeholder}
                rows={3}
                className="w-full resize-none rounded-2xl bg-transparent px-4 pt-4 pb-14 text-base focus:outline-none placeholder:text-muted-foreground"
              />
              <div className="absolute bottom-3 left-3 right-3 flex items-center justify-between">
                <div className="flex items-center gap-1">
                  <Button variant="ghost" size="icon-sm" className="text-muted-foreground hover:text-foreground">
                    <PlusIcon className="size-5" />
                  </Button>
                </div>
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

        {/* Materials Panel */}
        {showMaterials && (
          <div className="w-64 border-l bg-muted/30 overflow-y-auto">
            <div className="p-3 border-b">
              <h4 className="text-sm font-medium">{t.sidebar.research}</h4>
            </div>
            <div className="p-3 space-y-4">
              {/* Characters */}
              <div>
                <div className="flex items-center gap-2 mb-2">
                  <UsersIcon className="size-4" />
                  <span className="text-xs font-medium">{t.characters.title}</span>
                </div>
                {characters.length === 0 ? (
                  <p className="text-xs text-muted-foreground">{t.characters.empty}</p>
                ) : (
                  <div className="space-y-1">
                    {characters.slice(0, 5).map((c) => (
                      <div key={c.id} className="text-xs p-2 bg-background rounded">
                        <p className="font-medium">{c.name}</p>
                        {c.role && <p className="text-muted-foreground">{c.role}</p>}
                      </div>
                    ))}
                    {characters.length > 5 && (
                      <p className="text-xs text-muted-foreground">+{characters.length - 5}</p>
                    )}
                  </div>
                )}
              </div>

              {/* Worldbuilding */}
              <div>
                <div className="flex items-center gap-2 mb-2">
                  <GlobeIcon className="size-4" />
                  <span className="text-xs font-medium">{t.worldbuilding.title}</span>
                </div>
                {worldSettings.length === 0 ? (
                  <p className="text-xs text-muted-foreground">{t.worldbuilding.empty}</p>
                ) : (
                  <div className="space-y-1">
                    {worldSettings.slice(0, 5).map((w) => (
                      <div key={w.id} className="text-xs p-2 bg-background rounded">
                        <p className="font-medium">{w.name}</p>
                        {w.description && <p className="text-muted-foreground line-clamp-1">{w.description}</p>}
                      </div>
                    ))}
                    {worldSettings.length > 5 && (
                      <p className="text-xs text-muted-foreground">+{worldSettings.length - 5}</p>
                    )}
                  </div>
                )}
              </div>

              {/* Plot */}
              <div>
                <div className="flex items-center gap-2 mb-2">
                  <GitBranchIcon className="size-4" />
                  <span className="text-xs font-medium">{t.plot.title}</span>
                </div>
                {plotPoints.length === 0 ? (
                  <p className="text-xs text-muted-foreground">{t.plot.empty}</p>
                ) : (
                  <div className="space-y-1">
                    {plotPoints.slice(0, 5).map((p) => (
                      <div key={p.id} className="text-xs p-2 bg-background rounded">
                        <p className="font-medium">{p.title}</p>
                        {p.description && <p className="text-muted-foreground line-clamp-1">{p.description}</p>}
                      </div>
                    ))}
                    {plotPoints.length > 5 && (
                      <p className="text-xs text-muted-foreground">+{plotPoints.length - 5}</p>
                    )}
                  </div>
                )}
              </div>

              {/* Timeline */}
              <div>
                <div className="flex items-center gap-2 mb-2">
                  <ClockIcon className="size-4" />
                  <span className="text-xs font-medium">{t.timeline.title}</span>
                </div>
                {timelineEvents.length === 0 ? (
                  <p className="text-xs text-muted-foreground">{t.timeline.empty}</p>
                ) : (
                  <div className="space-y-1">
                    {timelineEvents.slice(0, 5).map((e) => (
                      <div key={e.id} className="text-xs p-2 bg-background rounded">
                        <p className="font-medium">{e.title}</p>
                        {e.event_date && <p className="text-muted-foreground">{e.event_date}</p>}
                      </div>
                    ))}
                    {timelineEvents.length > 5 && (
                      <p className="text-xs text-muted-foreground">+{timelineEvents.length - 5}</p>
                    )}
                  </div>
                )}
              </div>

              {/* Research */}
              <div>
                <div className="flex items-center gap-2 mb-2">
                  <BookMarkedIcon className="size-4" />
                  <span className="text-xs font-medium">{t.research.title}</span>
                </div>
                {researchItems.length === 0 ? (
                  <p className="text-xs text-muted-foreground">{t.research.empty}</p>
                ) : (
                  <div className="space-y-1">
                    {researchItems.slice(0, 5).map((r) => (
                      <div key={r.id} className="text-xs p-2 bg-background rounded">
                        <p className="font-medium">{r.title}</p>
                        {r.content && <p className="text-muted-foreground line-clamp-1">{r.content}</p>}
                      </div>
                    ))}
                    {researchItems.length > 5 && (
                      <p className="text-xs text-muted-foreground">+{researchItems.length - 5}</p>
                    )}
                  </div>
                )}
              </div>
            </div>
          </div>
        )}
      </div>
    );
  }

  return (
    <div className="flex h-full">
      {/* Main Chat Area */}
      <div className="flex flex-col flex-1 min-w-0">
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
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={() => setShowMaterials(!showMaterials)}
              title={showMaterials ? t.agentChat.hideMaterials : t.agentChat.showMaterials}
            >
              {showMaterials ? (
                <PanelRightCloseIcon className="size-4" />
              ) : (
                <PanelRightOpenIcon className="size-4" />
              )}
            </Button>
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

      {/* Materials Panel */}
      {showMaterials && (
        <div className="w-64 border-l bg-muted/30 overflow-y-auto">
          <div className="p-3 border-b">
            <h4 className="text-sm font-medium">{t.sidebar.research}</h4>
          </div>
          <div className="p-3 space-y-4">
            {/* Characters */}
            <div>
              <div className="flex items-center gap-2 mb-2">
                <UsersIcon className="size-4" />
                <span className="text-xs font-medium">{t.characters.title}</span>
              </div>
              {characters.length === 0 ? (
                <p className="text-xs text-muted-foreground">{t.characters.empty}</p>
              ) : (
                <div className="space-y-1">
                  {characters.slice(0, 5).map((c) => (
                    <div key={c.id} className="text-xs p-2 bg-background rounded">
                      <p className="font-medium">{c.name}</p>
                      {c.role && <p className="text-muted-foreground">{c.role}</p>}
                    </div>
                  ))}
                  {characters.length > 5 && (
                    <p className="text-xs text-muted-foreground">+{characters.length - 5}</p>
                  )}
                </div>
              )}
            </div>

            {/* Worldbuilding */}
            <div>
              <div className="flex items-center gap-2 mb-2">
                <GlobeIcon className="size-4" />
                <span className="text-xs font-medium">{t.worldbuilding.title}</span>
              </div>
              {worldSettings.length === 0 ? (
                <p className="text-xs text-muted-foreground">{t.worldbuilding.empty}</p>
              ) : (
                <div className="space-y-1">
                  {worldSettings.slice(0, 5).map((w) => (
                    <div key={w.id} className="text-xs p-2 bg-background rounded">
                      <p className="font-medium">{w.name}</p>
                      {w.description && <p className="text-muted-foreground line-clamp-1">{w.description}</p>}
                    </div>
                  ))}
                  {worldSettings.length > 5 && (
                    <p className="text-xs text-muted-foreground">+{worldSettings.length - 5}</p>
                  )}
                </div>
              )}
            </div>

            {/* Plot */}
            <div>
              <div className="flex items-center gap-2 mb-2">
                <GitBranchIcon className="size-4" />
                <span className="text-xs font-medium">{t.plot.title}</span>
              </div>
              {plotPoints.length === 0 ? (
                <p className="text-xs text-muted-foreground">{t.plot.empty}</p>
              ) : (
                <div className="space-y-1">
                  {plotPoints.slice(0, 5).map((p) => (
                    <div key={p.id} className="text-xs p-2 bg-background rounded">
                      <p className="font-medium">{p.title}</p>
                      {p.description && <p className="text-muted-foreground line-clamp-1">{p.description}</p>}
                    </div>
                  ))}
                  {plotPoints.length > 5 && (
                    <p className="text-xs text-muted-foreground">+{plotPoints.length - 5}</p>
                  )}
                </div>
              )}
            </div>

            {/* Timeline */}
            <div>
              <div className="flex items-center gap-2 mb-2">
                <ClockIcon className="size-4" />
                <span className="text-xs font-medium">{t.timeline.title}</span>
              </div>
              {timelineEvents.length === 0 ? (
                <p className="text-xs text-muted-foreground">{t.timeline.empty}</p>
              ) : (
                <div className="space-y-1">
                  {timelineEvents.slice(0, 5).map((e) => (
                    <div key={e.id} className="text-xs p-2 bg-background rounded">
                      <p className="font-medium">{e.title}</p>
                      {e.event_date && <p className="text-muted-foreground">{e.event_date}</p>}
                    </div>
                  ))}
                  {timelineEvents.length > 5 && (
                    <p className="text-xs text-muted-foreground">+{timelineEvents.length - 5}</p>
                  )}
                </div>
              )}
            </div>

            {/* Research */}
            <div>
              <div className="flex items-center gap-2 mb-2">
                <BookMarkedIcon className="size-4" />
                <span className="text-xs font-medium">{t.research.title}</span>
              </div>
              {researchItems.length === 0 ? (
                <p className="text-xs text-muted-foreground">{t.research.empty}</p>
              ) : (
                <div className="space-y-1">
                  {researchItems.slice(0, 5).map((r) => (
                    <div key={r.id} className="text-xs p-2 bg-background rounded">
                      <p className="font-medium">{r.title}</p>
                      {r.content && <p className="text-muted-foreground line-clamp-1">{r.content}</p>}
                    </div>
                  ))}
                  {researchItems.length > 5 && (
                    <p className="text-xs text-muted-foreground">+{researchItems.length - 5}</p>
                  )}
                </div>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
