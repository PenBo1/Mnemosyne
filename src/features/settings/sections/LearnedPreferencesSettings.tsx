/**
 * ═══════════════════════════════════════════════════════════════════════════
 * LearnedPreferencesSettings - 学习偏好设置页面
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useCallback, useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Input } from "@/components/ui/input";
import {
  RefreshCwIcon,
  Trash2Icon,
  SparklesIcon,
  HeartIcon,
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
  listLearnedPreferences,
  listHighConfidencePreferences,
  deleteLearnedPreference,
  analyzeLearnedPreferences,
} from "@/features/settings/services/learned-preferences";
import {
  confidenceColor,
  confidenceLabel,
  type LearnedPreferenceRow,
} from "@/features/settings/types/learned-preferences";

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/**
 * 获取置信度徽章变体
 */
function confidenceBadgeVariant(confidence: number) {
  if (confidence >= 0.8) return "default" as const;
  if (confidence >= 0.5) return "secondary" as const;
  return "outline" as const;
}

/**
 * 格式化相对时间
 */
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

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 学习偏好设置页面，展示和管理学习到的用户偏好
 */
export function LearnedPreferencesSettings() {
  const { t } = useI18n();
  const [prefs, setPrefs] = useState<LearnedPreferenceRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [highConfidenceOnly, setHighConfidenceOnly] = useState(false);
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [analyzeSessionId, setAnalyzeSessionId] = useState("");
  const [analyzing, setAnalyzing] = useState(false);

  // ── 数据加载 ──────────────────────────────────────────────────────────────

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      const rows = highConfidenceOnly
        ? await listHighConfidencePreferences(0.7)
        : await listLearnedPreferences();
      setPrefs(rows);
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to load learned preferences";
      toast.error(msg);
    } finally {
      setLoading(false);
    }
  }, [highConfidenceOnly]);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  // ── 事件处理 ──────────────────────────────────────────────────────────────

  /**
   * 删除偏好
   */
  const handleDelete = useCallback(async (id: string) => {
    if (!window.confirm(t.settings.learnedPreferences.deleteConfirm)) return;
    try {
      setActionLoading(id);
      await deleteLearnedPreference(id);
      toast.success(t.settings.learnedPreferences.deletedToast);
      await loadData();
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Delete failed";
      toast.error(msg);
    } finally {
      setActionLoading(null);
    }
  }, [loadData, t.settings.learnedPreferences.deleteConfirm, t.settings.learnedPreferences.deletedToast]);

  /**
   * 分析偏好
   */
  const handleAnalyze = useCallback(async () => {
    const sid = analyzeSessionId.trim();
    if (!sid) {
      toast.error("Session ID is required");
      return;
    }
    try {
      setAnalyzing(true);
      toast.info(t.settings.learnedPreferences.analyzingToast);
      const count = await analyzeLearnedPreferences(sid);
      toast.success(
        t.settings.learnedPreferences.analyzedToast.replace("{count}", String(count)),
      );
      await loadData();
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Analyze failed";
      toast.error(msg);
    } finally {
      setAnalyzing(false);
    }
  }, [analyzeSessionId, loadData, t.settings.learnedPreferences.analyzingToast, t.settings.learnedPreferences.analyzedToast]);

  // ── 计算属性 ──────────────────────────────────────────────────────────────

  const grouped = useMemo(() => {
    const m = new Map<string, LearnedPreferenceRow[]>();
    for (const p of prefs) {
      const arr = m.get(p.preferenceKey) ?? [];
      arr.push(p);
      m.set(p.preferenceKey, arr);
    }
    return Array.from(m.entries()).sort((a, b) => b[1].length - a[1].length);
  }, [prefs]);

  // ── 加载中状态 ────────────────────────────────────────────────────────────

  if (loading) {
    return (
      <PageContainer>
        <LoadingState label={t.common.loading} />
      </PageContainer>
    );
  }

  // ── 渲染 ──────────────────────────────────────────────────────────────────

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <HeartIcon className="size-4 text-rose-500" />
            {t.settings.learnedPreferences.label}
          </PageTitle>
          <PageDescription>{t.settings.learnedPreferences.description}</PageDescription>
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

      {/* ── 偏好分析触发 ────────────────────────────────────────────────────── */}
      <SettingsSection
        title={t.settings.learnedPreferences.analyze}
        description={t.settings.learnedPreferences.analyzeHint}
      >
        <SettingsRow
          label={t.settings.learnedPreferences.analyze}
          description={t.settings.learnedPreferences.analyzeHint}
        >
          <div className="flex items-center gap-2">
            <Input
              value={analyzeSessionId}
              onChange={(e) => setAnalyzeSessionId(e.target.value)}
              placeholder="session-id"
              className="h-8 w-48 text-xs"
              disabled={analyzing}
            />
            <Button
              variant="default"
              size="sm"
              className="h-8 gap-1.5 px-2 text-xs"
              disabled={analyzing || !analyzeSessionId.trim()}
              onClick={handleAnalyze}
            >
              <SparklesIcon className="size-3.5" />
              {t.settings.learnedPreferences.analyze}
            </Button>
          </div>
        </SettingsRow>
      </SettingsSection>

      {/* ── 偏好列表 ────────────────────────────────────────────────────────── */}
      <SettingsSection
        title={t.settings.learnedPreferences.label}
        description={t.settings.learnedPreferences.hint}
      >
        <SettingsRow
          label={t.settings.learnedPreferences.highConfidenceOnly}
          description={t.settings.learnedPreferences.hint}
        >
          <Switch
            checked={highConfidenceOnly}
            onCheckedChange={setHighConfidenceOnly}
          />
        </SettingsRow>
        {prefs.length === 0 ? (
          <div className="px-4 py-8 text-center text-sm text-muted-foreground">
            {t.settings.learnedPreferences.noPreferences}
          </div>
        ) : (
          <ScrollArea className="h-[400px]">
            <div className="divide-y">
              {grouped.map(([key, items]) => (
                <div key={key} className="px-4 py-3">
                  <div className="mb-2 text-xs font-medium text-muted-foreground">
                    {key}
                    <span className="ml-2 text-muted-foreground/70">({items.length})</span>
                  </div>
                  <div className="flex flex-col gap-1.5">
                    {items.map((p) => (
                      <div
                        key={p.id}
                        className="flex items-center justify-between gap-2 rounded border border-border/50 bg-muted/30 px-2 py-1.5"
                      >
                        {/* ── 偏好信息 ────────────────────────────────────── */}
                        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                          <span className="truncate text-sm">{p.preferenceValue}</span>
                          <span className="text-xs text-muted-foreground">
                            {t.settings.learnedPreferences.occurrenceCount}: {p.occurrenceCount}
                            {" · "}
                            {t.settings.learnedPreferences.lastSeenAt}: {formatRelativeTime(p.lastSeenAt)}
                          </span>
                        </div>
                        {/* ── 置信度和操作 ──────────────────────────────── */}
                        <div className="flex items-center gap-2">
                          <Badge variant={confidenceBadgeVariant(p.confidence)} className={confidenceColor(p.confidence)}>
                            {confidenceLabel(p.confidence)}
                          </Badge>
                          <Badge variant="outline">
                            {Math.round(p.confidence * 100)}%
                          </Badge>
                          <Button
                            variant="ghost"
                            size="sm"
                            className="size-7 p-0 text-red-500"
                            disabled={actionLoading === p.id}
                            onClick={() => handleDelete(p.id)}
                            title={t.settings.learnedPreferences.delete}
                          >
                            <Trash2Icon className="size-3.5" />
                          </Button>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </ScrollArea>
        )}
      </SettingsSection>
    </PageContainer>
  );
}