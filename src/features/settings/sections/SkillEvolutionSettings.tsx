// 技能进化设置页 —— 展示 skill_usage_stats 和 skill_candidate_proposals。
//
// 两个区块:
// 1. 技能使用统计(按 used_count 降序,显示成熟度)
// 2. 技能候选(可按 status 过滤,可批准/拒绝/删除)

import { useCallback, useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import {
  RefreshCwIcon,
  CheckIcon,
  XIcon,
  Trash2Icon,
  ThumbsUpIcon,
  ThumbsDownIcon,
  SparklesIcon,
} from "lucide-react";
import { toast } from "sonner";
import { useI18n } from "@/locales/i18n";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
  PageActions,
} from "@/components/shared/page-layout";
import { SettingsSection, SettingsRow } from "@/features/settings/components/settings-section";
import { LoadingState } from "@/components/shared/state";
import {
  listSkillCandidates,
  listSkillUsageStats,
  approveSkillCandidate,
  rejectSkillCandidate,
  deleteSkillCandidate,
  adjustSkillFeedback,
} from "@/features/skill/services/skill-evolution";
import {
  calculateMaturity,
  candidateStatusColor,
  candidateStatusLabel,
  maturityColor,
  maturityLabel,
  successRate,
  type SkillCandidate,
  type SkillCandidateStatus,
  type SkillUsageStats,
} from "@/features/skill/types/skill-evolution";

function formatRelativeTime(iso: string): string {
  try {
    const d = new Date(iso);
    const now = new Date();
    const diffMs = now.getTime() - d.getTime();
    const days = Math.floor(diffMs / (1000 * 60 * 60 * 24));
    if (days > 30) return `${Math.floor(days / 30)} 月前`;
    if (days > 0) return `${days} 天前`;
    const hours = Math.floor(diffMs / (1000 * 60 * 60));
    if (hours > 0) return `${hours} 小时前`;
    const mins = Math.floor(diffMs / (1000 * 60));
    if (mins > 0) return `${mins} 分钟前`;
    return "刚刚";
  } catch {
    return iso;
  }
}

