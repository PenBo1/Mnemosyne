/**
 * ═══════════════════════════════════════════════════════════════════════════
 * EventDetailDrawer - 审计事件详情抽屉组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { memo } from "react";
import { Badge } from "@/components/ui/badge";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useI18n } from "@/locales/i18n";
import { DENIED_EVENT_TYPES, SECURITY_EVENT_TYPES_SET } from "../types";
import type { AuditEventRow } from "../types";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface EventDetailDrawerProps {
  event: AuditEventRow | null;
  open: boolean;
  onClose: () => void;
}

interface DetailRowProps {
  label: string;
  value: string;
  mono?: boolean;
}

// ── 子组件 ──────────────────────────────────────────────────────────────────

/**
 * 详情行组件
 */
const DetailRow = memo(function DetailRow({ label, value, mono }: DetailRowProps) {
  return (
    <div className="flex flex-col gap-1">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div
        className={`truncate text-sm ${mono ? "font-mono" : ""}`}
        title={value}
      >
        {value}
      </div>
    </div>
  );
});

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 审计事件详情抽屉，从右侧滑入展示事件的完整 payload
 */
export function EventDetailDrawer({ event, open, onClose }: EventDetailDrawerProps) {
  const { t } = useI18n();

  if (!event) return null;

  const isDenied = DENIED_EVENT_TYPES.has(event.eventType) || event.isDenied;
  const isSecurity =
    SECURITY_EVENT_TYPES_SET.has(event.eventType) || event.isSecurityRelated;

  return (
    <Sheet open={open} onOpenChange={(v) => !v && onClose()}>
      <SheetContent className="flex w-full flex-col gap-4 sm:max-w-lg">
        <SheetHeader>
          <SheetTitle className="flex items-center gap-2">
            <span className="font-mono text-base">{event.eventType}</span>
            {event.operation && (
              <Badge variant="outline" className="font-mono text-xs">
                {event.operation}
              </Badge>
            )}
          </SheetTitle>
          <SheetDescription>{t.audit.eventDetailDescription}</SheetDescription>
        </SheetHeader>

        <div className="grid grid-cols-2 gap-2 px-1 text-sm">
          <DetailRow label={t.audit.eventId} value={event.id} mono />
          <DetailRow
            label={t.audit.recordedAt}
            value={new Date(event.recordedAt).toLocaleString()}
          />
          <DetailRow
            label={t.audit.workspaceId}
            value={event.workspaceId ?? "—"}
            mono
          />
          <div className="flex flex-col gap-1">
            <div className="text-xs text-muted-foreground">{t.audit.flags}</div>
            <div className="flex flex-wrap gap-1">
              {isDenied && (
                <Badge variant="destructive">{t.audit.denied}</Badge>
              )}
              {isSecurity && (
                <Badge variant="secondary" className="text-amber-500">
                  {t.audit.security}
                </Badge>
              )}
              {!isDenied && !isSecurity && (
                <Badge variant="outline">{t.audit.normal}</Badge>
              )}
            </div>
          </div>
        </div>

        <div className="flex-1 overflow-hidden">
          <div className="mb-2 text-xs font-medium text-muted-foreground">
            {t.audit.payload}
          </div>
          <ScrollArea className="h-full rounded-md border border-border bg-muted/30 p-3">
            <pre className="font-mono text-xs leading-relaxed">
              {JSON.stringify(event.payload, null, 2)}
            </pre>
          </ScrollArea>
        </div>
      </SheetContent>
    </Sheet>
  );
}