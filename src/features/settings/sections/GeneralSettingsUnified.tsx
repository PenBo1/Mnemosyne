/**
 * ═══════════════════════════════════════════════════════════════════════════
 * GeneralSettingsUnified - 通用设置合并页面
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useState, useEffect, useRef, useMemo } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  FolderOpenIcon,
  CopyIcon,
  CheckIcon,
  RotateCcwIcon,
  Trash2Icon,
  SearchIcon,
  ExternalLinkIcon,
  InfoIcon,
} from "lucide-react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getName, getVersion } from "@tauri-apps/api/app";
import { toast } from "sonner";
import { useTheme } from "@/lib/theme";
import { useI18n } from "@/locales/i18n";
import { SettingsSection, SettingsRow } from "@/features/settings/components/settings-section";
import { useGeneralSettings } from "@/features/settings/hooks";
import { getDataDirPath } from "@/features/settings/services";
import { useShortcuts } from "@/features/settings/hooks";
import {
  SHORTCUTS,
  SHORTCUT_GROUPS,
  getBindingTokens,
  type KeyBinding,
  type ShortcutId,
} from "@/lib/shortcuts";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
} from "@/components/shared/page-layout";
import type { LogLevel } from "@/services/settings";

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/**
 * 获取日志级别标签
 */
function getLogLevelLabel(t: ReturnType<typeof useI18n>["t"], level: LogLevel): string {
  return t.common.logLevels[level] || level;
}

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

// ── 常量配置 ────────────────────────────────────────────────────────────────

const APP_BUNDLE_ID = "com.admin.mnemosyne";
const APP_LICENSE = "MIT";
const REPO_URL = "https://github.com/admin/Mnemosyne";
const WEBSITE_URL = "https://github.com/admin/Mnemosyne";

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

// ── 子组件 ──────────────────────────────────────────────────────────────────

/**
 * 快捷键行组件
 */
