/**
 * ═══════════════════════════════════════════════════════════════════════════
 * LoopCard - 循环任务卡片组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { memo } from "react";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Clock,
  Zap,
  Edit2,
  Square,
  Trash2,
  Play,
  Pause,
  AlertCircle,
  CheckCircle,
} from "lucide-react";
import { useI18n } from "@/locales/i18n";
import type { PanelLoopState, LoopStatus } from "@/features/loop/types";

// ── 常量配置 ────────────────────────────────────────────────────────────────

const STATUS_ICONS: Record<LoopStatus, React.ReactNode> = {
  running: <Play className="size-3 text-green-500" />,
  paused: <Pause className="size-3 text-yellow-500" />,
  error: <AlertCircle className="size-3 text-red-500" />,
  idle: <CheckCircle className="size-3 text-muted-foreground" />,
};

const STATUS_VARIANTS: Record<LoopStatus, "default" | "secondary" | "destructive" | "outline"> = {
  running: "default",
  paused: "secondary",
  error: "destructive",
  idle: "outline",
};

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface LoopCardProps {
  loop: PanelLoopState;
  onEdit: (loop: PanelLoopState) => void;
  onStop: (loopId: string) => void;
  onDelete: (loopId: string) => void;
}

// ── 子组件 ──────────────────────────────────────────────────────────────────

/**
 * 状态图标组件
 */
function StatusIcon({ status }: { status: LoopStatus }) {
  return STATUS_ICONS[status];
}

/**
 * 状态徽章组件
 */
const StatusBadge = memo(function StatusBadge({ status, label }: { status: LoopStatus; label: string }) {
  return (
    <Badge variant={STATUS_VARIANTS[status]} className="text-[10px]">
      {label}
    </Badge>
  );
});

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 循环任务卡片，展示单个循环任务的状态和操作按钮
 */
export const LoopCard = memo(function LoopCard({ loop, onEdit, onStop, onDelete }: LoopCardProps) {
  const { t } = useI18n();
  return (
    <Card size="sm">
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="flex items-center gap-1.5 text-xs">
            <StatusIcon status={loop.status} />
            <span className="truncate">{loop.name}</span>
          </CardTitle>
          <StatusBadge status={loop.status} label={t.loop.status[loop.status] || loop.status} />
        </div>
      </CardHeader>
      <CardContent className="flex flex-col gap-2">
        <div className="flex items-center gap-2 text-[10px] text-muted-foreground">
          <Badge variant="outline" className="text-[10px]">
            {loop.triggerType === "timer" ? (
              <><Clock className="size-2.5" data-icon="inline-start" /> {t.loopPanel.timer}</>
            ) : (
              <><Zap className="size-2.5" data-icon="inline-start" /> {t.loopPanel.event}</>
            )}
          </Badge>
          <span>
            {loop.triggerType === "timer"
              ? `${loop.triggerConfig.intervalMs ? Math.round(loop.triggerConfig.intervalMs / 1000) : 0}s`
              : loop.triggerConfig.eventType || "-"}
          </span>
        </div>

        {loop.nextWakeTime && (
          <p className="text-[10px] text-muted-foreground">
            {t.loopPanel.nextWake}: {new Date(loop.nextWakeTime).toLocaleString()}
          </p>
        )}

        <div className="flex items-center justify-between text-[10px] text-muted-foreground">
          <span>{t.loopPanel.iterations}: {loop.iterations}/{loop.maxIterations || "∞"}</span>
        </div>

        <div className="flex gap-1 mt-1">
          <Button
            size="xs"
            variant="ghost"
            onClick={() => onEdit(loop)}
            className="flex-1"
          >
            <Edit2 className="size-3" />
          </Button>
          {loop.status === "running" && (
            <Button
              size="xs"
              variant="ghost"
              onClick={() => onStop(loop.id)}
              className="flex-1"
            >
              <Square className="size-3" />
            </Button>
          )}
          <Button
            size="xs"
            variant="ghost"
            onClick={() => onDelete(loop.id)}
            className="flex-1 text-destructive hover:text-destructive"
          >
            <Trash2 className="size-3" />
          </Button>
        </div>
      </CardContent>
    </Card>
  );
});