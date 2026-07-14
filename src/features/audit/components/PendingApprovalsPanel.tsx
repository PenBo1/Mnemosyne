// PendingApprovalsPanel —— 安全拦截实时面板。
//
// 展示当前 pending 的 approval tokens,用户可批准/拒绝。
// 每个事件经 SecurityKernel 决策为 RequireApproval 且无 token 时,
// 会通过 security://event 推送 ApprovalRequested 事件,本面板展示并允许用户操作。
//
// 数据来源:
// - listPendingApprovals() —— 从 ApprovalStore 拉取当前 pending
// - useAuditStream() —— 监听 ApprovalRequested 事件自动刷新
// - approval_grant / approval_reject —— 执行操作

import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  ScrollArea,
} from "@/components/ui/scroll-area";
import {
  ShieldCheckIcon,
  ShieldXIcon,
  RefreshCwIcon,
  ClockIcon,
} from "lucide-react";
import { toast } from "sonner";
import { useI18n } from "@/locales/i18n";
import {
  SettingsSection,
  SettingsRow,
} from "@/features/settings/components/settings-section";
import {
  listPendingApprovals,
  getApprovalStats,
  grantApproval,
  rejectApproval,
  cleanupExpiredApprovals,
} from "../services";
import { useAuditStream } from "../hooks/useAuditStream";
import type { ApprovalTokenDto, ApprovalStatsDto } from "../types";

export function PendingApprovalsPanel() {
  const { t } = useI18n();
  const [tokens, setTokens] = useState<ApprovalTokenDto[]>([]);
  const [stats, setStats] = useState<ApprovalStatsDto | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);

  // 监听实时事件 —— 收到 ApprovalRequested 时自动刷新 pending 列表
  const { events: liveEvents } = useAuditStream(true);

  const loadData = useCallback(async () => {
    setRefreshing(true);
    try {
      const [pending, s] = await Promise.all([
        listPendingApprovals(),
        getApprovalStats(),
      ]);
      setTokens(pending);
      setStats(s);
    } catch (e) {
      toast.error(`${t.audit.loadFailed}: ${String(e)}`);
    } finally {
      setRefreshing(false);
      setLoading(false);
    }
  }, [t.audit.loadFailed]);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  // 收到 ApprovalRequested 事件触发刷新
  useEffect(() => {
    const hasApprovalRequest = liveEvents.some(
      (e) => e.eventType === "approval_requested",
    );
    if (hasApprovalRequest) {
      void loadData();
    }
  }, [liveEvents, loadData]);

  const handleGrant = async (token: ApprovalTokenDto) => {
    try {
      await grantApproval(token.id, "user");
      toast.success(t.audit.approvalGranted);
      await loadData();
    } catch (e) {
      toast.error(`${t.audit.approvalGrantFailed}: ${String(e)}`);
    }
  };

  const handleReject = async (token: ApprovalTokenDto) => {
    try {
      await rejectApproval(token.id, "user", t.audit.rejectedByUser);
      toast.success(t.audit.approvalRejected);
      await loadData();
    } catch (e) {
      toast.error(`${t.audit.approvalRejectFailed}: ${String(e)}`);
    }
  };

  const handleCleanup = async () => {
    try {
      const n = await cleanupExpiredApprovals();
      toast.success(t.audit.cleanupDone.replace("{n}", String(n)));
      await loadData();
    } catch (e) {
      toast.error(`${t.audit.cleanupFailed}: ${String(e)}`);
    }
  };

  if (loading) {
    return null;
  }

  return (
    <SettingsSection
      title={t.audit.pendingApprovalsTitle}
      description={t.audit.pendingApprovalsHint}
    >
      {stats && (
        <SettingsRow
          label={t.audit.approvalStats}
          description={t.audit.approvalStatsHint}
        >
          <div className="flex gap-2">
            <Badge variant="outline" className="text-amber-500">
              {t.audit.pending}: {stats.pending}
            </Badge>
            <Badge variant="outline" className="text-emerald-500">
              {t.audit.approved}: {stats.approved}
            </Badge>
            <Badge variant="outline" className="text-rose-500">
              {t.audit.rejected}: {stats.rejected}
            </Badge>
            <Badge variant="outline" className="text-muted-foreground">
              {t.audit.expired}: {stats.expired}
            </Badge>
          </div>
        </SettingsRow>
      )}

      <SettingsRow
        label={t.audit.actions}
        description={t.audit.actionsHint}
      >
        <div className="flex gap-2">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => void loadData()}
            disabled={refreshing}
          >
            <RefreshCwIcon className={refreshing ? "size-4 animate-spin" : "size-4"} />
            {t.audit.refresh}
          </Button>
          <Button variant="outline" size="sm" onClick={() => void handleCleanup()}>
            {t.audit.cleanupExpired}
          </Button>
        </div>
      </SettingsRow>

      <div className="px-4 pb-4">
        {tokens.length === 0 ? (
          <div className="flex h-24 items-center justify-center text-sm text-muted-foreground">
            {t.audit.noPendingApprovals}
          </div>
        ) : (
          <ScrollArea className="h-64 rounded-md border border-border">
            <div className="space-y-1 p-2">
              {tokens.map((token) => (
                <ApprovalTokenCard
                  key={token.id}
                  token={token}
                  onGrant={() => void handleGrant(token)}
                  onReject={() => void handleReject(token)}
                  labels={{
                    riskLevel: t.audit.riskLevel,
                    workspace: t.audit.workspace,
                    remaining: t.audit.remaining,
                    approve: t.audit.approve,
                    reject: t.audit.reject,
                  }}
                />
              ))}
            </div>
          </ScrollArea>
        )}
      </div>
    </SettingsSection>
  );
}

interface ApprovalTokenCardProps {
  token: ApprovalTokenDto;
  onGrant: () => void;
  onReject: () => void;
  labels: {
    riskLevel: string;
    workspace: string;
    remaining: string;
    approve: string;
    reject: string;
  };
}

function ApprovalTokenCard({ token, onGrant, onReject, labels }: ApprovalTokenCardProps) {
  const riskColor =
    token.riskLevel === "critical"
      ? "text-rose-600"
      : token.riskLevel === "high"
        ? "text-rose-500"
        : token.riskLevel === "medium"
          ? "text-amber-500"
          : "text-emerald-500";

  const isExpiringSoon = token.remainingSeconds < 10;

  return (
    <div className="rounded-md border border-border p-2">
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2 text-xs">
          <Badge variant="outline" className={riskColor}>
            {labels.riskLevel}: {token.riskLevel}
          </Badge>
          <span className="font-mono text-muted-foreground">
            {token.id.slice(0, 8)}…
          </span>
        </div>
        <div
          className={`flex items-center gap-1 text-xs ${
            isExpiringSoon ? "text-rose-500" : "text-muted-foreground"
          }`}
        >
          <ClockIcon className="size-3" />
          {labels.remaining}: {token.remainingSeconds}s
        </div>
      </div>
      <div className="mt-1 truncate text-xs text-muted-foreground">
        {labels.workspace}: {token.workspace.slice(0, 8)}…
      </div>
      <div className="mt-2 flex gap-1">
        <Button size="sm" variant="default" className="flex-1" onClick={onGrant}>
          <ShieldCheckIcon className="size-3" />
          {labels.approve}
        </Button>
        <Button
          size="sm"
          variant="outline"
          className="flex-1 text-rose-500"
          onClick={onReject}
        >
          <ShieldXIcon className="size-3" />
          {labels.reject}
        </Button>
      </div>
    </div>
  );
}
