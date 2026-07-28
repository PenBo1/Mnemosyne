/**
 * ═══════════════════════════════════════════════════════════════════════════
 * SchedulerPanel - 调度器面板组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useI18n } from "@/locales/i18n";
import { usePipeline } from "@/features/pipeline/hooks";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { PlayIcon, SquareIcon, ZapIcon } from "lucide-react";

export function SchedulerPanel() {
  const { t } = useI18n();
  const { schedulerStatus, startScheduler, stopScheduler, triggerWriteCycle, running } = usePipeline();

  const isRunning = schedulerStatus?.running ?? false;

  return (
    <Card className="p-4">
      <div className="flex items-center justify-between gap-2">
        <div className="flex flex-col gap-1">
          <div className="flex items-center gap-2">
            <span className="font-medium">{t.pipeline.scheduler}</span>
            <Badge variant={isRunning ? "default" : "secondary"}>
              {isRunning ? t.pipeline.status.running : t.pipeline.status.idle}
            </Badge>
          </div>
          {schedulerStatus && (
            <div className="text-xs text-muted-foreground">
              {t.pipeline.maxConcurrent}: {schedulerStatus.config.max_concurrent_books} ·{" "}
              {t.pipeline.chaptersPerCycle}: {schedulerStatus.config.chapters_per_cycle}
            </div>
          )}
        </div>
        <div className="flex items-center gap-2">
          {isRunning ? (
            <Button variant="outline" size="sm" onClick={stopScheduler}>
              <SquareIcon className="size-4" />
              {t.pipeline.stop}
            </Button>
          ) : (
            <Button size="sm" onClick={startScheduler}>
              <PlayIcon className="size-4" />
              {t.pipeline.start}
            </Button>
          )}
          <Button variant="outline" size="sm" onClick={triggerWriteCycle} disabled={running}>
            <ZapIcon className="size-4" />
            {t.pipeline.triggerNow}
          </Button>
        </div>
      </div>
    </Card>
  );
}
