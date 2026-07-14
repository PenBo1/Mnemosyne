import { useI18n } from "@/locales/i18n";
import type { LoopState, LoopPattern } from "@/features/loop/types";
import { cn } from "@/lib/utils";
import { Play, Pause, PlayCircle, Trash2, Clock, Zap } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { EmptyState } from "@/components/shared/state";

interface LoopDashboardProps {
  states: LoopState[];
  patterns: LoopPattern[];
  onRun: (stateId: string) => void;
  onPause: (stateId: string) => void;
  onResume: (stateId: string) => void;
  onDelete: (stateId: string) => void;
  onSelect: (stateId: string) => void;
  selectedStateId: string | null;
}

const STATUS_STYLES: Record<string, string> = {
  idle: "bg-primary",
  running: "bg-primary animate-pulse",
  paused: "bg-muted-foreground",
  error: "bg-destructive",
};

export function LoopDashboard({
  states,
  patterns,
  onRun,
  onPause,
  onResume,
  onDelete,
  onSelect,
  selectedStateId,
}: LoopDashboardProps) {
  const { t } = useI18n();

  const getPatternName = (patternId: string) =>
    patterns.find((p) => p.id === patternId)?.name ?? patternId;

  const getPatternCadence = (patternId: string) =>
    patterns.find((p) => p.id === patternId)?.cadence ?? "—";

  const getReadinessLabel = (level: string) =>
    t.loop.readiness[level as keyof typeof t.loop.readiness];

  if (states.length === 0) {
    return (
      <EmptyState
        title={t.loop.common.noLoops}
        className="h-40 border border-dashed rounded-[var(--radius-6)]"
      />
    );
  }

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
      {states.map((ls) => {
        const usagePercent =
          ls.tokenCapDaily > 0
            ? Math.round((ls.tokenUsageToday / ls.tokenCapDaily) * 100)
            : 0;

        return (
          <Card
            key={ls.id}
            size="sm"
            className={cn(
              "cursor-pointer transition-shadow",
              selectedStateId === ls.id && "ring-2 ring-primary"
            )}
            onClick={() => onSelect(ls.id)}
          >
            <CardContent className="flex flex-col gap-2">
              <div className="flex items-start justify-between">
                <div className="flex items-center gap-2">
                  <div
                    className={cn(
                      "size-2.5 rounded-full",
                      STATUS_STYLES[ls.status] ?? "bg-muted-foreground"
                    )}
                  />
                  <span className="text-sm font-medium">
                    {getPatternName(ls.patternId)}
                  </span>
                </div>
                <div className="flex items-center gap-1">
                  {ls.status === "idle" && (
                    <>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="size-6"
                        onClick={(e) => {
                          e.stopPropagation();
                          onRun(ls.id);
                        }}
                      >
                        <Play className="size-3" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="size-6"
                        onClick={(e) => {
                          e.stopPropagation();
                          onPause(ls.id);
                        }}
                      >
                        <Pause className="size-3" />
                      </Button>
                    </>
                  )}
                  {ls.status === "paused" && (
                    <Button
                      variant="ghost"
                      size="icon"
                      className="size-6"
                      onClick={(e) => {
                        e.stopPropagation();
                        onResume(ls.id);
                      }}
                    >
                      <PlayCircle className="size-3" />
                    </Button>
                  )}
                  <Button
                    variant="ghost"
                    size="icon"
                    className="size-6 text-destructive"
                    onClick={(e) => {
                      e.stopPropagation();
                      onDelete(ls.id);
                    }}
                  >
                    <Trash2 className="size-3" />
                  </Button>
                </div>
              </div>

              <div className="flex flex-col gap-1.5 text-xs text-muted-foreground">
                <div className="flex items-center gap-1.5">
                  <Clock className="size-3" />
                  <span>{getPatternCadence(ls.patternId)}</span>
                </div>
                <div className="flex items-center gap-1.5">
                  <Zap className="size-3" />
                  <span>{getReadinessLabel(ls.readinessLevel)}</span>
                </div>
                {ls.lastRunAt && (
                  <div className="text-[10px]">
                    {new Date(ls.lastRunAt).toLocaleString()}
                  </div>
                )}
              </div>

              <div className="flex flex-col gap-1">
                <div className="flex items-center justify-between text-[10px] text-muted-foreground">
                  <span>{t.loop.budget.used}</span>
                  <span>
                    {ls.tokenUsageToday.toLocaleString()} / {ls.tokenCapDaily.toLocaleString()}
                  </span>
                </div>
                <Progress
                  value={usagePercent}
                  className={cn("h-1.5", usagePercent > 80 && "[&>div]:bg-destructive")}
                />
              </div>
            </CardContent>
          </Card>
        );
      })}
    </div>
  );
}
