/**
 * ═══════════════════════════════════════════════════════════════════════════
 * PipelineProgress - 流水线进度展示组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { memo } from "react";
import { useI18n } from "@/locales/i18n";
import { cn } from "@/lib/utils";
import { CheckCircle2, Circle, Loader2 } from "lucide-react";

// ── 类型定义 ────────────────────────────────────────────────────────────────

export type PipelinePhase =
  | "plan"
  | "compose"
  | "write"
  | "audit"
  | "revise"
  | "settle";

export interface PipelineStep {
  phase: PipelinePhase;
  subagentName: string;
  status: "pending" | "running" | "completed" | "error";
  startedAt?: number;
  completedAt?: number;
}

export interface PipelineState {
  currentChapter: number;
  steps: PipelineStep[];
  currentStepIndex: number;
  startTime?: number;
  endTime?: number;
}

interface PipelineProgressProps {
  pipelineState: PipelineState | null;
  className?: string;
}

interface PipelineStepItemProps {
  step: PipelineStep;
  isCurrentStep: boolean;
}

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/**
 * 获取状态对应的图标组件
 */
function getStatusIcon(status: PipelineStep["status"]) {
  switch (status) {
    case "completed":
      return (
        <CheckCircle2 className="h-4 w-4 text-green-500" aria-hidden="true" />
      );
    case "running":
      return (
        <Loader2 className="h-4 w-4 animate-spin text-blue-500" aria-hidden="true" />
      );
    case "error":
      return (
        <Circle className="h-4 w-4 text-red-500" aria-hidden="true" />
      );
    default:
      return (
        <Circle className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
      );
  }
}

// ── 子组件 ──────────────────────────────────────────────────────────────────

/**
 * 流水线步骤项组件
 */
const PipelineStepItem = memo(function PipelineStepItem({
  step,
  isCurrentStep,
}: PipelineStepItemProps) {
  const { t } = useI18n();
  const statusIcon = getStatusIcon(step.status);

  return (
    <div
      className={cn(
        "flex items-center gap-2 rounded-md px-2 py-1.5 transition-colors",
        isCurrentStep && "bg-muted",
        step.status === "running" && "animate-pulse"
      )}
    >
      {statusIcon}
      <div className="flex flex-1 items-center justify-between">
        <span className={cn(
          "text-sm",
          step.status === "running" && "font-medium text-foreground",
          step.status === "completed" && "text-muted-foreground",
          step.status === "pending" && "text-muted-foreground"
        )}>
          {t.pipeline.phases[step.phase]}
        </span>
        <span className={cn(
          "text-xs",
          step.status === "running" && "text-blue-500",
          step.status === "completed" && "text-green-500",
          step.status === "error" && "text-red-500"
        )}>
          {step.subagentName}
        </span>
      </div>
      {step.status === "running" && (
        <div className="flex items-center gap-1">
          <span className="relative flex h-2 w-2">
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-blue-400 opacity-75" />
            <span className="relative inline-flex h-2 w-2 rounded-full bg-blue-500" />
          </span>
        </div>
      )}
    </div>
  );
});

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 流水线进度展示组件，显示章节写作流水线的各阶段状态
 */
const PipelineProgress = memo(function PipelineProgress({
  pipelineState,
  className,
}: PipelineProgressProps) {
  const { t } = useI18n();

  if (!pipelineState) {
    return null;
  }

  const { currentChapter, steps, currentStepIndex } = pipelineState;
  const completedSteps = steps.filter((s) => s.status === "completed").length;
  const totalSteps = steps.length;
  const progress = totalSteps > 0 ? (completedSteps / totalSteps) * 100 : 0;

  return (
    <div className={cn("rounded-lg border bg-card p-4", className)}>
      <div className="mb-3 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h3 className="text-sm font-medium">
            {t.pipeline.title} - {t.pipeline.chapter} {currentChapter}
          </h3>
          <span className="text-xs text-muted-foreground">
            {completedSteps}/{totalSteps}
          </span>
        </div>
        <div className="text-xs text-muted-foreground">
          {Math.round(progress)}%
        </div>
      </div>

      <div className="mb-4 h-2 w-full overflow-hidden rounded-full bg-secondary">
        <div
          className="h-full bg-primary transition-all duration-300"
          style={{ width: `${progress}%` }}
        />
      </div>

      <div className="space-y-2">
        {steps.map((step, index) => (
          <PipelineStepItem
            key={step.phase}
            step={step}
            isCurrentStep={index === currentStepIndex}
          />
        ))}
      </div>
    </div>
  );
});

export { PipelineProgress };

// ── 工具函数 ────────────────────────────────────────────────────────────────

/**
 * 创建初始流水线状态
 */
export function createInitialPipelineState(chapter: number): PipelineState {
  return {
    currentChapter: chapter,
    steps: [
      { phase: "plan", subagentName: "Planner", status: "pending" },
      { phase: "compose", subagentName: "Composer", status: "pending" },
      { phase: "write", subagentName: "Writer", status: "pending" },
      { phase: "audit", subagentName: "Auditor", status: "pending" },
      { phase: "revise", subagentName: "Reviser", status: "pending" },
      { phase: "settle", subagentName: "Settler", status: "pending" },
    ],
    currentStepIndex: 0,
    startTime: Date.now(),
  };
}

/**
 * 更新流水线步骤状态
 */
export function updatePipelineStep(
  state: PipelineState,
  stepIndex: number,
  updates: Partial<PipelineStep>
): PipelineState {
  const newSteps = [...state.steps];
  if (stepIndex >= 0 && stepIndex < newSteps.length) {
    newSteps[stepIndex] = { ...newSteps[stepIndex], ...updates };
  }
  return {
    ...state,
    steps: newSteps,
    currentStepIndex: stepIndex,
  };
}

/**
 * 推进到下一步骤
 */
export function advanceToNextStep(state: PipelineState): PipelineState {
  const nextIndex = state.currentStepIndex + 1;
  if (nextIndex >= state.steps.length) {
    return {
      ...state,
      endTime: Date.now(),
    };
  }
  const newSteps = [...state.steps];
  newSteps[nextIndex] = { ...newSteps[nextIndex], status: "running", startedAt: Date.now() };
  return {
    ...state,
    steps: newSteps,
    currentStepIndex: nextIndex,
  };
}