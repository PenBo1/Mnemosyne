/**
 * ═══════════════════════════════════════════════════════════════════════════
 * ApprovalDialog - 安全审批对话框组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useEffect, useState, useCallback } from "react";
import { AlertTriangle, Check, X, Clock, FileText, Hash } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { useI18n } from "@/locales/i18n";
import type {
  ApprovalRequest,
  RiskLevel,
} from "@/services/ipc/security";
import {
  riskLevelToLabel,
  formatOperationDescription,
  formatOperationDetails,
} from "@/features/agent/services/security/policy-check";
import { approvalConsume, approvalReject } from "@/services/ipc/security";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface ApprovalDialogProps {
  request: ApprovalRequest | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onResolved: (result: { approved: boolean; tokenId: string }) => void;
}

// ── 常量配置 ────────────────────────────────────────────────────────────────

const RISK_BADGE_VARIANTS: Record<RiskLevel, "success" | "warning" | "destructive" | "outline"> = {
  low: "success",
  medium: "warning",
  high: "destructive",
  critical: "destructive",
};

// ── 模态对话框版本 ──────────────────────────────────────────────────────────

/**
 * 安全审批对话框组件，用于审批高风险操作
 */
export function ApprovalDialog({
  request,
  open,
  onOpenChange,
  onResolved,
}: ApprovalDialogProps) {
  const { t } = useI18n();
  const [remainingSeconds, setRemainingSeconds] = useState(30);
  const [submitting, setSubmitting] = useState(false);

  const token = request?.token;
  const operation = request?.operation;

  // ── 倒计时逻辑 ────────────────────────────────────────────────────────────

  useEffect(() => {
    if (!token || !open) {
      setRemainingSeconds(30);
      return;
    }

    const expireTime = new Date(token.expire).getTime();
    const now = Date.now();
    const initialSeconds = Math.max(0, Math.floor((expireTime - now) / 1000));
    setRemainingSeconds(initialSeconds);

    const interval = setInterval(() => {
      const currentNow = Date.now();
      const seconds = Math.max(0, Math.floor((expireTime - currentNow) / 1000));
      setRemainingSeconds(seconds);

      if (seconds <= 0) {
        clearInterval(interval);
        // 内联 reject 逻辑避免 handleReject 依赖缺失
        void approvalReject(token.id.uuid, "Token expired")
          .then(() => onResolved({ approved: false, tokenId: token.id.uuid }))
          .catch((err) => {
            console.error("Auto-reject failed:", err);
            onResolved({ approved: false, tokenId: token.id.uuid });
          })
          .finally(() => onOpenChange(false));
      }
    }, 1000);

    return () => clearInterval(interval);
  }, [token, open, onResolved, onOpenChange]);

  // ── 事件处理 ──────────────────────────────────────────────────────────────

  /**
   * 处理批准操作
   */
  const handleApprove = useCallback(async () => {
    if (!token || submitting) return;

    setSubmitting(true);
    try {
      const result = await approvalConsume(token.id.uuid);
      onResolved({ approved: result.approved, tokenId: token.id.uuid });
      onOpenChange(false);
    } catch (err) {
      console.error("Approval failed:", err);
      onResolved({ approved: false, tokenId: token.id.uuid });
      onOpenChange(false);
    } finally {
      setSubmitting(false);
    }
  }, [token, submitting, onResolved, onOpenChange]);

  /**
   * 处理拒绝操作
   */
  const handleReject = useCallback(async (reason?: string) => {
    if (!token || submitting) return;

    setSubmitting(true);
    try {
      await approvalReject(token.id.uuid, reason);
      onResolved({ approved: false, tokenId: token.id.uuid });
      onOpenChange(false);
    } catch (err) {
      console.error("Rejection failed:", err);
      onResolved({ approved: false, tokenId: token.id.uuid });
      onOpenChange(false);
    } finally {
      setSubmitting(false);
    }
  }, [token, submitting, onResolved, onOpenChange]);

  // ── 渲染 ──────────────────────────────────────────────────────────────────

  if (!request || !token || !operation) return null;

  const riskLevel = token.riskLevel;
  const riskLabel = riskLevelToLabel(riskLevel);
  const badgeVariant = RISK_BADGE_VARIANTS[riskLevel];
  const operationDesc = formatOperationDescription(operation);
  const operationDetails = formatOperationDetails(operation);

  const isExpired = remainingSeconds <= 0;
  const isCritical = riskLevel === "critical";

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md" showCloseButton={!submitting}>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <AlertTriangle className="size-4 text-yellow-600" />
            {t.security.approval.title}
          </DialogTitle>
          <DialogDescription>
            {isCritical
              ? t.security.approval.criticalWarning
              : t.security.approval.description}
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-3">
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium">{operationDesc}</span>
            <Badge variant={badgeVariant}>{riskLabel}</Badge>
          </div>

          <div className="rounded bg-[var(--bg-overlay-l1)] p-3 flex flex-col gap-2">
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <Clock className="size-3" />
              <span>
                {isExpired ? t.security.approval.expired : t.security.approval.remaining.replace("{seconds}", String(remainingSeconds))}
              </span>
            </div>

            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <Hash className="size-3" />
              <span className="font-mono truncate">{token.actionHash.slice(0, 16)}...</span>
            </div>

            <Separator />
            <div>
              <div className="flex items-center gap-2 text-xs text-muted-foreground mb-1">
                <FileText className="size-3" />
                <span>{t.security.approval.operationDetails}</span>
              </div>
              <pre className="text-xs whitespace-pre-wrap break-words overflow-auto max-h-32">
                {operationDetails}
              </pre>
            </div>
          </div>

          {request.reason && (
            <div className="text-xs text-muted-foreground">
              <span className="font-medium">{t.security.approval.reason}: </span>
              {request.reason}
            </div>
          )}
        </div>

        <DialogFooter>
          {isCritical ? (
            <Button
              variant="outline"
              disabled={submitting}
              onClick={() => onOpenChange(false)}
            >
              {t.common.close}
            </Button>
          ) : (
            <>
              <Button
                variant="outline"
                disabled={submitting || isExpired}
                onClick={() => handleReject("User rejected")}
              >
                <X className="size-3.5" />
                {t.security.approval.reject}
              </Button>
              <Button
                variant="default"
                disabled={submitting || isExpired}
                onClick={handleApprove}
              >
                <Check className="size-3.5" />
                {t.security.approval.approve}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ── 内联卡片版本 ────────────────────────────────────────────────────────────

/**
 * 内联审批卡片组件，用于在聊天界面中展示审批请求
 */
export function ApprovalDialogInline({
  request,
  submitting,
  onRespond,
}: {
  request: ApprovalRequest | null;
  submitting: boolean;
  onRespond: (tokenId: string, approved: boolean) => void;
}) {
  const { t } = useI18n();
  const [remainingSeconds, setRemainingSeconds] = useState(30);

  const token = request?.token;
  const operation = request?.operation;

  // ── 倒计时逻辑 ────────────────────────────────────────────────────────────

  useEffect(() => {
    if (!token) {
      setRemainingSeconds(30);
      return;
    }

    const expireTime = new Date(token.expire).getTime();
    const now = Date.now();
    const initialSeconds = Math.max(0, Math.floor((expireTime - now) / 1000));
    setRemainingSeconds(initialSeconds);

    const interval = setInterval(() => {
      const currentNow = Date.now();
      const seconds = Math.max(0, Math.floor((expireTime - currentNow) / 1000));
      setRemainingSeconds(seconds);

      if (seconds <= 0) {
        clearInterval(interval);
        if (!submitting) {
          onRespond(token.id.uuid, false);
        }
      }
    }, 1000);

    return () => clearInterval(interval);
  }, [token, submitting, onRespond]);

  // ── 渲染 ──────────────────────────────────────────────────────────────────

  if (!request || !token || !operation) return null;

  const riskLevel = token.riskLevel;
  const riskLabel = riskLevelToLabel(riskLevel);
  const badgeVariant = RISK_BADGE_VARIANTS[riskLevel];
  const operationDesc = formatOperationDescription(operation);
  const operationDetails = formatOperationDetails(operation);

  const isExpired = remainingSeconds <= 0;
  const isCritical = riskLevel === "critical";

  return (
    <div className="mx-auto max-w-3xl border border-[var(--color-warning-border)] bg-[var(--color-warning-bg)] rounded-lg p-4">
      <div className="flex items-start gap-3">
        <AlertTriangle className="mt-0.5 size-5 shrink-0 text-[var(--color-warning)]" />
        <div className="min-w-0 flex-1">
          <div className="mb-2 flex items-center gap-2">
            <span className="font-medium text-sm">{operationDesc}</span>
            <Badge variant={badgeVariant}>{riskLabel}</Badge>
            <div className="flex items-center gap-1 text-xs text-muted-foreground">
              <Clock className="size-3" />
              <span>{isExpired ? t.security.approval.expired : `${remainingSeconds}s`}</span>
            </div>
          </div>

          <div className="mb-3 rounded bg-[var(--bg-overlay-l1)] p-2 flex flex-col gap-1">
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <Hash className="size-3" />
              <span className="font-mono truncate">{token.actionHash.slice(0, 12)}...</span>
            </div>
            <pre className="max-h-24 overflow-auto text-xs whitespace-pre-wrap break-words">
              {operationDetails}
            </pre>
          </div>

          {request.reason && (
            <div className="mb-3 text-xs text-muted-foreground">
              <span className="font-medium">{t.security.approval.reason}: </span>
              {request.reason}
            </div>
          )}

          <div className="flex gap-2">
            {isCritical ? (
              <span className="text-xs text-destructive">{t.security.approval.criticalWarning}</span>
            ) : (
              <>
                <Button
                  size="sm"
                  variant="default"
                  disabled={submitting || isExpired}
                  onClick={() => onRespond(token.id.uuid, true)}
                >
                  <Check className="size-3.5" data-icon="inline-start" />
                  {t.security.approval.approve}
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  disabled={submitting || isExpired}
                  onClick={() => onRespond(token.id.uuid, false)}
                >
                  <X className="size-3.5" data-icon="inline-start" />
                  {t.security.approval.reject}
                </Button>
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}