/**
 * ═══════════════════════════════════════════════════════════════════════════
 * AboutSettings - 关于页面设置
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { ExternalLinkIcon, InfoIcon } from "lucide-react";
import { getName, getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useI18n } from "@/locales/i18n";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
} from "@/components/shared/page-layout";
import { SettingsSection, SettingsRow } from "@/features/settings/components/settings-section";

// ── 常量配置 ────────────────────────────────────────────────────────────────

const APP_BUNDLE_ID = "com.admin.mnemosyne";
const APP_LICENSE = "MIT";
const REPO_URL = "https://github.com/admin/Mnemosyne";
const WEBSITE_URL = "https://github.com/admin/Mnemosyne";

// ── 辅助组件 ────────────────────────────────────────────────────────────────

/**
 * GitHub 图标组件
 */
function GithubIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" className={className} aria-hidden="true">
      <path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12" />
    </svg>
  );
}

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/**
 * 检测操作系统平台
 */
function detectPlatform(): string {
  const ua = navigator.userAgent;
  if (ua.includes("Win")) return "Windows";
  if (ua.includes("Mac")) return "macOS";
  if (ua.includes("Linux")) return "Linux";
  return "Unknown";
}

/**
 * 检测 CPU 架构
 */
function detectArch(): string {
  const ua = navigator.userAgent;
  if (ua.includes("x64") || ua.includes("Win64") || ua.includes("x86_64")) return "x86_64";
  if (ua.includes("arm64") || ua.includes("aarch64")) return "arm64";
  return "unknown";
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 关于页面，展示应用信息、版本和源代码链接
 */
export function AboutSettings() {
  const { t } = useI18n();
  const [name, setName] = useState("Mnemosyne");
  const [version, setVersion] = useState("0.1.0");
  const [platform] = useState(detectPlatform);
  const [arch] = useState(detectArch);

  // ── 初始化 ────────────────────────────────────────────────────────────────

  useEffect(() => {
    let cancelled = false;
    getName()
      .then((n) => {
        if (!cancelled) setName(n);
      })
      .catch((err) => { console.error("[AboutSettings] get app name failed:", err); });
    getVersion()
      .then((v) => {
        if (!cancelled) setVersion(v);
      })
      .catch((err) => { console.error("[AboutSettings] get app version failed:", err); });
    return () => {
      cancelled = true;
    };
  }, []);

  // ── 渲染 ──────────────────────────────────────────────────────────────────

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.settings.about}</PageTitle>
          <PageDescription>{t.settings.aboutDesc}</PageDescription>
        </PageHeading>
      </PageHeader>

      {/* ── 应用信息卡片 ────────────────────────────────────────────────────── */}
      <Card className="py-0">
        <CardContent className="p-0">
          <div className="flex items-center gap-4 px-4 py-4 border-b">
            <div className="flex size-12 shrink-0 items-center justify-center rounded-xl bg-[var(--bg-brand-popup)] text-[var(--text-brand)]">
              <InfoIcon className="size-6" />
            </div>
            <div className="flex min-w-0 flex-col gap-0.5">
              <span className="text-base font-semibold tracking-tight">{name}</span>
              <span className="text-xs text-muted-foreground">{t.app.description}</span>
              <span className="mt-0.5 font-mono text-[11px] text-muted-foreground">v{version}</span>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* ── 版本信息 ────────────────────────────────────────────────────────── */}
      <SettingsSection title={t.settings.aboutVersion}>
        <SettingsRow label={t.settings.aboutVersion}>
          <Badge variant="secondary" className="font-mono text-xs">v{version}</Badge>
        </SettingsRow>
        <SettingsRow label={t.settings.aboutBuild}>
          <span className="text-sm text-muted-foreground">{platform} · {arch}</span>
        </SettingsRow>
        <SettingsRow label={t.settings.aboutBundleId}>
          <span className="font-mono text-xs text-muted-foreground">{APP_BUNDLE_ID}</span>
        </SettingsRow>
        <SettingsRow label={t.common.framework}>
          <span className="text-sm text-muted-foreground">Tauri v2 + React 19</span>
        </SettingsRow>
        <SettingsRow label={t.common.runtime}>
          <span className="text-sm text-muted-foreground">Vite + TypeScript</span>
        </SettingsRow>
        <SettingsRow label={t.settings.aboutLicense}>
          <Badge variant="outline" className="text-xs">{APP_LICENSE}</Badge>
        </SettingsRow>
      </SettingsSection>

      {/* ── 源代码链接 ──────────────────────────────────────────────────────── */}
      <SettingsSection title={t.settings.aboutSourceCode}>
        <SettingsRow label={t.settings.aboutSourceCode}>
          <Button
            variant="ghost"
            size="sm"
            className="h-8 gap-1.5 px-2 text-xs"
            onClick={() => void openUrl(REPO_URL)}
          >
            <GithubIcon className="size-3.5" />
            <span className="max-w-[180px] truncate">{REPO_URL.replace("https://", "")}</span>
            <ExternalLinkIcon className="size-3 opacity-60" />
          </Button>
        </SettingsRow>
        <SettingsRow label={t.settings.aboutWebsite}>
          <Button
            variant="ghost"
            size="sm"
            className="h-8 gap-1.5 px-2 text-xs"
            onClick={() => void openUrl(WEBSITE_URL)}
          >
            <ExternalLinkIcon className="size-3.5" />
            <span className="max-w-[180px] truncate">{WEBSITE_URL.replace("https://", "")}</span>
          </Button>
        </SettingsRow>
      </SettingsSection>

      {/* ── 操作按钮 ────────────────────────────────────────────────────────── */}
      <div className="flex flex-wrap gap-2">
        <Button variant="outline" size="sm" onClick={() => void openUrl(REPO_URL)}>
          <GithubIcon className="size-4" />
          {t.settings.aboutViewOnGithub}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => void openUrl(`${REPO_URL}/issues/new`)}
        >
          {t.settings.aboutReportIssue}
        </Button>
      </div>
    </PageContainer>
  );
}