import type { ReactNode } from "react";
import { useI18n } from "@/shared/i18n";
import type { LoopRunLog as LoopRunLogType } from "@/shared/types";
import { CheckCircle, XCircle, AlertTriangle, Clock } from "lucide-react";

interface LoopRunLogProps {
  logs: LoopRunLogType[];
  selectedStateId: string | null;
}

const STATUS_ICONS: Record<string, ReactNode> = {
  success: <CheckCircle className="h-3.5 w-3.5 text-[var(--status-success-default)]" />,
  partial: <AlertTriangle className="h-3.5 w-3.5 text-muted-foreground" />,
  failed: <XCircle className="h-3.5 w-3.5 text-destructive" />,
  escalated: <AlertTriangle className="h-3.5 w-3.5 text-[var(--status-warning-default)]" />,
};

export function LoopRunLog({ logs, selectedStateId }: LoopRunLogProps) {
  const { t } = useI18n();
  const filteredLogs = selectedStateId
    ? logs.filter((l) => l.loop_state_id === selectedStateId)
    : logs;

  return (
    <div className="border rounded-lg">
      <div className="px-3 py-2 border-b">
        <span className="text-sm font-medium">{t.loop.runLogs}</span>
      </div>

      <div className="max-h-[500px] overflow-y-auto">
        {filteredLogs.length === 0 ? (
          <div className="flex items-center justify-center h-32 text-xs text-muted-foreground">
            {t.loop.common.noLogs}
          </div>
        ) : (
          <div className="divide-y">
            {filteredLogs.map((log) => (
              <div key={log.id} className="flex flex-col gap-1 px-3 py-2 hover:bg-muted/50">
                <div className="flex items-center gap-2">
                  {STATUS_ICONS[log.status]}
                  <span className="text-xs font-medium">
                    {t.loop.logStatus[log.status]}
                  </span>
                  <span className="text-[10px] text-muted-foreground ml-auto">
                    {new Date(log.created_at).toLocaleString()}
                  </span>
                </div>

                <div className="flex items-center gap-3 text-[10px] text-muted-foreground">
                  <span className="flex items-center gap-1">
                    <Clock className="h-2.5 w-2.5" />
                    {(log.duration_ms / 1000).toFixed(1)}s
                  </span>
                  <span>
                    {log.tokens_used.toLocaleString()} {t.loop.metrics.tokens}
                  </span>
                </div>

                {log.findings.length > 0 && (
                  <div className="text-[10px] flex flex-col gap-0.5">
                    {log.findings.slice(0, 3).map((f, i) => (
                      <div key={i} className="text-muted-foreground">
                        • {f}
                      </div>
                    ))}
                    {log.findings.length > 3 && (
                      <div className="text-muted-foreground">
                        +{log.findings.length - 3} {t.loop.metrics.more}
                      </div>
                    )}
                  </div>
                )}

                {log.escalations.length > 0 && (
                  <div className="text-[10px] text-orange-600">
                    ⚠ {log.escalations.length} {t.loop.metrics.escalations}
                  </div>
                )}

                {log.error_message && (
                  <div className="text-[10px] text-destructive">
                    {log.error_message}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
