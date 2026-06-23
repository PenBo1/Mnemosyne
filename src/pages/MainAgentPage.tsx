import { useState, useEffect, useRef } from "react";
import { useMainAgentStore } from "@/stores/main-agent";
import { listenToMainAgentEvents } from "@/services/main-agent";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  AlertTriangle,
  CheckCircle2,
  Circle,
  Loader2,
  Send,
  StopCircle,
  XCircle,
  ChevronRight,
  Zap,
} from "lucide-react";
import type { MainAgentEvent, PlanStep } from "@/types/main-agent";

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

  // Listen to main agent events
  useEffect(() => {
    const unlisten = listenToMainAgentEvents((event: MainAgentEvent) => {
      handleEvent(event);
    });
    return () => {
      unlisten();
    };
  }, [handleEvent]);

  // Auto-scroll on new messages
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
    <div className="flex h-full">
      {/* Left: Chat / Execution Panel */}
      <div className="flex-1 flex flex-col border-r">
        {/* Header */}
        <div className="border-b px-4 py-3 flex items-center justify-between">
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
        <ScrollArea className="flex-1" ref={scrollRef}>
          <div className="p-4 space-y-4">
            {activeSession ? (
              activeSession.messages.map((msg) => (
                <MessageBubble key={msg.id} message={msg} />
              ))
            ) : (
              <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
                <Zap className="h-12 w-12 mb-4 opacity-50" />
                <p className="text-lg font-medium">Main Agent</p>
                <p className="text-sm">
                  Enter a goal and the agent will autonomously plan and execute
                  it.
                </p>
              </div>
            )}

            {/* Streaming indicator */}
            {loading && activeSession?.status !== "WaitingForConfirmation" && (
              <div className="flex items-center gap-2 text-muted-foreground">
                <Loader2 className="h-4 w-4 animate-spin" />
                <span className="text-sm">{activeSession?.status ?? "Starting..."}</span>
              </div>
            )}
          </div>
        </ScrollArea>

        {/* Confirmation bar */}
        {activeSession?.confirmation && (
          <ConfirmationBar
            confirmation={activeSession.confirmation}
            onApprove={() => respondToConfirmation(true)}
            onReject={() => respondToConfirmation(false)}
          />
        )}

        {/* Input */}
        <div className="border-t p-4">
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

      {/* Right: Plan & Progress Panel */}
      <div className="w-[360px] flex flex-col">
        <div className="border-b px-4 py-3">
          <h2 className="text-sm font-semibold">Execution Plan</h2>
        </div>

        <ScrollArea className="flex-1">
          <div className="p-4">
            {activeSession && activeSession.status !== "Idle" ? (
              <PlanView
                plan={activeSession.plan}
                currentStep={activeSession.currentStep}
                status={activeSession.status}
                result={activeSession.result}
                error={activeSession.error}
              />
            ) : (
              <p className="text-sm text-muted-foreground">
                No active execution. Enter a goal to start.
              </p>
            )}
          </div>
        </ScrollArea>
      </div>
    </div>
  );
}

// ── Sub-components ──────────────────────────────────────────

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

function PlanView({
  plan,
  currentStep,
  status,
  result,
  error,
}: {
  plan: PlanStep[];
  currentStep: number | null;
  status: string;
  result: string | null;
  error: string | null;
}) {
  if (plan.length === 0 && status === "Planning") {
    return (
      <div className="flex items-center gap-2 text-muted-foreground">
        <Loader2 className="h-4 w-4 animate-spin" />
        <span className="text-sm">Creating plan...</span>
      </div>
    );
  }

  if (plan.length === 0) {
    return null;
  }

  return (
    <div className="space-y-2">
      {plan.map((step) => (
        <StepCard
          key={step.id}
          step={step}
          isCurrent={currentStep === step.id}
        />
      ))}

      {/* Result */}
      {result && (
        <Card className="mt-4">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm flex items-center gap-2">
              <CheckCircle2 className="h-4 w-4 text-green-500" />
              Execution Complete
            </CardTitle>
          </CardHeader>
          <CardContent>
            <pre className="text-xs text-muted-foreground whitespace-pre-wrap">
              {result}
            </pre>
          </CardContent>
        </Card>
      )}

      {/* Error */}
      {error && (
        <Card className="mt-4 border-destructive">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm flex items-center gap-2 text-destructive">
              <XCircle className="h-4 w-4" />
              Failed
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-xs text-destructive">{error}</p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

function StepCard({
  step,
  isCurrent,
}: {
  step: PlanStep;
  isCurrent: boolean;
}) {
  const statusIcon = () => {
    switch (step.status) {
      case "Completed":
        return <CheckCircle2 className="h-4 w-4 text-green-500" />;
      case "InProgress":
        return <Loader2 className="h-4 w-4 text-blue-500 animate-spin" />;
      case "Failed":
        return <XCircle className="h-4 w-4 text-destructive" />;
      case "Skipped":
        return <XCircle className="h-4 w-4 text-muted-foreground" />;
      default:
        return <Circle className="h-4 w-4 text-muted-foreground" />;
    }
  };

  const riskBadge = () => {
    if (step.risk_level === "Safe") return null;
    return (
      <Badge
        variant={step.risk_level === "High" ? "destructive" : "secondary"}
        className="text-[10px] px-1 py-0"
      >
        {step.risk_level}
      </Badge>
    );
  };

  return (
    <div
      className={`rounded-lg border p-3 text-sm ${
        isCurrent ? "border-primary bg-primary/5" : ""
      } ${step.status === "Skipped" ? "opacity-50" : ""}`}
    >
      <div className="flex items-start gap-2">
        {statusIcon()}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="text-xs text-muted-foreground">Step {step.id}</span>
            {riskBadge()}
          </div>
          <p className="mt-1">{step.description}</p>
          {step.tool_name && (
            <p className="text-xs text-muted-foreground mt-1">
              <ChevronRight className="h-3 w-3 inline" />
              {step.tool_name}
            </p>
          )}
          {step.result && step.status === "Completed" && (
            <p className="text-xs text-green-600 mt-1">
              {step.result.length > 100
                ? step.result.slice(0, 100) + "..."
                : step.result}
            </p>
          )}
          {step.result && step.status === "Failed" && (
            <p className="text-xs text-destructive mt-1">{step.result}</p>
          )}
        </div>
      </div>
    </div>
  );
}
