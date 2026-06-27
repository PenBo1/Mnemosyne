import { useState, useEffect, useRef } from "react";
import { useMainAgentStore } from "@/stores/main-agent";
import { listenToMainAgentEvents } from "@/services/main-agent";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import {
  AlertTriangle,
  CheckCircle2,
  Loader2,
  Send,
  StopCircle,
  XCircle,
  Zap,
} from "lucide-react";
import type { MainAgentEvent } from "@/types/main-agent";

export default function MainAgentPage() {
  const [goal, setGoal] = useState("");
  const {
    sessions,
    activeSessionId,
    loading,
    startExecution,
    respondToConfirmation,
    cancelExecution,
    handleEvent,
  } = useMainAgentStore();
  const scrollRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const activeSession = activeSessionId ? sessions[activeSessionId] : null;

  useEffect(() => {
    const unlisten = listenToMainAgentEvents((event: MainAgentEvent) => {
      handleEvent(event);
    });
    return () => {
      unlisten();
    };
  }, [handleEvent]);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [activeSession?.messages]);

  const handleSubmit = async () => {
    if (!goal.trim() || loading) return;
    const trimmed = goal.trim();
    setGoal("");
    await startExecution(trimmed);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="border-b px-4 py-3 flex items-center justify-between shrink-0">
        <div className="flex items-center gap-2">
          <Zap className="h-5 w-5 text-primary" />
          <h1 className="text-lg font-semibold">Main Agent</h1>
        </div>
        {activeSession && loading && (
          <Button
            variant="destructive"
            size="sm"
            onClick={cancelExecution}
          >
            <StopCircle className="h-4 w-4 mr-1" />
            Cancel
          </Button>
        )}
      </div>

      {/* Messages */}
      <div ref={scrollRef} className="flex-1 overflow-auto md-scrollbar">
        <div className="p-4 space-y-4">
          {activeSession ? (
            activeSession.messages.map((msg) => (
              <MessageBubble key={msg.id} message={msg} />
            ))
          ) : (
            <div className="flex flex-col items-center justify-center h-full text-muted-foreground py-20">
              <Zap className="h-12 w-12 mb-4 opacity-50" />
              <p className="text-lg font-medium">Main Agent</p>
              <p className="text-sm">
                Enter a goal and the agent will autonomously plan and execute
                it.
              </p>
            </div>
          )}

          {loading && activeSession?.status !== "WaitingForConfirmation" && (
            <div className="flex items-center gap-2 text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              <span className="text-sm">{activeSession?.status ?? "Starting..."}</span>
            </div>
          )}
        </div>
      </div>

      {/* Confirmation bar */}
      {activeSession?.confirmation && (
        <ConfirmationBar
          confirmation={activeSession.confirmation}
          onApprove={() => respondToConfirmation(true)}
          onReject={() => respondToConfirmation(false)}
        />
      )}

      {/* Input */}
      <div className="border-t p-4 shrink-0">
        <div className="flex gap-2">
          <Textarea
            ref={textareaRef}
            value={goal}
            onChange={(e) => setGoal(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Enter a goal for the agent..."
            className="min-h-[60px] resize-none"
            disabled={loading}
          />
          <Button
            onClick={handleSubmit}
            disabled={!goal.trim() || loading}
            size="lg"
          >
            <Send className="h-4 w-4" />
          </Button>
        </div>
      </div>
    </div>
  );
}

function MessageBubble({
  message,
}: {
  message: { role: string; content: string; timestamp: string };
}) {
  const isUser = message.role === "user";
  return (
    <div className={`flex ${isUser ? "justify-end" : "justify-start"}`}>
      <div
        className={`max-w-[80%] rounded-lg px-4 py-2 ${
          isUser
            ? "bg-primary text-primary-foreground"
            : message.role === "system"
            ? "bg-muted text-muted-foreground text-sm"
            : "bg-card border text-card-foreground"
        }`}
      >
        <p className="whitespace-pre-wrap text-sm">{message.content}</p>
        <p className="text-xs opacity-60 mt-1">
          {new Date(message.timestamp).toLocaleTimeString()}
        </p>
      </div>
    </div>
  );
}

function ConfirmationBar({
  confirmation,
  onApprove,
  onReject,
}: {
  confirmation: {
    step_id: number;
    description: string;
    details: string;
    risk_level: string;
  };
  onApprove: () => void;
  onReject: () => void;
}) {
  const isHighRisk = confirmation.risk_level === "High";

  return (
    <div
      className={`border-t px-4 py-3 ${
        isHighRisk ? "bg-destructive/10" : "bg-yellow-500/10"
      }`}
    >
      <div className="flex items-start gap-3">
        <AlertTriangle
          className={`h-5 w-5 mt-0.5 flex-shrink-0 ${
            isHighRisk ? "text-destructive" : "text-yellow-600"
          }`}
        />
        <div className="flex-1 min-w-0">
          <p className="font-medium text-sm">{confirmation.description}</p>
          <p className="text-xs text-muted-foreground mt-1 break-all">
            {confirmation.details}
          </p>
          <div className="flex gap-2 mt-3">
            <Button size="sm" onClick={onApprove}>
              <CheckCircle2 className="h-3 w-3 mr-1" />
              Approve
            </Button>
            <Button size="sm" variant="outline" onClick={onReject}>
              <XCircle className="h-3 w-3 mr-1" />
              Reject
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
