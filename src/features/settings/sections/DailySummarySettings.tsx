/**
 * ═══════════════════════════════════════════════════════════════════════════
 * DailySummarySettings - 每日摘要任务设置页面
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import {
  RefreshCwIcon,
  CalendarClockIcon,
  PlayIcon,
  SquareIcon,
  ZapIcon,
  SaveIcon,
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
  getDailySummaryConfig,
  updateDailySummaryConfig,
  startDailySummary,
  stopDailySummary,
  triggerDailySummary,
  isDailySummaryRunning,
} from "@/features/settings/services/daily-summary";
import {
  DEFAULT_DAILY_SUMMARY_CONFIG,
  INTERVAL_PRESETS,
  formatInterval,
  formatDuration,
  type DailySummaryConfig,
  type DailySummaryReport,
} from "@/features/settings/types/daily-summary";

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 每日摘要任务设置页面，控制 DailySummaryTask 的启停、配置和手动触发
 */
export function DailySummarySettings() {
  const { t } = useI18n();
  const [config, setConfig] = useState<DailySummaryConfig>(DEFAULT_DAILY_SUMMARY_CONFIG);
  const [running, setRunning] = useState(false);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState(false);
  const [triggering, setTriggering] = useState(false);
  const [saving, setSaving] = useState(false);
  const [lastReport, setLastReport] = useState<DailySummaryReport | null>(null);

  /** 本地草稿（用户编辑中、未保存的配置） */
  const [draft, setDraft] = useState<DailySummaryConfig>(DEFAULT_DAILY_SUMMARY_CONFIG);

  // ── 数据加载 ──────────────────────────────────────────────────────────────

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      const [cfg, isRunning] = await Promise.all([
        getDailySummaryConfig(),
        isDailySummaryRunning(),
      ]);
      setConfig(cfg);
      setDraft(cfg);
      setRunning(isRunning);
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to load daily summary config";
      toast.error(msg);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  // ── 事件处理 ──────────────────────────────────────────────────────────────

  /**
   * 启动定时任务
   */
  const handleStart = useCallback(async () => {
    try {
      setActionLoading(true);
      await startDailySummary();
      setRunning(true);
      toast.success(t.settings.dailySummary.taskStarted);
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Start failed";
      toast.error(msg);
    } finally {
      setActionLoading(false);
    }
  }, [t.settings.dailySummary.taskStarted]);

  /**
   * 停止定时任务
   */
  const handleStop = useCallback(async () => {
    try {
      setActionLoading(true);
      await stopDailySummary();
      setRunning(false);
      toast.success(t.settings.dailySummary.taskStopped);
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Stop failed";
      toast.error(msg);
    } finally {
      setActionLoading(false);
    }
  }, [t.settings.dailySummary.taskStopped]);

  /**
   * 手动触发一次摘要生成
   */
  const handleTrigger = useCallback(async () => {
    try {
      setTriggering(true);
      const report = await triggerDailySummary();
      setLastReport(report);
      toast.success(t.settings.dailySummary.triggerSuccess);
    } catch (err) {
      const msg = err instanceof Error ? err.message : t.settings.dailySummary.triggerFailed;
      toast.error(msg);
    } finally {
      setTriggering(false);
    }
  }, [t.settings.dailySummary.triggerSuccess, t.settings.dailySummary.triggerFailed]);

  /**
   * 切换启用状态
   */
  const handleToggleEnabled = useCallback(async (enabled: boolean) => {
    try {
      setSaving(true);
      const newCfg = { ...draft, enabled };
      const saved = await updateDailySummaryConfig(newCfg);
      setConfig(saved);
      setDraft(saved);
      if (enabled) {
        // 启用后自动启动任务
        await startDailySummary();
        setRunning(true);
      } else {
        await stopDailySummary();
        setRunning(false);
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Update failed";
      toast.error(msg);
    } finally {
      setSaving(false);
    }
  }, [draft]);

  /**
   * 保存配置
   */
  const handleSaveConfig = useCallback(async () => {
    try {
      setSaving(true);
      const saved = await updateDailySummaryConfig(draft);
      setConfig(saved);
      setDraft(saved);
      toast.success(t.settings.dailySummary.configSaved);
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Save failed";
      toast.error(msg);
    } finally {
      setSaving(false);
    }
  }, [draft, t.settings.dailySummary.configSaved]);

  // ── 计算属性 ──────────────────────────────────────────────────────────────

  const intervalPresetValue = String(draft.intervalMs);
  const isPresetMatched = INTERVAL_PRESETS.some((p) => String(p.value) === intervalPresetValue);

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
            <CalendarClockIcon className="size-4 text-violet-500" />
            {t.settings.dailySummary.label}
          </PageTitle>
          <PageDescription>{t.settings.dailySummary.description}</PageDescription>
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

      {/* ── 任务控制 ────────────────────────────────────────────────────────── */}
      <SettingsSection
        title={t.settings.dailySummary.taskControl}
        description={t.settings.dailySummary.taskControlHint}
      >
        <SettingsRow
          label={t.settings.dailySummary.enabled}
          description={t.settings.dailySummary.enabledHint}
        >
          <div className="flex items-center gap-3">
            <Switch
              checked={draft.enabled}
              onCheckedChange={handleToggleEnabled}
              disabled={saving}
            />
            <Badge variant={running ? "default" : "outline"}>
              {running ? t.settings.dailySummary.running : t.settings.dailySummary.stopped}
            </Badge>
          </div>
        </SettingsRow>

        <SettingsRow
          label={t.settings.dailySummary.triggerOnce}
          description={t.settings.dailySummary.triggerOnceHint}
        >
          <div className="flex items-center gap-2">
            <Button
              variant="default"
              size="sm"
              className="h-8 gap-1.5 px-2 text-xs"
              disabled={triggering}
              onClick={handleTrigger}
            >
              <ZapIcon className="size-3.5" />
              {triggering ? t.settings.dailySummary.triggering : t.settings.dailySummary.triggerOnce}
            </Button>
            {running ? (
              <Button
                variant="outline"
                size="sm"
                className="h-8 gap-1.5 px-2 text-xs"
                disabled={actionLoading}
                onClick={handleStop}
              >
                <SquareIcon className="size-3.5" />
                {t.settings.dailySummary.stop}
              </Button>
            ) : (
              <Button
                variant="outline"
                size="sm"
                className="h-8 gap-1.5 px-2 text-xs"
                disabled={actionLoading || !draft.enabled}
                onClick={handleStart}
              >
                <PlayIcon className="size-3.5" />
                {t.settings.dailySummary.start}
              </Button>
            )}
          </div>
        </SettingsRow>
      </SettingsSection>

      {/* ── 配置 ────────────────────────────────────────────────────────────── */}
      <SettingsSection
        title={t.settings.dailySummary.config}
        description={t.settings.dailySummary.configHint}
      >
        <SettingsRow
          label={t.settings.dailySummary.interval}
          description={t.settings.dailySummary.intervalHint}
        >
          <div className="flex items-center gap-2">
            <Select
              value={isPresetMatched ? intervalPresetValue : "custom"}
              onValueChange={(v) => {
                if (v === "custom") return;
                setDraft({ ...draft, intervalMs: Number(v) });
              }}
            >
              <SelectTrigger className="h-8 w-40 text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {INTERVAL_PRESETS.map((p) => (
                  <SelectItem key={p.value} value={String(p.value)} className="text-xs">
                    {p.label}
                  </SelectItem>
                ))}
                <SelectItem value="custom" className="text-xs">
                  {t.settings.dailySummary.intervalCustom}
                </SelectItem>
              </SelectContent>
            </Select>
            {!isPresetMatched && (
              <Input
                type="number"
                min={60000}
                max={604800000}
                value={draft.intervalMs}
                onChange={(e) => setDraft({ ...draft, intervalMs: Number(e.target.value) })}
                className="h-8 w-32 text-xs"
              />
            )}
            <span className="text-xs text-muted-foreground">
              {formatInterval(draft.intervalMs)}
            </span>
          </div>
        </SettingsRow>

        <SettingsRow
          label={t.settings.dailySummary.lookbackDays}
          description={t.settings.dailySummary.lookbackDaysHint}
        >
          <Input
            type="number"
            min={1}
            max={30}
            value={draft.lookbackDays}
            onChange={(e) => setDraft({ ...draft, lookbackDays: Number(e.target.value) })}
            className="h-8 w-24 text-xs"
          />
        </SettingsRow>

        <SettingsRow
          label={t.settings.dailySummary.staleCutoffDays}
          description={t.settings.dailySummary.staleCutoffDaysHint}
        >
          <Input
            type="number"
            min={1}
            max={365}
            value={draft.staleCutoffDays}
            onChange={(e) => setDraft({ ...draft, staleCutoffDays: Number(e.target.value) })}
            className="h-8 w-24 text-xs"
          />
        </SettingsRow>

        <SettingsRow
          label={t.settings.dailySummary.saveConfig}
          description={t.settings.dailySummary.configHint}
        >
          <Button
            variant="default"
            size="sm"
            className="h-8 gap-1.5 px-2 text-xs"
            disabled={saving || JSON.stringify(draft) === JSON.stringify(config)}
            onClick={handleSaveConfig}
          >
            <SaveIcon className="size-3.5" />
            {t.settings.dailySummary.saveConfig}
          </Button>
        </SettingsRow>
      </SettingsSection>

      {/* ── 最近执行结果 ──────────────────────────────────────────────────── */}
      <SettingsSection
        title={t.settings.dailySummary.lastReport}
        description={t.settings.dailySummary.lastReportHint}
      >
        {lastReport ? (
          <ScrollArea className="max-h-96">
            <div className="flex flex-col gap-3 px-4 py-2">
              {/* ── 统计卡片 ────────────────────────────────────────────────── */}
              <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
                <div className="rounded border border-border/50 bg-muted/30 px-3 py-2">
                  <div className="text-xs text-muted-foreground">
                    {t.settings.dailySummary.reviewedShortTerm}
                  </div>
                  <div className="text-lg font-semibold">{lastReport.reviewedShortTerm}</div>
                </div>
                <div className="rounded border border-border/50 bg-muted/30 px-3 py-2">
                  <div className="text-xs text-muted-foreground">
                    {t.settings.dailySummary.decayedPreferences}
                  </div>
                  <div className="text-lg font-semibold">{lastReport.decayedPreferences}</div>
                </div>
                <div className="rounded border border-border/50 bg-muted/30 px-3 py-2">
                  <div className="text-xs text-muted-foreground">
                    {t.settings.dailySummary.updatedRoles}
                  </div>
                  <div className="text-lg font-semibold">{lastReport.updatedRoles.length}</div>
                </div>
                <div className="rounded border border-border/50 bg-muted/30 px-3 py-2">
                  <div className="text-xs text-muted-foreground">
                    {t.settings.dailySummary.duration}
                  </div>
                  <div className="text-lg font-semibold">
                    {formatDuration(lastReport.durationMs)}
                  </div>
                </div>
              </div>

              {/* ── 更新的角色 ──────────────────────────────────────────────── */}
              {lastReport.updatedRoles.length > 0 && (
                <div className="flex flex-col gap-1.5">
                  <div className="text-xs font-medium text-muted-foreground">
                    {t.settings.dailySummary.updatedRoles}
                  </div>
                  <div className="flex flex-wrap gap-1.5">
                    {lastReport.updatedRoles.map((role) => (
                      <Badge key={role} variant="secondary" className="text-xs">
                        {role}
                      </Badge>
                    ))}
                  </div>
                </div>
              )}

              {/* ── 错误信息 ────────────────────────────────────────────────── */}
              <div className="flex flex-col gap-1.5">
                <div className="text-xs font-medium text-muted-foreground">
                  {t.settings.dailySummary.errors}
                </div>
                {lastReport.errors.length === 0 ? (
                  <span className="text-xs text-emerald-600">
                    {t.settings.dailySummary.noErrors}
                  </span>
                ) : (
                  <div className="flex flex-col gap-1">
                    {lastReport.errors.map((err, idx) => (
                      <div
                        key={idx}
                        className="rounded border border-red-500/30 bg-red-500/5 px-2 py-1 text-xs text-red-600"
                      >
                        {err}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          </ScrollArea>
        ) : (
          <div className="px-4 py-8 text-center text-sm text-muted-foreground">
            {t.settings.dailySummary.noReport}
          </div>
        )}
      </SettingsSection>
    </PageContainer>
  );
}