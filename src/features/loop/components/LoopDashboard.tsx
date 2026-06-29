import { useI18n } from "@/shared/i18n";
import type { LoopState, LoopPattern } from "@/shared/types";
import { cn } from "@/shared/utils";
import { Play, Pause, PlayCircle, Trash2, Clock, Zap } from "lucide-react";
import { Button } from "@/components/ui/button";

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
      <div className="flex items-center justify-center h-40 text-muted-foreground border border-dashed rounded-[var(--radius-6)]">
        {t.loop.common.noLoops}
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
      {states.map((ls) => {
        const usagePercent =
          ls.token_cap_daily > 0
            ? Math.round((ls.token_usage_today / ls.token_cap_daily) * 100)
            : 0;

        return (
          <div
            key={ls.id}
            className={cn(
              "border rounded-[var(--radius-6)] p-3 cursor-pointer transition-shadow flex flex-col gap-2",
              selectedStateId === ls.id && "ring-2 ring-primary"
            )}
            onClick={() => onSelect(ls.id)}
          >
            <div className="flex items-start justify-between">
              <div className="flex items-center gap-2">
                <div
                  className={cn(
                    "w-2.5 h-2.5 rounded-full",
                    STATUS_STYLES[ls.status] ?? "bg-gray-400"
                  )}
                />
                <span className="text-sm font-medium">
                  {getPatternName(ls.pattern_id)}
                </span>
              </div>
              <div className="flex items-center gap-1">
                {ls.status === "idle" && (
                  <>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-6 w-6"
                      onClick={(e) => {
                        e.stopPropagation();
                        onRun(ls.id);
                      }}
                    >
                      <Play className="h-3 w-3" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-6 w-6"
                      onClick={(e) => {
                        e.stopPropagation();
                        onPause(ls.id);
                      }}
                    >
                      <Pause className="h-3 w-3" />
                    </Button>
                  </>
                )}
                {ls.status === "paused" && (
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-6 w-6"
                    onClick={(e) => {
                      e.stopPropagation();
                      onResume(ls.id);
                    }}
                  >
                    <PlayCircle className="h-3 w-3" />
                  </Button>
                )}
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-6 w-6 text-destructive"
                  onClick={(e) => {
                    e.stopPropagation();
                    onDelete(ls.id);
                  }}
                >
                  <Trash2 className="h-3 w-3" />
                </Button>
              </div>
            </div>

            <div className="flex flex-col gap-1.5 text-xs text-muted-foreground">
              <div className="flex items-center gap-1.5">
                <Clock className="h-3 w-3" />
                <span>{getPatternCadence(ls.pattern_id)}</span>
              </div>
              <div className="flex items-center gap-1.5">
                <Zap className="h-3 w-3" />
                <span>{getReadinessLabel(ls.readiness_level)}</span>
              </div>
              {ls.last_run_at && (
                <div className="text-[10px]">
                  {new Date(ls.last_run_at).toLocaleString()}
                </div>
              )}
            </div>

            <div className="flex flex-col gap-1">
              <div className="flex items-center justify-between text-[10px] text-muted-foreground">
                <span>{t.loop.budget.used}</span>
                <span>
                  {ls.token_usage_today.toLocaleString()} / {ls.token_cap_daily.toLocaleString()}
                </span>
              </div>
              <div className="h-1.5 bg-muted rounded-full overflow-hidden">
                <div
                  className={cn(
                    "h-full rounded-full transition-all",
                    usagePercent > 80 ? "bg-destructive" : "bg-primary"
                  )}
                  style={{ width: `${Math.min(usagePercent, 100)}%` }}
                />
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}
