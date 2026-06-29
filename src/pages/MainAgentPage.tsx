import { useCallback, useEffect, useRef, useState } from "react";
import { useMainAgent } from "@/features/chat/hooks";
import { useI18n } from "@/shared/i18n";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import {
  AlertTriangle,
  CheckCircle2,
  ChevronRight,
  Circle,
  Loader2,
  Send,
  StopCircle,
  XCircle,
  Zap,
} from "lucide-react";
import type { MainAgentMessage, MainAgentSession, PlanStep } from "@/shared/types/main-agent";

// ── 布局常量 ──────────────────────────────────────────
const SIDE_PANEL_WIDTH = 360;
const TEXTAREA_MIN_HEIGHT = 60;
const MESSAGE_BUBBLE_MAX_WIDTH = 80; // 百分比
const STEP_RESULT_PREVIEW_LIMIT = 100; // 步骤结果预览字符数

export default function MainAgentPage() {
  const { t } = useI18n();
  const {
    loading,
    activeSession,
    startExecution,
    respondToConfirmation,
    cancelExecution,
  } = useMainAgent();

  const [goal, setGoal] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);

  // 新消息时自动滚动到底部
  useEffect(() => {
    const el = scrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [activeSession?.messages]);

  const handleSubmit = useCallback(async () => {
    const trimmed = goal.trim();
    if (!trimmed || loading) return;
    setGoal("");
    await startExecution(trimmed);
  }, [goal, loading, startExecution]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        void handleSubmit();
      }
    },
    [handleSubmit],
  );

  const isWaiting = activeSession?.status === "WaitingForConfirmation";

  return (
    <div className="flex h-full">
      {/* 左侧：聊天/执行面板 */}
      <div className="flex-1 flex flex-col border-r">
        <header className="border-b px-4 py-3 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Zap className="size-5 text-primary" />
            <h1 className="text-lg font-semibold">{t.mainAgent.title}</h1>
          </div>
          {activeSession && loading && (
            <Button variant="destructive" size="sm" onClick={cancelExecution}>
              <StopCircle className="size-4" />
              {t.mainAgent.cancel}
            </Button>
          )}
        </header>

        {/* 消息列表 */}
        <ScrollArea className="flex-1" ref={scrollRef}>
          <div className="p-4 flex flex-col gap-4">
            {activeSession ? (
              activeSession.messages.map((msg) => (
                <MessageBubble key={msg.id} message={msg} />
              ))
            ) : (
              <div className="flex flex-col items-center justify-center h-full gap-2 text-muted-foreground">
                <Zap className="size-12 opacity-50" />
                <p className="text-lg font-medium">{t.mainAgent.emptyTitle}</p>
                <p className="text-sm">{t.mainAgent.emptyDesc}</p>
              </div>
            )}

            {/* 流式指示器 */}
            {loading && !isWaiting && (
              <div className="flex items-center gap-2 text-muted-foreground">
                <Loader2 className="size-4 animate-spin" />
                <span className="text-sm">
                  {activeSession?.status
                    ? t.mainAgent.agentStatus[activeSession.status]
                    : t.mainAgent.starting}
                </span>
              </div>
            )}
          </div>
        </ScrollArea>

        {/* 确认栏 */}
        {activeSession?.confirmation && (
          <ConfirmationBar
            confirmation={activeSession.confirmation}
            onApprove={() => respondToConfirmation(true)}
            onReject={() => respondToConfirmation(false)}
          />
        )}

        {/* 输入区 */}
        <div className="border-t p-4">
          <div className="flex gap-2">
            <Textarea
              value={goal}
              onChange={(e) => setGoal(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={t.mainAgent.placeholder}
              className="resize-none"
              style={{ minHeight: TEXTAREA_MIN_HEIGHT }}
              disabled={loading}
            />
            <Button
              onClick={handleSubmit}
              disabled={!goal.trim() || loading}
              size="lg"
            >
              <Send className="size-4" />
            </Button>
          </div>
        </div>
      </div>

      {/* 右侧：计划与进度面板 */}
      <aside className="flex flex-col border-l" style={{ width: SIDE_PANEL_WIDTH }}>
        <div className="border-b px-4 py-3">
          <h2 className="text-sm font-semibold">{t.mainAgent.executionPlan}</h2>
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
                {t.mainAgent.noActiveExecution}
              </p>
            )}
          </div>
        </ScrollArea>
      </aside>
    </div>
  );
}

// ── 子组件 ──────────────────────────────────────────

