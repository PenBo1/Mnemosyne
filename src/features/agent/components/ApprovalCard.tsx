/**
 * ═══════════════════════════════════════════════════════════════════════════
 * ApprovalCard - 工具调用审批卡片组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useMemo, memo } from "react";
import { AlertTriangle, Check, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { useI18n } from "@/locales/i18n";
import type { PendingConfirmation } from "@/types";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface ApprovalCardProps {
  confirmation: PendingConfirmation | null;
  submitting: boolean;
  onRespond: (approvalId: string, approved: boolean) => void;
}

// ── 组件实现 ────────────────────────────────────────────────────────────────

/**
 * 审批卡片组件，用于展示待审批的工具调用
 */
export const ApprovalCard = memo(function ApprovalCard({
  confirmation,
  submitting,
  onRespond,
}: ApprovalCardProps) {
  const { t } = useI18n();
  const argsStr = useMemo(
    () => (confirmation ? JSON.stringify(confirmation.args, null, 2) : ""),
    [confirmation]
  );

  if (!confirmation) return null;

  return (
    <Card className="mx-auto max-w-3xl border-[var(--color-warning-border)] bg-[var(--color-warning-bg)] p-4">
      <div className="flex items-start gap-3">
        <AlertTriangle className="mt-0.5 size-5 shrink-0 text-[var(--color-warning)]" />
        <div className="min-w-0 flex-1">
          <div className="mb-2">
            <span className="font-medium">
              {t.agentChat.approve || "Approve tool call"}:{" "}
              <code className="rounded bg-muted px-1.5 py-0.5 text-sm">
                {confirmation.toolName}
              </code>
            </span>
          </div>
          <pre className="max-h-40 overflow-auto rounded bg-[var(--bg-overlay-l1)] p-2 text-xs whitespace-pre-wrap break-words">
            {argsStr}
          </pre>
          <div className="mt-3 flex gap-2">
            <Button
              size="sm"
              variant="default"
              disabled={submitting}
              onClick={() => onRespond(confirmation.toolCallId, true)}
            >
              <Check className="size-3.5" data-icon="inline-start" />
              {t.agentChat.approve || "Approve"}
            </Button>
            <Button
              size="sm"
              variant="outline"
              disabled={submitting}
              onClick={() => onRespond(confirmation.toolCallId, false)}
            >
              <X className="size-3.5" data-icon="inline-start" />
              {t.agentChat.reject || "Reject"}
            </Button>
          </div>
        </div>
      </div>
    </Card>
  );
});