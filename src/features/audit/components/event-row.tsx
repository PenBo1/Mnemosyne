/**
 * ═══════════════════════════════════════════════════════════════════════════
 * EventRow - 审计事件列表行组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { memo } from "react";
import { Badge } from "@/components/ui/badge";
import {
  EyeIcon,
  ShieldIcon,
  ShieldAlertIcon,
  ShieldXIcon,
} from "lucide-react";
import type { AuditEventRow } from "../types";
import { DENIED_EVENT_TYPES, SECURITY_EVENT_TYPES_SET } from "../types";

// ── 常量配置 ────────────────────────────────────────────────────────────────

/**
 * 事件类型与颜色映射表
 */
export const EVENT_TYPE_COLOR: Record<string, string> = {
  operation_start: "text-emerald-500",
  operation_complete: "text-emerald-600",
  approval_requested: "text-amber-500",
  approval_granted: "text-emerald-500",
  approval_rejected: "text-rose-500",
  policy_denied: "text-rose-600",
  rate_limited: "text-orange-500",
  resource_exceeded: "text-orange-600",
  plugin_loaded: "text-blue-500",
  plugin_unloaded: "text-blue-400",
  quota_set: "text-purple-500",
  override_applied: "text-purple-400",
  override_expired: "text-purple-300",
};

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface EventRowProps {
  event: AuditEventRow;
  onSelect: (id: string) => void;
  isSelected: boolean;
  labels: {
    viewDetails: string;
    workspace: string;
    denied: string;
    security: string;
  };
}

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/**
 * 格式化时间为相对时间
 */
function formatTime(iso: string): string {
  const d = new Date(iso);
  const now = new Date();
  const diffMs = now.getTime() - d.getTime();
  const diffMin = Math.floor(diffMs / 60_000);
  if (diffMin < 1) return "now";
  if (diffMin < 60) return `${diffMin}m`;
  const diffH = Math.floor(diffMin / 60);
  if (diffH < 24) return `${diffH}h`;
  return d.toLocaleDateString();
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 审计事件列表行，展示单个事件并支持点击查看详情
 */
export const EventRow = memo(function EventRow({ event, onSelect, isSelected, labels }: EventRowProps) {
  const colorClass = EVENT_TYPE_COLOR[event.eventType] ?? "text-muted-foreground";
  const isDenied = DENIED_EVENT_TYPES.has(event.eventType) || event.isDenied;
  const isSecurity = SECURITY_EVENT_TYPES_SET.has(event.eventType) || event.isSecurityRelated;

  return (
    <button
      onClick={() => onSelect(event.id)}
      className={`flex w-full items-center gap-3 rounded-md border px-3 py-2 text-left transition hover:bg-accent ${
        isSelected ? "border-primary bg-accent" : "border-border"
      }`}
    >
      <div className={`flex size-8 items-center justify-center rounded-full ${colorClass}`}>
        {isDenied ? (
          <ShieldXIcon className="size-4" />
        ) : isSecurity ? (
          <ShieldAlertIcon className="size-4" />
        ) : (
          <ShieldIcon className="size-4" />
        )}
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className={`text-sm font-medium ${colorClass}`}>
            {event.eventType}
          </span>
          {event.operation && (
            <Badge variant="outline" className="font-mono text-xs">
              {event.operation}
            </Badge>
          )}
          {isDenied && (
            <Badge variant="destructive" className="text-xs">
              {labels.denied}
            </Badge>
          )}
          {isSecurity && !isDenied && (
            <Badge variant="secondary" className="text-xs text-amber-500">
              {labels.security}
            </Badge>
          )}
        </div>
        <div className="mt-0.5 truncate text-xs text-muted-foreground">
          {event.workspaceId ? (
            <>{labels.workspace}: {event.workspaceId.slice(0, 8)}…</>
          ) : (
            "—"
          )}
        </div>
      </div>
      <div className="text-right text-xs text-muted-foreground">
        <div>{formatTime(event.recordedAt)}</div>
        <div className="mt-0.5 flex items-center justify-end gap-1 text-[10px] opacity-60">
          <EyeIcon className="size-3" />
          {labels.viewDetails}
        </div>
      </div>
    </button>
  );
});