function MessageBubble({ message }: { message: MainAgentMessage }) {
  const isUser = message.role === "user";
  const bubbleClass = isUser
    ? "bg-primary text-primary-foreground"
    : message.role === "system"
    ? "bg-muted text-muted-foreground text-sm"
    : "bg-card border text-card-foreground";

  return (
    <div className={`flex ${isUser ? "justify-end" : "justify-start"}`}>
      <div
        className={`rounded-lg px-4 py-2 flex flex-col gap-1 ${bubbleClass}`}
        style={{ maxWidth: `${MESSAGE_BUBBLE_MAX_WIDTH}%` }}
      >
        <p className="whitespace-pre-wrap text-sm">{message.content}</p>
        <p className="text-xs opacity-60">
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
  confirmation: NonNullable<MainAgentSession["confirmation"]>;
  onApprove: () => void;
  onReject: () => void;
}) {
  const { t } = useI18n();
  const isHighRisk = confirmation.risk_level === "High";

  return (
    <div
      className={`border-t px-4 py-3 ${
        isHighRisk ? "bg-destructive/10" : "bg-warning/10"
      }`}
    >
      <div className="flex items-start gap-3">
        <AlertTriangle
          className={`size-5 flex-shrink-0 ${
            isHighRisk ? "text-destructive" : "text-warning"
          }`}
        />
        <div className="flex-1 min-w-0 flex flex-col gap-3">
          <div className="flex flex-col gap-1">
            <p className="font-medium text-sm">{confirmation.description}</p>
            <p className="text-xs text-muted-foreground break-all">
              {confirmation.details}
            </p>
          </div>
          <div className="flex gap-2">
            <Button size="sm" onClick={onApprove}>
              <CheckCircle2 className="size-3" />
              {t.mainAgent.approve}
            </Button>
            <Button size="sm" variant="outline" onClick={onReject}>
              <XCircle className="size-3" />
              {t.mainAgent.reject}
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
  status: MainAgentSession["status"];
  result: string | null;
  error: string | null;
}) {
  const { t } = useI18n();

  if (plan.length === 0 && status === "Planning") {
    return (
      <div className="flex items-center gap-2 text-muted-foreground">
        <Loader2 className="size-4 animate-spin" />
        <span className="text-sm">{t.mainAgent.creatingPlan}</span>
      </div>
    );
  }

  if (plan.length === 0) return null;

  return (
    <div className="flex flex-col gap-2">
      {plan.map((step) => (
        <StepCard key={step.id} step={step} isCurrent={currentStep === step.id} />
      ))}

      {result && (
        <Card className="transition-shadow hover:shadow-md">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm flex items-center gap-2">
              <CheckCircle2 className="size-4 text-emerald-600 dark:text-emerald-400" />
              {t.mainAgent.executionComplete}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <pre className="text-xs text-muted-foreground whitespace-pre-wrap">
              {result}
            </pre>
          </CardContent>
        </Card>
      )}

      {error && (
        <Card className="border-destructive transition-shadow hover:shadow-md">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm flex items-center gap-2 text-destructive">
              <XCircle className="size-4" />
              {t.mainAgent.failed}
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

function StepCard({ step, isCurrent }: { step: PlanStep; isCurrent: boolean }) {
  const { t } = useI18n();

  const statusIcon = (() => {
    switch (step.status) {
      case "Completed":
        return <CheckCircle2 className="size-4 text-emerald-600 dark:text-emerald-400" />;
      case "InProgress":
        return <Loader2 className="size-4 text-primary animate-spin" />;
      case "Failed":
        return <XCircle className="size-4 text-destructive" />;
      case "Skipped":
        return <XCircle className="size-4 text-muted-foreground" />;
      default:
        return <Circle className="size-4 text-muted-foreground" />;
    }
  })();

  const riskBadge =
    step.risk_level === "Safe" ? null : (
      <Badge
        variant={step.risk_level === "High" ? "destructive" : "secondary"}
        className="text-[10px] px-1 py-0"
      >
        {t.mainAgent.riskLevel[step.risk_level]}
      </Badge>
    );

  const previewResult =
    step.result && step.status === "Completed"
      ? step.result.length > STEP_RESULT_PREVIEW_LIMIT
        ? `${step.result.slice(0, STEP_RESULT_PREVIEW_LIMIT)}...`
        : step.result
      : null;

  return (
    <div
      className={`rounded-lg border p-3 text-sm ${
        isCurrent ? "border-primary bg-primary/5" : ""
      } ${step.status === "Skipped" ? "opacity-50" : ""}`}
    >
      <div className="flex items-start gap-2">
        {statusIcon}
        <div className="flex-1 min-w-0 flex flex-col gap-1">
          <div className="flex items-center gap-2">
            <span className="text-xs text-muted-foreground">
              {t.mainAgent.stepLabel.replace("{id}", String(step.id))}
            </span>
            {riskBadge}
          </div>
          <p>{step.description}</p>
          {step.tool_name && (
            <p className="text-xs text-muted-foreground">
              <ChevronRight className="size-3 inline" />
              {step.tool_name}
            </p>
          )}
          {previewResult && (
            <p className="text-xs text-emerald-600 dark:text-emerald-400">{previewResult}</p>
          )}
          {step.result && step.status === "Failed" && (
            <p className="text-xs text-destructive">{step.result}</p>
          )}
        </div>
      </div>
    </div>
  );
}
