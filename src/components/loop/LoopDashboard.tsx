import type { LoopState, LoopPattern } from "@/types";
import { cn } from "@/lib/utils";
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
  idle: "bg-green-500",
  running: "bg-blue-500 animate-pulse",
  paused: "bg-yellow-500",
  error: "bg-red-500",
};

const LEVEL_LABELS: Record<string, string> = {
  L0: "Draft",
  L1: "Report",
  L2: "Assisted",
  L3: "Unattended",
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
  const getPatternName = (patternId: string) =>
    patterns.find((p) => p.id === patternId)?.name ?? patternId;

  const getPatternCadence = (patternId: string) =>
    patterns.find((p) => p.id === patternId)?.cadence ?? "—";

  if (states.length === 0) {
    return (
      <div className="flex items-center justify-center h-40 text-muted-foreground border border-dashed rounded-lg">
        No active loops. Create one to get started.
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
              "border rounded-lg p-3 cursor-pointer hover:shadow-md transition-shadow",
              selectedStateId === ls.id && "ring-2 ring-primary"
            )}
            onClick={() => onSelect(ls.id)}
          >
            <div className="flex items-start justify-between mb-2">
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

            <div className="space-y-1.5 text-xs text-muted-foreground">
              <div className="flex items-center gap-1.5">
                <Clock className="h-3 w-3" />
                <span>Cadence: {getPatternCadence(ls.pattern_id)}</span>
              </div>
              <div className="flex items-center gap-1.5">
                <Zap className="h-3 w-3" />
                <span>Level: {LEVEL_LABELS[ls.readiness_level] ?? ls.readiness_level}</span>
              </div>
              {ls.last_run_at && (
                <div className="text-[10px]">
                  Last run: {new Date(ls.last_run_at).toLocaleString()}
                </div>
              )}
            </div>

            <div className="mt-2">
              <div className="flex items-center justify-between text-[10px] text-muted-foreground mb-1">
                <span>Token usage</span>
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