export function SkillEvolutionSettings() {
  const { t } = useI18n();
  const [usage, setUsage] = useState<SkillUsageStats[]>([]);
  const [candidates, setCandidates] = useState<SkillCandidate[]>([]);
  const [loading, setLoading] = useState(true);
  const [statusFilter, setStatusFilter] = useState<SkillCandidateStatus | "all">("pending");
  const [actionLoading, setActionLoading] = useState<string | null>(null);

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      const [u, c] = await Promise.all([
        listSkillUsageStats(),
        listSkillCandidates(),
      ]);
      setUsage(u);
      setCandidates(c);
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to load skill evolution data";
      toast.error(msg);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  const filteredCandidates = useMemo(() => {
    if (statusFilter === "all") return candidates;
    return candidates.filter((c) => c.status === statusFilter);
  }, [candidates, statusFilter]);

  const handleApprove = useCallback(async (candidateId: string) => {
    if (!window.confirm(t.settings.skillEvolution.approveConfirm)) return;
    try {
      setActionLoading(candidateId);
      const ok = await approveSkillCandidate(candidateId);
      if (ok) {
        toast.success(t.settings.skillEvolution.approvedToast);
        await loadData();
      } else {
        toast.error("Approve failed");
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Approve failed";
      toast.error(msg);
    } finally {
      setActionLoading(null);
    }
  }, [loadData, t.settings.skillEvolution.approveConfirm, t.settings.skillEvolution.approvedToast]);

  const handleReject = useCallback(async (candidateId: string) => {
    if (!window.confirm(t.settings.skillEvolution.rejectConfirm)) return;
    try {
      setActionLoading(candidateId);
      const ok = await rejectSkillCandidate(candidateId);
      if (ok) {
        toast.success(t.settings.skillEvolution.rejectedToast);
        await loadData();
      } else {
        toast.error("Reject failed");
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Reject failed";
      toast.error(msg);
    } finally {
      setActionLoading(null);
    }
  }, [loadData, t.settings.skillEvolution.rejectConfirm, t.settings.skillEvolution.rejectedToast]);

  const handleDelete = useCallback(async (candidateId: string) => {
    if (!window.confirm(t.settings.skillEvolution.deleteConfirm)) return;
    try {
      setActionLoading(candidateId);
      const ok = await deleteSkillCandidate(candidateId);
      if (ok) {
        toast.success(t.settings.skillEvolution.deletedToast);
        await loadData();
      } else {
        toast.error("Delete failed");
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Delete failed";
      toast.error(msg);
    } finally {
      setActionLoading(null);
    }
  }, [loadData, t.settings.skillEvolution.deleteConfirm, t.settings.skillEvolution.deletedToast]);

  const handleFeedback = useCallback(async (skillName: string, delta: number) => {
    try {
      setActionLoading(`feedback-${skillName}`);
      await adjustSkillFeedback(skillName, delta);
      await loadData();
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Feedback failed";
      toast.error(msg);
    } finally {
      setActionLoading(null);
    }
  }, [loadData]);

  if (loading) {
    return (
      <PageContainer scrollable={false}>
        <LoadingState label={t.common.loading} />
      </PageContainer>
    );
  }

  return (
    <PageContainer scrollable={false}>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <SparklesIcon className="size-4 text-amber-500" />
            {t.settings.skillEvolution.label}
          </PageTitle>
          <PageDescription>{t.settings.skillEvolution.description}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button
            variant="outline"
            size="sm"
            className="h-8 gap-1.5 px-2 text-xs"
            onClick={loadData}
          >
            <RefreshCwIcon className="size-3.5" />
            {t.common.reload}
          </Button>
        </PageActions>
      </PageHeader>

      {/* 1. 技能使用统计 */}
      <SettingsSection
        title={t.settings.skillEvolution.usageStats}
        description={t.settings.skillEvolution.usageStatsHint}
      >
        {usage.length === 0 ? (
          <div className="px-4 py-8 text-center text-sm text-muted-foreground">
            {t.settings.skillEvolution.noUsage}
          </div>
        ) : (
          <ScrollArea className="h-[280px]">
            <div className="divide-y">
              {usage.map((s) => {
                const m = calculateMaturity(s);
                const r = successRate(s);
                return (
                  <div
                    key={s.skillName}
                    className="flex items-center justify-between gap-3 px-4 py-2.5"
                  >
                    <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                      <span className="truncate text-sm font-medium">{s.skillName}</span>
                      <span className="text-xs text-muted-foreground">
                        {t.settings.skillEvolution.usedCount}: {s.usedCount}
                        {" · "}
                        {t.settings.skillEvolution.successRate}: {(r * 100).toFixed(0)}%
                        {" · "}
                        {t.settings.skillEvolution.lastUsed}: {formatRelativeTime(s.lastUsedAt)}
                      </span>
                    </div>
                    <div className="flex items-center gap-2">
                      <Badge variant="outline" className={maturityColor(m)}>
                        {maturityLabel(m)}
                      </Badge>
                      <Badge variant="outline">
                        {t.settings.skillEvolution.feedbackScore}: {s.userFeedbackScore.toFixed(1)}
                      </Badge>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="size-7 p-0"
                        disabled={actionLoading === `feedback-${s.skillName}`}
                        onClick={() => handleFeedback(s.skillName, 1.0)}
                        title={t.settings.skillEvolution.upvote}
                      >
                        <ThumbsUpIcon className="size-3.5" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="size-7 p-0"
                        disabled={actionLoading === `feedback-${s.skillName}`}
                        onClick={() => handleFeedback(s.skillName, -1.0)}
                        title={t.settings.skillEvolution.downvote}
                      >
                        <ThumbsDownIcon className="size-3.5" />
                      </Button>
                    </div>
                  </div>
                );
              })}
            </div>
          </ScrollArea>
        )}
      </SettingsSection>

      {/* 2. 技能候选 */}
      <SettingsSection
        title={t.settings.skillEvolution.candidates}
        description={t.settings.skillEvolution.candidatesHint}
      >
        <SettingsRow
          label={t.settings.skillEvolution.pendingOnly}
          description={t.settings.skillEvolution.candidatesHint}
        >
          <Select
            value={statusFilter}
            onValueChange={(v) => setStatusFilter(v as SkillCandidateStatus | "all")}
          >
            <SelectTrigger className="w-40">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="pending">{t.settings.skillEvolution.statusPending}</SelectItem>
              <SelectItem value="approved">{t.settings.skillEvolution.statusApproved}</SelectItem>
              <SelectItem value="rejected">{t.settings.skillEvolution.statusRejected}</SelectItem>
              <SelectItem value="superseded">{t.settings.skillEvolution.statusSuperseded}</SelectItem>
              <SelectItem value="all">{t.settings.skillEvolution.all}</SelectItem>
            </SelectContent>
          </Select>
        </SettingsRow>
        {filteredCandidates.length === 0 ? (
          <div className="px-4 py-8 text-center text-sm text-muted-foreground">
            {t.settings.skillEvolution.noCandidates}
          </div>
        ) : (
          <ScrollArea className="h-[320px]">
            <div className="divide-y">
              {filteredCandidates.map((c) => (
                <div key={c.id} className="flex flex-col gap-2 px-4 py-3">
                  <div className="flex items-start justify-between gap-3">
                    <div className="flex min-w-0 flex-1 flex-col gap-1">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-medium">{c.candidateName}</span>
                        <Badge variant="outline" className={candidateStatusColor(c.status)}>
                          {candidateStatusLabel(c.status)}
                        </Badge>
                      </div>
                      <span className="text-xs text-muted-foreground">
                        {c.candidateDescription}
                      </span>
                      <span className="text-xs text-muted-foreground">
                        {t.settings.skillEvolution.sourceSession}: {c.sourceSessionId}
                        {" · "}
                        {t.settings.skillEvolution.createdAt}: {formatRelativeTime(c.createdAt)}
                      </span>
                    </div>
                    <div className="flex items-center gap-1">
                      {c.status === "pending" && (
                        <>
                          <Button
                            variant="ghost"
                            size="sm"
                            className="size-7 p-0 text-emerald-600"
                            disabled={actionLoading === c.id}
                            onClick={() => handleApprove(c.id)}
                            title={t.settings.skillEvolution.approve}
                          >
                            <CheckIcon className="size-3.5" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="sm"
                            className="size-7 p-0 text-amber-600"
                            disabled={actionLoading === c.id}
                            onClick={() => handleReject(c.id)}
                            title={t.settings.skillEvolution.reject}
                          >
                            <XIcon className="size-3.5" />
                          </Button>
                        </>
                      )}
                      <Button
                        variant="ghost"
                        size="sm"
                        className="size-7 p-0 text-red-500"
                        disabled={actionLoading === c.id}
                        onClick={() => handleDelete(c.id)}
                        title={t.settings.skillEvolution.delete}
                      >
                        <Trash2Icon className="size-3.5" />
                      </Button>
                    </div>
                  </div>
                  {c.candidateContent && (
                    <pre className="overflow-x-auto rounded bg-muted/50 p-2 text-xs">
                      {c.candidateContent.slice(0, 500)}
                      {c.candidateContent.length > 500 ? "…" : ""}
                    </pre>
                  )}
                  {c.reviewerNotes && (
                    <span className="text-xs italic text-muted-foreground">
                      {t.settings.skillEvolution.reviewerNotes}: {c.reviewerNotes}
                    </span>
                  )}
                </div>
              ))}
            </div>
          </ScrollArea>
        )}
      </SettingsSection>
    </PageContainer>
  );
}