function ShortcutRow({
  shortcut,
  isRecording,
  onStartRecording,
  onStopRecording,
  onRecord,
  onClear,
  onReset,
  userBindings,
}: {
  shortcut: { id: ShortcutId; group: string; defaultBindings?: KeyBinding[] };
  isRecording: boolean;
  onStartRecording: () => void;
  onStopRecording: () => void;
  onRecord: (b: KeyBinding) => void;
  onClear: () => void;
  onReset: () => void;
  userBindings?: KeyBinding[];
}) {
  const { t } = useI18n();
  const label = t.shortcuts.items[shortcut.id as keyof typeof t.shortcuts.items] ?? shortcut.id;

  const isModified = userBindings !== undefined;
  const bindings = isModified ? userBindings : shortcut.defaultBindings;
  const hasBindings = bindings && bindings.length > 0;

  return (
    <div className="group flex items-center justify-between gap-4 px-4 py-2.5 transition-colors hover:bg-muted/30">
      <span className="text-sm">{label}</span>
      <div className="flex items-center gap-2">
        {isRecording ? (
          <Recorder onRecord={onRecord} onCancel={onStopRecording} />
        ) : (
          <>
            <button
              type="button"
              onClick={onStartRecording}
              className="flex min-w-[100px] cursor-pointer items-center justify-end gap-1"
            >
              {hasBindings ? (
                getBindingTokens(bindings[0]).map((tok, i) => (
                  <Badge
                    key={i}
                    variant="outline"
                    className="gap-0.5 font-mono text-[11px] font-normal transition-colors group-hover:bg-accent group-hover:text-accent-foreground"
                  >
                    <span className="px-0.5">{tok}</span>
                  </Badge>
                ))
              ) : (
                <span className="text-[11px] text-muted-foreground italic">
                  {t.shortcuts.unassigned}
                </span>
              )}
            </button>
            <div className="flex items-center gap-1">
              {isModified && (
                <Button
                  variant="ghost"
                  size="icon-sm"
                  className="text-muted-foreground hover:text-foreground"
                  onClick={onReset}
                  title={t.shortcuts.resetToDefault}
                >
                  <RotateCcwIcon className="size-3.5" />
                </Button>
              )}
              <Button
                variant="ghost"
                size="icon-sm"
                className="text-muted-foreground hover:text-destructive opacity-0 transition-opacity group-hover:opacity-100"
                onClick={onClear}
                title={t.shortcuts.clearShortcut}
              >
                <Trash2Icon className="size-3.5" />
              </Button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

/**
 * 快捷键录制器组件
 */
function Recorder({
  onRecord,
  onCancel,
}: {
  onRecord: (b: KeyBinding) => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();

  useEffect(() => {
    const onDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      if (e.key === "Escape") {
        onCancel();
        return;
      }

      const isMod = ["Control", "Shift", "Alt", "Meta"].includes(e.key);
      if (isMod) return;

      const hasPrimaryModifier = e.ctrlKey || e.altKey || e.metaKey;
      const isCharacterKey = e.key.length === 1;
      if (!hasPrimaryModifier && (!e.shiftKey || isCharacterKey)) {
        return;
      }

      onRecord({
        key: e.key,
        ctrl: e.ctrlKey,
        shift: e.shiftKey,
        alt: e.altKey,
        meta: e.metaKey,
      });
    };

    window.addEventListener("keydown", onDown, { capture: true });
    return () => window.removeEventListener("keydown", onDown, { capture: true });
  }, [onRecord, onCancel]);

  return (
    <div className="flex items-center gap-2 rounded bg-accent/50 px-2 py-1 text-[11px] ring-1 ring-accent">
      <span className="animate-pulse font-medium">{t.shortcuts.recording}</span>
      <span className="text-muted-foreground">{t.shortcuts.recordingHint}</span>
    </div>
  );
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 通用设置合并页面，包含基础设置、系统设置、快捷键和关于
 */
export function GeneralSettingsUnified() {
  const { theme, setTheme } = useTheme();
  const { locale, setLocale, t } = useI18n();
  // 使用 any 绕过类型检查
  const tg = (t.settings as any).settingsGeneral;
  const {
    notifications,
    logLevel,
    logLevelChanged,
    restoreWindow,
    toggleNotifications,
    changeLogLevel,
    toggleRestoreWindow,
  } = useGeneralSettings();

  // ── 系统设置状态 ────────────────────────────────────────────────────────

  const [dataDir, setDataDir] = useState<string>("");
  const [copied, setCopied] = useState(false);
  const copyTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // ── 关于设置状态 ────────────────────────────────────────────────────────

  const [appName, setAppName] = useState("Mnemosyne");
  const [appVersion, setAppVersion] = useState("0.1.0");
  const [platform] = useState(detectPlatform);
  const [arch] = useState(detectArch);

  // ── 快捷键状态 ──────────────────────────────────────────────────────────

  const shortcuts = useShortcuts();
  const [query, setQuery] = useState("");
  const [recordingId, setRecordingId] = useState<ShortcutId | null>(null);
  const [resetAllOpen, setResetAllOpen] = useState(false);

  // ── 初始化 ──────────────────────────────────────────────────────────────

  // 加载数据目录
  useEffect(() => {
    let cancelled = false;
    getDataDirPath()
      .then((path) => {
        if (!cancelled) setDataDir(path);
      })
      .catch((e) => {
        console.error("[system] load data dir path failed", e);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // 加载应用信息
  useEffect(() => {
    let cancelled = false;
    getName()
      .then((n) => {
        if (!cancelled) setAppName(n);
      })
      .catch((err) => {
        console.error("[AboutSettings] get app name failed:", err);
      });
    getVersion()
      .then((v) => {
        if (!cancelled) setAppVersion(v);
      })
      .catch((err) => {
        console.error("[AboutSettings] get app version failed:", err);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // 清理定时器
  useEffect(() => {
    return () => {
      if (copyTimerRef.current) clearTimeout(copyTimerRef.current);
    };
  }, []);

  // ── 事件处理 ────────────────────────────────────────────────────────────

  const handleOpen = async () => {
    if (!dataDir) return;
    try {
      await revealItemInDir(dataDir);
    } catch (e) {
      console.error("[system] reveal data dir failed", e);
      toast.error(t.common.failedToOpen);
    }
  };

  const handleCopy = async () => {
    if (!dataDir) return;
    try {
      await navigator.clipboard.writeText(dataDir);
      setCopied(true);
      toast.success(t.common.copiedToClipboard);
      if (copyTimerRef.current) clearTimeout(copyTimerRef.current);
      copyTimerRef.current = setTimeout(() => setCopied(false), 1500);
    } catch (e) {
      console.error("[system] copy path failed", e);
      toast.error(t.common.failedToCopy);
    }
  };

  // ── 快捷键过滤 ──────────────────────────────────────────────────────────

  const filtered = useMemo(() => {
    if (!query.trim()) return SHORTCUTS;
    const q = query.toLowerCase();
    return SHORTCUTS.filter((s) => {
      const label = t.shortcuts.items[s.id as keyof typeof t.shortcuts.items] ?? s.id;
      const group = t.shortcuts.groups[s.group];
      return label.toLowerCase().includes(q) || group.toLowerCase().includes(q);
    });
  }, [query, t]);

  const onRecord = (id: ShortcutId, binding: KeyBinding) => {
    void shortcuts.record(id, binding);
    setRecordingId(null);
  };

  const onClear = (id: ShortcutId) => {
    void shortcuts.clear(id);
  };

  const onReset = (id: ShortcutId) => {
    void shortcuts.reset(id);
  };

  // ── 渲染 ────────────────────────────────────────────────────────────────

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.settings.general}</PageTitle>
          <PageDescription>{t.settings.generalDesc}</PageDescription>
        </PageHeading>
      </PageHeader>

      <Tabs defaultValue="basic" className="w-full">
        <TabsList className="mb-4">
          <TabsTrigger value="basic">{tg?.basic ?? t.settings.general}</TabsTrigger>
          <TabsTrigger value="system">{tg?.system ?? t.settings.system}</TabsTrigger>
          <TabsTrigger value="shortcuts">{t.settings.shortcuts}</TabsTrigger>
          <TabsTrigger value="about">{t.settings.about}</TabsTrigger>
        </TabsList>

        {/* ── 基础设置 ────────────────────────────────────────────────────── */}
        <TabsContent value="basic" className="space-y-4">
          <SettingsSection title={t.settings.field.language}>
            <SettingsRow label={t.settings.field.language} description={t.settings.description.language}>
              <Select value={locale} onValueChange={(v) => setLocale(v as "en" | "zh")}>
                <SelectTrigger className="w-32">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="en">English</SelectItem>
                  <SelectItem value="zh">中文</SelectItem>
                </SelectContent>
              </Select>
            </SettingsRow>
          </SettingsSection>

          <SettingsSection title={t.settings.description.theme}>
            <SettingsRow label={t.settings.field.theme} description={t.settings.description.theme}>
              <Select value={theme} onValueChange={(v) => setTheme(v as "light" | "dark" | "system")}>
                <SelectTrigger className="w-32">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="light">{t.settings.themeLight}</SelectItem>
                  <SelectItem value="dark">{t.settings.themeDark}</SelectItem>
                  <SelectItem value="system">{t.settings.themeSystem}</SelectItem>
                </SelectContent>
              </Select>
            </SettingsRow>
            <SettingsRow label={t.settings.field.notifications} description={t.settings.description.notifications}>
              <Switch checked={notifications} onCheckedChange={toggleNotifications} />
            </SettingsRow>
          </SettingsSection>

          <SettingsSection title={t.settings.startup}>
            <SettingsRow label={t.settings.field.restoreWindow} description={t.settings.description.restoreWindow}>
              <Switch checked={restoreWindow} onCheckedChange={toggleRestoreWindow} />
            </SettingsRow>
          </SettingsSection>

          <SettingsSection title={t.settings.field.logLevel}>
            <div className="flex flex-col gap-3 px-4 py-3">
              <div className="flex items-center justify-between">
                <div className="flex flex-col gap-0.5">
                  <span className="text-sm font-medium">{t.settings.field.logLevel}</span>
                  <span className="text-xs text-muted-foreground">{t.settings.description.logLevel}</span>
                </div>
                <Select value={logLevel} onValueChange={changeLogLevel}>
                  <SelectTrigger className="w-32">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {(["trace", "debug", "info", "warn", "error"] as LogLevel[]).map((level) => (
                      <SelectItem key={level} value={level}>{getLogLevelLabel(t, level)}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              {logLevelChanged && (
                <Alert>
                  <AlertDescription>{t.settings.logLevelRestartRequired}</AlertDescription>
                </Alert>
              )}
            </div>
          </SettingsSection>
        </TabsContent>

        {/* ── 系统设置 ────────────────────────────────────────────────────── */}
        <TabsContent value="system" className="space-y-4">
          <SettingsSection title={t.settings.field.dataDirPath}>
            <SettingsRow
              label={t.settings.field.dataDirPath}
              description={t.settings.description.dataDirPath}
            >
              <div className="flex items-center gap-1.5">
                <Button
                  variant="outline"
                  size="sm"
                  className="h-8 gap-1.5 px-2 text-xs"
                  disabled={!dataDir}
                  onClick={() => void handleOpen()}
                >
                  <FolderOpenIcon className="size-3.5" />
                  {t.settings.openDataDir}
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-8 gap-1.5 px-2 text-xs"
                  disabled={!dataDir}
                  onClick={() => void handleCopy()}
                >
                  {copied ? (
                    <CheckIcon className="size-3.5" />
                  ) : (
                    <CopyIcon className="size-3.5" />
                  )}
                  {copied ? t.common.copied : t.settings.copyPath}
                </Button>
              </div>
            </SettingsRow>
            {dataDir && (
              <div className="px-4 py-3 border-t">
                <p className="font-mono text-xs text-muted-foreground break-all">{dataDir}</p>
              </div>
            )}
          </SettingsSection>
        </TabsContent>

        {/* ── 快捷键设置 ────────────────────────────────────────────────── */}
        <TabsContent value="shortcuts" className="space-y-4">
          <div className="flex items-center justify-between">
            <div className="relative flex-1 max-w-sm">
              <SearchIcon className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder={t.shortcuts.searchPlaceholder}
                className="h-9 pl-9 text-sm"
              />
            </div>
            <Button
              variant="outline"
              size="sm"
              className="h-8 gap-1.5"
              onClick={() => setResetAllOpen(true)}
            >
              <RotateCcwIcon className="size-3.5" />
              {t.shortcuts.resetAll}
            </Button>
          </div>

          <div className="flex flex-col gap-6">
            {SHORTCUT_GROUPS.map((group) => {
              const items = filtered.filter((s) => s.group === group);
              if (items.length === 0) return null;
              return (
                <div key={group} className="flex flex-col gap-2">
                  <h3 className="text-xs font-semibold tracking-wider text-muted-foreground uppercase">
                    {t.shortcuts.groups[group]}
                  </h3>
                  <Card className="py-0">
                    <CardContent className="divide-y px-0">
                      {items.map((s) => (
                        <ShortcutRow
                          key={s.id}
                          shortcut={s}
                          isRecording={recordingId === s.id}
                          onStartRecording={() => setRecordingId(s.id)}
                          onStopRecording={() => setRecordingId(null)}
                          onRecord={(b) => onRecord(s.id, b)}
                          onClear={() => onClear(s.id)}
                          onReset={() => onReset(s.id)}
                          userBindings={shortcuts.overrides[s.id]}
                        />
                      ))}
                    </CardContent>
                  </Card>
                </div>
              );
            })}
          </div>

          <p className="text-xs text-muted-foreground">{t.shortcuts.customizeHint}</p>

          {/* ── 重置全部确认对话框 ──────────────────────────────────────── */}
          <Dialog open={resetAllOpen} onOpenChange={setResetAllOpen}>
            <DialogContent className="max-w-md">
              <DialogHeader>
                <DialogTitle>{t.shortcuts.resetAllTitle}</DialogTitle>
              </DialogHeader>
              <p className="text-sm text-muted-foreground">{t.shortcuts.resetAllDesc}</p>
              <DialogFooter>
                <Button variant="outline" onClick={() => setResetAllOpen(false)}>
                  {t.common.cancel}
                </Button>
                <Button
                  variant="destructive"
                  onClick={() => {
                    setResetAllOpen(false);
                    void shortcuts.resetAll();
                  }}
                >
                  {t.shortcuts.resetAllConfirm}
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        </TabsContent>

        {/* ── 关于 ────────────────────────────────────────────────────────── */}
        <TabsContent value="about" className="space-y-4">
          <Card className="py-0">
            <CardContent className="p-0">
              <div className="flex items-center gap-4 px-4 py-4 border-b">
                <div className="flex size-12 shrink-0 items-center justify-center rounded-xl bg-[var(--bg-brand-popup)] text-[var(--text-brand)]">
                  <InfoIcon className="size-6" />
                </div>
                <div className="flex min-w-0 flex-col gap-0.5">
                  <span className="text-base font-semibold tracking-tight">{appName}</span>
                  <span className="text-xs text-muted-foreground">{t.app.description}</span>
                  <span className="mt-0.5 font-mono text-[11px] text-muted-foreground">v{appVersion}</span>
                </div>
              </div>
            </CardContent>
          </Card>

          <SettingsSection title={t.settings.aboutVersion}>
            <SettingsRow label={t.settings.aboutVersion}>
              <Badge variant="secondary" className="font-mono text-xs">v{appVersion}</Badge>
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
        </TabsContent>
      </Tabs>
    </PageContainer>
  );
}