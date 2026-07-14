// 工具执行上限设置页 —— 防止 Agent 异常循环的安全阀。
//
// 核心功能:
// 1. 显示当前 ToolLimitsConfig(max_tool_calls / max_failures / timeout)
// 2. 编辑后实时校验(对齐 Rust 端 validate 边界)
// 3. 保存 / 重置为默认值
//
// 与 Effort 的关系:
// - EffortLevel.max_tool_steps 是"每次 send_message 中工具执行轮数"
// - ToolLimits 是更细粒度的配置,当前仅作为用户可见的"安全阀"展示,
//   未来可在 AgentEngine.run_agent_stream 中读取并强制执行。

import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import {
  GaugeIcon,
  SaveIcon,
  RotateCcwIcon,
  RefreshCwIcon,
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
  getToolLimits,
  updateToolLimits,
  resetToolLimits,
  validateToolLimits,
  DEFAULT_TOOL_LIMITS,
  TOOL_LIMITS_BOUNDS,
  type ToolLimitsConfig,
} from "@/features/settings/services/tool-limits";

export function ToolLimitsSettings() {
  const { t } = useI18n();
  const [config, setConfig] = useState<ToolLimitsConfig | null>(null);
  const [draft, setDraft] = useState<ToolLimitsConfig>(DEFAULT_TOOL_LIMITS);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  const loadData = useCallback(async () => {
    try {
      const c = await getToolLimits();
      setConfig(c);
      setDraft(c);
    } catch (e) {
      toast.error(`${t.settings.toolLimits.loadFailed}: ${String(e)}`);
    } finally {
      setLoading(false);
    }
  }, [t.settings.toolLimits.loadFailed]);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  const handleSave = async () => {
    const err = validateToolLimits(draft);
    if (err) {
      toast.error(`${t.settings.toolLimits.validateFailed}: ${err}`);
      return;
    }
    setSaving(true);
    try {
      await updateToolLimits(draft);
      setConfig(draft);
      toast.success(t.settings.toolLimits.saved);
    } catch (e) {
      toast.error(`${t.settings.toolLimits.saveFailed}: ${String(e)}`);
    } finally {
      setSaving(false);
    }
  };

  const handleReset = async () => {
    if (!confirm(t.settings.toolLimits.resetConfirm)) return;
    try {
      await resetToolLimits();
      setConfig(DEFAULT_TOOL_LIMITS);
      setDraft(DEFAULT_TOOL_LIMITS);
      toast.success(t.settings.toolLimits.resetDone);
    } catch (e) {
      toast.error(`${t.settings.toolLimits.resetFailed}: ${String(e)}`);
    }
  };

  const isDirty = config != null && (
    draft.maxToolCallsPerDialog !== config.maxToolCallsPerDialog ||
    draft.maxConsecutiveFailures !== config.maxConsecutiveFailures ||
    draft.timeoutPerCallMs !== config.timeoutPerCallMs
  );

  if (loading || !config) {
    return (
      <PageContainer>
        <LoadingState label={t.settings.toolLimits.loading} />
      </PageContainer>
    );
  }

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <GaugeIcon className="size-5" />
            {t.settings.toolLimits.title}
          </PageTitle>
          <PageDescription>{t.settings.toolLimits.description}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button variant="ghost" size="sm" onClick={() => void loadData()}>
            <RefreshCwIcon className="size-4" />
            {t.settings.toolLimits.refresh}
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => void handleReset()}
          >
            <RotateCcwIcon className="size-4" />
            {t.settings.toolLimits.reset}
          </Button>
          <Button size="sm" onClick={handleSave} disabled={!isDirty || saving}>
            <SaveIcon className="size-4" />
            {saving ? t.settings.toolLimits.saving : t.settings.toolLimits.save}
          </Button>
        </PageActions>
      </PageHeader>

      <SettingsSection
        title={t.settings.toolLimits.sectionLimits}
        description={t.settings.toolLimits.sectionLimitsHint}
      >
        <SettingsRow
          label={t.settings.toolLimits.maxToolCallsLabel}
          description={t.settings.toolLimits.maxToolCallsHint}
        >
          <div className="flex items-center gap-2">
            <Input
              type="number"
              min={TOOL_LIMITS_BOUNDS.maxToolCallsPerDialog.min}
              max={TOOL_LIMITS_BOUNDS.maxToolCallsPerDialog.max}
              value={draft.maxToolCallsPerDialog}
              onChange={(e) =>
                setDraft({ ...draft, maxToolCallsPerDialog: Number(e.target.value) })
              }
              className="w-24"
            />
            <Badge variant="outline">
              {t.settings.toolLimits.unitCalls}
            </Badge>
          </div>
        </SettingsRow>

        <SettingsRow
          label={t.settings.toolLimits.maxFailuresLabel}
          description={t.settings.toolLimits.maxFailuresHint}
        >
          <div className="flex items-center gap-2">
            <Input
              type="number"
              min={TOOL_LIMITS_BOUNDS.maxConsecutiveFailures.min}
              max={TOOL_LIMITS_BOUNDS.maxConsecutiveFailures.max}
              value={draft.maxConsecutiveFailures}
              onChange={(e) =>
                setDraft({ ...draft, maxConsecutiveFailures: Number(e.target.value) })
              }
              className="w-24"
            />
            <Badge variant="outline">
              {t.settings.toolLimits.unitCalls}
            </Badge>
          </div>
        </SettingsRow>

        <SettingsRow
          label={t.settings.toolLimits.timeoutLabel}
          description={t.settings.toolLimits.timeoutHint}
        >
          <div className="flex items-center gap-2">
            <Input
              type="number"
              min={TOOL_LIMITS_BOUNDS.timeoutPerCallMs.min}
              max={TOOL_LIMITS_BOUNDS.timeoutPerCallMs.max}
              step={1000}
              value={draft.timeoutPerCallMs}
              onChange={(e) =>
                setDraft({ ...draft, timeoutPerCallMs: Number(e.target.value) })
              }
              className="w-28"
            />
            <Badge variant="outline">
              {t.settings.toolLimits.unitMs}
            </Badge>
          </div>
        </SettingsRow>
      </SettingsSection>

      {isDirty && (
        <SettingsSection title={t.settings.toolLimits.sectionStatus}>
          <div className="px-4 py-3">
            <Badge variant="outline" className="text-amber-500">
              {t.settings.toolLimits.unsaved}
            </Badge>
          </div>
        </SettingsSection>
      )}

      <SettingsSection
        title={t.settings.toolLimits.sectionDefaults}
        description={t.settings.toolLimits.sectionDefaultsHint}
      >
        <SettingsRow
          label={t.settings.toolLimits.defaultMaxToolCalls}
          description={t.settings.toolLimits.defaultMaxToolCallsHint}
        >
          <Badge>{DEFAULT_TOOL_LIMITS.maxToolCallsPerDialog}</Badge>
        </SettingsRow>
        <SettingsRow
          label={t.settings.toolLimits.defaultMaxFailures}
          description={t.settings.toolLimits.defaultMaxFailuresHint}
        >
          <Badge>{DEFAULT_TOOL_LIMITS.maxConsecutiveFailures}</Badge>
        </SettingsRow>
        <SettingsRow
          label={t.settings.toolLimits.defaultTimeout}
          description={t.settings.toolLimits.defaultTimeoutHint}
        >
          <Badge>{DEFAULT_TOOL_LIMITS.timeoutPerCallMs} ms</Badge>
        </SettingsRow>
      </SettingsSection>
    </PageContainer>
  );
}
