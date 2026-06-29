import { useMemo, useRef, useState } from "react";
import { useChat } from "@/features/chat/hooks/useChat";
import { useI18n } from "@/shared/i18n";
import { ChatTopBar } from "@/features/chat/components/ChatTopBar";
import { MessageList } from "@/features/chat/components/MessageList";
import { ChatInput } from "@/features/chat/components/ChatInput";
import { MessageTimeline } from "@/features/chat/components/MessageTimeline";

export default function ChatPage() {
  const { t } = useI18n();
  const {
    activeSession,
    messages,
    streaming,
    streamingContent,
    error,
    sendMessage,
    cancel,
    handleNewSession,
    handleDeleteSession,
  } = useChat();

  const [input, setInput] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);

  const userMessages = useMemo(
    () =>
      messages
        .filter((m) => m.role === "user")
        .map((m) => ({ id: m.id, content: m.content })),
    [messages]
  );

  const handleSubmit = () => {
    const trimmed = input.trim();
    if (!trimmed || streaming) return;
    setInput("");
    void sendMessage(trimmed);
  };

  const title = activeSession?.title || t.agentChat.title;

  return (
    <div className="flex h-full bg-background">
      {/* 主聊天区 */}
      <main className="flex flex-1 flex-col min-w-0">
        <ChatTopBar
          title={title}
          streaming={streaming}
          hasSession={!!activeSession}
          onNewSession={handleNewSession}
          onDeleteSession={handleDeleteSession}
        />
        <MessageList
          messages={messages}
          streaming={streaming}
          streamingContent={streamingContent}
          error={error}
          scrollRef={scrollRef}
        />
        <ChatInput
          value={input}
          onChange={setInput}
          onSubmit={handleSubmit}
          onCancel={cancel}
          streaming={streaming}
        />
      </main>

      {/* 右侧消息导航时间轴 */}
      <MessageTimeline userMessages={userMessages} scrollRef={scrollRef} />
    </div>
  );
}
