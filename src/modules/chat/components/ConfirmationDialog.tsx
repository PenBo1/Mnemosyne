import { useEffect, useMemo, useState } from "react";
import { ShieldAlertIcon, ShieldCheckIcon, ShieldIcon } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Textarea } from "@/components/ui/textarea";
import { useI18n } from "@/shared/i18n";
import { cn } from "@/shared/utils";
import type { PendingConfirmation } from "@/shared/types";

interface ConfirmationDialogProps {
  pending: PendingConfirmation | null;
  submitting: boolean;
  onRespond: (params: {
    approved: boolean;
    autoApproveSimilar: boolean;
    modifiedArgs?: string;
  }) => void;
}

type RiskVariant = "safe" | "moderate" | "high";

function riskVariant(level: string): RiskVariant {
  const normalized = level.toLowerCase();
  if (normalized === "high") return "high";
  if (normalized === "modate" || normalized === "moderate") return "moderate";
  return "safe";
}

/**
 * SafetyGate 确认对话框：高风险工具调用前请求用户授权。
 *
 * - 高/中风险：弹出对话框
 * - 用户可选：批准 / 批准并自动放行同类 / 拒绝 / 提交修改后的参数
 * - 修改参数模式：本地校验 JSON 合法性，非法时禁用提交按钮
 */
export function ConfirmationDialog({
  pending,
  submitting,
  onRespond,
}: ConfirmationDialogProps) {
  const { t } = useI18n();
  const [editMode, setEditMode] = useState(false);
  const [editedArgs, setEditedArgs] = useState("");

  // 每次新的确认请求到来时，重置编辑状态
  useEffect(() => {
    if (pending) {
      setEditMode(false);
      setEditedArgs(pending.details || "");
    }
  }, [pending?.toolCallId, pending?.details]); // eslint-disable-line react-hooks/exhaustive-deps

  const open = pending !== null;
  const variant = pending ? riskVariant(pending.riskLevel) : "safe";

  const riskIcon = useMemo(() => {
    if (variant === "high") return <ShieldAlertIcon className="size-4" />;
    if (variant === "moderate") return <ShieldIcon className="size-4" />;
    return <ShieldCheckIcon className="size-4" />;
  }, [variant]);

  const riskBadgeClass = cn(
    variant === "high" && "bg-[var(--status-error-surface-l1)] text-[var(--status-error-default)]",
    variant === "moderate" && "bg-[var(--status-warning-surface-l1)] text-[var(--status-warning-default)]",
    variant === "safe" && "bg-[var(--status-success-surface-l1)] text-[var(--status-success-default)]"
  );

  const riskLabelKey = useMemo(() => {
    if (variant === "high") return "riskHigh";
    if (variant === "moderate") return "riskModerate";
    return "riskSafe";
  }, [variant]);

  // 校验编辑后的 JSON 合法性（仅在 editMode 启用按钮）
  const editedJsonValid = useMemo(() => {
    if (!editMode) return true;
    const trimmed = editedArgs.trim();
    if (trimmed.length === 0) return false;
    try {
      JSON.parse(trimmed);
      return true;
    } catch {
      return false;
    }
  }, [editMode, editedArgs]);

  if (!pending) return null;

  const handleApprove = () => onRespond({ approved: true, autoApproveSimilar: false });
  const handleApproveAuto = () => onRespond({ approved: true, autoApproveSimilar: true });
  const handleReject = () => onRespond({ approved: false, autoApproveSimilar: false });
  const handleSubmitModified = () => {
    if (!editedJsonValid) return;
    onRespond({
      approved: true,
      autoApproveSimilar: false,
      modifiedArgs: editedArgs.trim(),
    });
  };

  return (
    <Dialog open={open}>
      <DialogContent showCloseButton={false} className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            {riskIcon}
            <span>{t.agentChat.confirmationTitle}</span>
            <Badge variant="outline" className={cn("ml-auto", riskBadgeClass)}>
              {t.agentChat[riskLabelKey]}
            </Badge>
          </DialogTitle>
          <DialogDescription>{t.agentChat.confirmationDesc}</DialogDescription>
        </DialogHeader>

        <div className="grid gap-2 text-xs">
          <div className="flex gap-2">
            <span className="w-16 shrink-0 text-muted-foreground">{t.agentChat.toolLabel}</span>
            <code className="font-mono text-foreground">{pending.tool}</code>
          </div>
          {pending.description && (
            <div className="flex gap-2">
              <span className="w-16 shrink-0 text-muted-foreground">{t.agentChat.riskLabel}</span>
              <span className="text-foreground">{pending.description}</span>
            </div>
          )}
          <div className="flex gap-2">
            <span className="w-16 shrink-0 text-muted-foreground">{t.agentChat.argsLabel}</span>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-5 px-2 text-[11px]"
              onClick={() => setEditMode((v) => !v)}
              disabled={submitting}
            >
              {editMode ? t.agentChat.reasoningCollapse : t.agentChat.reasoningExpand}
            </Button>
          </div>
          {!editMode ? (
            <pre className="max-h-48 overflow-auto rounded-[var(--radius-4)] bg-[var(--bg-overlay-l2)] p-2 font-mono text-[11px] text-foreground whitespace-pre-wrap break-all">
              {pending.details || "{}"}
            </pre>
          ) : (
            <div className="grid gap-1">
              <Textarea
                value={editedArgs}
                onChange={(e) => setEditedArgs(e.target.value)}
                placeholder={t.agentChat.argsEditPlaceholder}
                className="max-h-48 min-h-24 font-mono text-[11px]"
                disabled={submitting}
              />
              {!editedJsonValid && (
                <span className="text-[11px] text-[var(--status-error-default)]">
                  {t.agentChat.argsInvalidJson}
                </span>
              )}
            </div>
          )}
        </div>

        <DialogFooter className="gap-2">
          <Button
            variant="outline"
            onClick={handleReject}
            disabled={submitting}
          >
            {t.agentChat.reject}
          </Button>
          {editMode ? (
            <Button
              onClick={handleSubmitModified}
              disabled={submitting || !editedJsonValid}
            >
              {t.agentChat.submitModified}
            </Button>
          ) : (
            <>
              <Button
                variant="secondary"
                onClick={handleApprove}
                disabled={submitting}
              >
                {t.agentChat.approve}
              </Button>
              <Button
                onClick={handleApproveAuto}
                disabled={submitting}
              >
                {t.agentChat.approveAuto}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
