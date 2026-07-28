/**
 * ═══════════════════════════════════════════════════════════════════════════
 * EmbeddingSettings - 向量嵌入设置页面
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Badge } from "@/components/ui/badge";
import { useI18n } from "@/locales/i18n";
import { toast } from "sonner";
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
  embeddingGetConfig,
  embeddingSetConfig,
  embeddingTest,
  embeddingStats,
  type EmbeddingConfig,
  type VectorStats,
} from "@/features/settings/services";

// ── 常量配置 ────────────────────────────────────────────────────────────────

/** 默认嵌入配置 */
const DEFAULT_CONFIG: EmbeddingConfig = {
  enabled: false,
  baseUrl: "http://localhost:11434/v1",
  apiKey: "",
  model: "nomic-embed-text",
  dim: 768,
};

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 向量嵌入设置页面，配置嵌入模型和查看索引统计
 */
export function EmbeddingSettings() {
  const { t } = useI18n();
  const [config, setConfig] = useState<EmbeddingConfig>(DEFAULT_CONFIG);
  const [stats, setStats] = useState<VectorStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState(false);

  // ── 数据加载 ──────────────────────────────────────────────────────────────

  useEffect(() => {
    let cancelled = false;
    (async () => {
      setLoading(true);
      try {
        const [cfg, st] = await Promise.all([
          embeddingGetConfig(),
          embeddingStats().catch(() => null),
        ]);
        if (cancelled) return;
        setConfig(cfg);
        setStats(st);
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => { cancelled = true; };
  }, []);

  // ── 事件处理 ──────────────────────────────────────────────────────────────

  /**
   * 切换启用状态
   */
  const handleToggleEnabled = async (checked: boolean) => {
    const prev = config.enabled;
    setConfig({ ...config, enabled: checked });
    try {
      await embeddingSetConfig({ ...config, enabled: checked });
      toast.success(t.common.updatedSuccessfully);
    } catch {
      setConfig({ ...config, enabled: prev });
      toast.error(t.common.failedToUpdate);
    }
  };

  /**
   * 保存配置
   */
  const handleSaveConfig = async () => {
    setSaving(true);
    try {
      await embeddingSetConfig(config);
      toast.success(t.common.updatedSuccessfully);
    } catch {
      toast.error(t.common.failedToUpdate);
    } finally {
      setSaving(false);
    }
  };

  /**
   * 测试连接
   */
  const handleTest = async () => {
    // 测试前先保存当前配置
    setTesting(true);
    try {
      await embeddingSetConfig(config);
      const dim = await embeddingTest();
      toast.success(`${t.settings.embeddingSettings.testSuccess} (dim=${dim})`);
    } catch (e) {
      toast.error(`${t.settings.embeddingSettings.testFailed}: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setTesting(false);
    }
  };

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
          <PageTitle>{t.settings.embedding}</PageTitle>
          <PageDescription>{t.settings.embeddingDesc}</PageDescription>
        </PageHeading>
        {config.enabled && (
          <PageActions>
            <Button variant="outline" size="sm" onClick={handleTest} disabled={testing}>
              {testing ? t.settings.embeddingSettings.testing : t.settings.embeddingSettings.testConnection}
            </Button>
            <Button size="sm" onClick={handleSaveConfig} disabled={saving}>
              {saving ? t.common.saving : t.common.save}
            </Button>
          </PageActions>
        )}
      </PageHeader>

      {/* ── 功能开关 ────────────────────────────────────────────────────────── */}
      <SettingsSection title={t.settings.embeddingSettings.featureToggle}>
        <SettingsRow
          label={t.settings.embeddingSettings.featureToggle}
          description={t.settings.embeddingSettings.featureToggleHint}
        >
          <Switch
            checked={config.enabled}
            onCheckedChange={handleToggleEnabled}
          />
        </SettingsRow>
      </SettingsSection>

      {/* ── 配置表单 ────────────────────────────────────────────────────────── */}
      {config.enabled && (
        <SettingsSection title={t.settings.embeddingSettings.title}>
          <SettingsRow
            label={t.settings.embeddingSettings.baseUrl}
            description={t.settings.embeddingSettings.baseUrlHint}
          >
            <Input
              value={config.baseUrl}
              placeholder="http://localhost:11434/v1"
              onChange={(e) => setConfig({ ...config, baseUrl: e.target.value })}
              className="w-80"
            />
          </SettingsRow>

          <SettingsRow
            label={t.settings.embeddingSettings.model}
            description={t.settings.embeddingSettings.modelHint}
          >
            <Input
              value={config.model}
              placeholder="nomic-embed-text"
              onChange={(e) => setConfig({ ...config, model: e.target.value })}
              className="w-56"
            />
          </SettingsRow>

          <SettingsRow
            label={t.settings.embeddingSettings.apiKey}
            description={t.settings.embeddingSettings.apiKeyHint}
          >
            <Input
              type="password"
              value={config.apiKey}
              placeholder={t.settings.embeddingSettings.apiKeyPlaceholder}
              onChange={(e) => setConfig({ ...config, apiKey: e.target.value })}
              className="w-56"
            />
          </SettingsRow>

          <SettingsRow
            label={t.settings.embeddingSettings.dim}
            description={t.settings.embeddingSettings.dimHint}
          >
            <Input
              type="number"
              value={config.dim}
              onChange={(e) =>
                setConfig({ ...config, dim: parseInt(e.target.value, 10) || 0 })
              }
              className="w-28"
            />
          </SettingsRow>
        </SettingsSection>
      )}

      {/* ── 索引统计 ────────────────────────────────────────────────────────── */}
      {config.enabled && stats && (
        <SettingsSection title={t.settings.embeddingSettings.statsTitle}>
          <SettingsRow
            label={t.settings.embeddingSettings.totalVectors}
            description={undefined}
          >
            <Badge variant="secondary">{stats.total}</Badge>
          </SettingsRow>
          {stats.byDocType.map((item) => (
            <SettingsRow
              key={item.docType}
              label={item.docType}
            >
              <Badge variant="outline">{item.count}</Badge>
            </SettingsRow>
          ))}
        </SettingsSection>
      )}

      {/* ── 未启用说明 ──────────────────────────────────────────────────────── */}
      {!config.enabled && (
        <SettingsSection title={t.settings.embeddingSettings.title}>
          <div className="px-4 py-3">
            <p className="text-sm text-muted-foreground">
              {t.settings.embeddingSettings.disabledHint}
            </p>
          </div>
        </SettingsSection>
      )}
    </PageContainer>
  );
}