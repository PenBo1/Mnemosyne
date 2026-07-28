/**
 * ═══════════════════════════════════════════════════════════════════════════
 * PipelineProgress - 流水线进度组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useI18n } from "@/locales/i18n";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import { Timeline, TimelineItem, TimelineSeparator, TimelineConnector, TimelineContent } from "@/components/ui/timeline";
import {
  CheckCircleIcon,
  LoaderIcon,
  XCircleIcon,
  CircleIcon,
  ZapIcon,
} from "lucide-react";
import { cn } from "@/lib/utils";

export type PipelineStepStatus = "pending" | "running" | "completed" | "failed";

export interface PipelineStepInfo {
  name: string;
  status: PipelineStepStatus;
  duration?: number;
  message?: string;
}

interface PipelineProgressProps {
  steps: PipelineStepInfo[];
  currentStep: number;
  totalSteps: number;
  startTime?: Date;
  estimatedTime?: number;
}

const STATUS_ICONS: Record<PipelineStepStatus, React.ReactNode> = {
  pending: <CircleIcon className="size-4 text-muted-foreground" />,
  running: <LoaderIcon className="size-4 animate-spin text-primary" />,
  completed: <CheckCircleIcon className="size-4 text-success" />,
  failed: <XCircleIcon className="size-4 text-destructive" />,
};

const STATUS_COLORS: Record<PipelineStepStatus, string> = {
  pending: "bg-muted",
  running: "bg-primary/20",
  completed: "bg-success/20",
  failed: "bg-destructive/20",
};

type StepName = "plan" | "compose" | "write" | "audit" | "revise" | "validate" | "reflect" | "consolidate";

export function PipelineProgress({
  steps,
  currentStep,
  totalSteps,
  startTime,
  estimatedTime,
}: PipelineProgressProps) {
  const { t } = useI18n();

  const progressPercent = Math.round((currentStep / totalSteps) * 100);

  const formatDuration = (ms: number): string => {
    const seconds = Math.floor(ms / 1000);
    const minutes = Math.floor(seconds / 60);
    if (minutes > 0) {
      return `${minutes}m ${seconds % 60}s`;
    }
    return `${seconds}s`;
  };

  const elapsedMs = startTime ? Date.now() - startTime.getTime() : 0;

  const getStepLabel = (name: string): string => {
    const validSteps: StepName[] = ["plan", "compose", "write", "audit", "revise", "validate", "reflect", "consolidate"];
    if (validSteps.includes(name as StepName)) {
      return t.pipeline.steps[name as StepName];
    }
    return name;
  };

  return (
    <Card className="p-4">
      <div className="flex items-center justify-between gap-2 mb-4">
        <div className="flex items-center gap-2">
          <ZapIcon className="size-5 text-muted-foreground" />
          <span className="font-medium">{t.pipeline.progress}</span>
        </div>
        <Badge variant="secondary">
          {currentStep}/{totalSteps}
        </Badge>
      </div>

      <div className="flex flex-col gap-4">
        <div className="flex items-center gap-2">
          <Progress value={progressPercent} className="h-2" />
          <span className="text-xs text-muted-foreground w-8 text-right">
            {progressPercent}%
          </span>
        </div>

        {startTime && (
          <div className="flex items-center justify-between text-xs text-muted-foreground">
            <span>{t.pipeline.elapsed}: {formatDuration(elapsedMs)}</span>
            {estimatedTime && (
              <span>{t.pipeline.estimated}: {formatDuration(estimatedTime)}</span>
            )}
          </div>
        )}

        <Timeline>
          {steps.map((step, index) => (
            <TimelineItem key={step.name}>
              <TimelineSeparator>
                {STATUS_ICONS[step.status]}
                {index < steps.length - 1 && (
                  <TimelineConnector
                    className={cn(
                      step.status === "completed" ? "bg-success" : "bg-muted",
                    )}
                  />
                )}
              </TimelineSeparator>
              <TimelineContent>
                <div className="flex items-center justify-between gap-2">
                  <span
                    className={cn(
                      "text-sm",
                      step.status === "running" && "font-medium text-primary",
                      step.status === "completed" && "text-success",
                      step.status === "failed" && "text-destructive",
                    )}
                  >
                    {getStepLabel(step.name)}
                  </span>
                  <div className="flex items-center gap-2">
                    {step.duration && (
                      <span className="text-xs text-muted-foreground">
                        {formatDuration(step.duration)}
                      </span>
                    )}
                    <Badge
                      variant="outline"
                      className={cn("text-xs", STATUS_COLORS[step.status])}
                    >
                      {t.pipeline.statuses[step.status]}
                    </Badge>
                  </div>
                </div>
                {step.message && (
                  <p className="text-xs text-muted-foreground mt-1">
                    {step.message}
                  </p>
                )}
              </TimelineContent>
            </TimelineItem>
          ))}
        </Timeline>
      </div>
    </Card>
  );
}