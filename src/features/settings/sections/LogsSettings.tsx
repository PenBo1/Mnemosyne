/**
 * ═══════════════════════════════════════════════════════════════════════════
 * LogsSettings - 日志查看设置页面
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useEffect, useState, useRef, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  RefreshCwIcon,
  Trash2Icon,
  FolderOpenIcon,
  ScrollTextIcon,
  CopyIcon,
  CheckIcon,
  PauseIcon,
  PlayIcon,
} from "lucide-react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
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
import {
  SettingsSection,
  SettingsRow,
} from "@/features/settings/components/settings-section";
import {
  listLogFiles,
  readLogFile,
  clearLogFile,
  type LogFileInfo,
} from "@/features/settings/services/logs";
import { getDataDirPath } from "@/features/settings/services";

const DISPLAY_LINE_CAP = 5000;
const AUTO_REFRESH_INTERVAL_MS = 2000;

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
}

/** tracing 默认格式: `<RFC3339 时间戳> <LEVEL> <target>: <message>` */
interface LogLineParts {
  timestamp: string;
  level: string;
  target: string;
  message: string;
}

// 匹配 tracing 默认 fmt 输出：时间戳 级别 target: 消息
// 例: "2026-07-11T10:30:45.123456Z  INFO module::path: some message"
const LOG_LINE_RE = /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z?)\s+(TRACE|DEBUG|INFO|WARN|ERROR)\s+([^:]+):\s?(.*)$/;

/** 解析单行日志为各组成部分；无法解析时返回 null（整行作为 message） */
function parseLogLine(line: string): LogLineParts | null {
  const match = line.match(LOG_LINE_RE);
  if (!match) return null;
  return {
    timestamp: match[1],
    level: match[2],
    target: match[3],
    message: match[4] || "",
  };
}

/** 日志级别 → 文字色 */
const LEVEL_COLORS: Record<string, string> = {
  ERROR: "text-[var(--status-error-default)]",
  WARN: "text-[var(--status-warning-default)]",
  INFO: "text-[var(--status-info-default, rgb(59 130 246))]",
  DEBUG: "text-muted-foreground",
  TRACE: "text-muted-foreground/70",
};

export function LogsSettings() {
  const { t } = useI18n();
  const [files, setFiles] = useState<LogFileInfo[]>([]);
  const [selected, setSelected] = useState<string>("");
  const [content, setContent] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [autoRefresh, setAutoRefresh] = useState(false);
  const [copied, setCopied] = useState(false);
  const [logsDir, setLogsDir] = useState<string>("");
  const viewportRef = useRef<HTMLDivElement>(null);
  const autoScrollRef = useRef(true);
  const copyTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // 获取日志目录路径（data_dir/logs）
  useEffect(() => {
    let cancelled = false;
    getDataDirPath()
      .then((dir) => {
        if (!cancelled) {
          // data_dir/logs 子目录,Windows 与 Unix 均接受 `/` 分隔
          setLogsDir(`${dir}/logs`);
        }
      })
      .catch((e) => console.error("[logs] get data dir failed", e));
    return () => {
      cancelled = true;
    };
  }, []);

  // 加载日志文件列表
  const loadFiles = useCallback(async () => {
    try {
      const list = await listLogFiles();
      setFiles(list);
      if (list.length > 0 && !list.some((f) => f.name === selected)) {
        setSelected(list[0].name);
      } else if (list.length === 0) {
        setSelected("");
        setContent("");
      }
    } catch (e) {
      console.error("[logs] load files failed", e);
      toast.error(t.common.failedToLoad);
    }
  }, [selected, t.common.failedToLoad]);

  // 加载当前选中日志文件内容
  const loadContent = useCallback(async () => {
    if (!selected) {
      setContent("");
      return;
    }
    setLoading(true);
    try {
      const text = await readLogFile(selected);
      setContent(text);
    } catch (e) {
      console.error("[logs] read file failed", e);
      toast.error(t.common.failedToLoad);
    } finally {
      setLoading(false);
    }
  }, [selected, t.common.failedToLoad]);

  // 初始加载文件列表
  useEffect(() => {
    void loadFiles();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 选中文件变化时加载内容
  useEffect(() => {
    void loadContent();
  }, [selected, loadContent]);

  // 自动刷新轮询
  useEffect(() => {
    if (!autoRefresh || !selected) return;
    const timer = setInterval(() => {
      void loadContent();
    }, AUTO_REFRESH_INTERVAL_MS);
    return () => clearInterval(timer);
  }, [autoRefresh, selected, loadContent]);

  // 内容更新后自动滚动到底部
  useEffect(() => {
    if (autoScrollRef.current && viewportRef.current) {
      viewportRef.current.scrollTop = viewportRef.current.scrollHeight;
    }
  }, [content]);

  // 组件卸载时清理 copy 状态定时器，避免卸载后 setState
  useEffect(() => {
    return () => {
      if (copyTimerRef.current) clearTimeout(copyTimerRef.current);
    };
  }, []);

  const handleRefresh = useCallback(() => {
    void loadFiles();
    void loadContent();
  }, [loadFiles, loadContent]);

  const handleClear = useCallback(async () => {
    if (!selected) return;
    try {
      await clearLogFile(selected);
      setContent("");
      toast.success(t.settings.logs.cleared);
      void loadFiles();
    } catch (e) {
      console.error("[logs] clear failed", e);
      toast.error(t.common.failedToUpdate);
    }
  }, [selected, t.settings.logs.cleared, t.common.failedToUpdate, loadFiles]);

  const handleCopy = useCallback(async () => {
    if (!content) return;
    try {
      await navigator.clipboard.writeText(content);
      setCopied(true);
      toast.success(t.common.copiedToClipboard);
      if (copyTimerRef.current) clearTimeout(copyTimerRef.current);
      copyTimerRef.current = setTimeout(() => setCopied(false), 1500);
    } catch (e) {
      console.error("[logs] copy failed", e);
      toast.error(t.common.failedToCopy);
    }
  }, [content, t.common.copiedToClipboard, t.common.failedToCopy]);

  const handleOpenFolder = useCallback(async () => {
    if (!logsDir) return;
    try {
      await revealItemInDir(logsDir);
    } catch (e) {
      console.error("[logs] open folder failed", e);
      toast.error(t.common.failedToOpen);
    }
  }, [logsDir, t.common.failedToOpen]);

  const handleScroll = useCallback(() => {
    const el = viewportRef.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
    autoScrollRef.current = atBottom;
  }, []);

  const current = files.find((f) => f.name === selected);

  // 截取尾部行以避免渲染过慢
  const lines = content.split("\n");
  const truncated = lines.length > DISPLAY_LINE_CAP;
  const displayLines = truncated ? lines.slice(-DISPLAY_LINE_CAP) : lines;

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>{t.settings.logs.label}</PageTitle>
          <PageDescription>{t.settings.logs.description}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button
            variant="outline"
            size="sm"
            className="h-8 gap-1.5 px-2 text-xs"
            disabled={!logsDir}
            onClick={() => void handleOpenFolder()}
          >
            <FolderOpenIcon className="size-3.5" />
            {t.settings.logs.openFolder}
          </Button>
          <Button
            variant="outline"
            size="sm"
            className="h-8 gap-1.5 px-2 text-xs"
            onClick={() => void handleRefresh()}
          >
            <RefreshCwIcon className={`size-3.5 ${loading ? "animate-spin" : ""}`} />
            {t.common.reload}
          </Button>
        </PageActions>
      </PageHeader>

      <SettingsSection title={t.settings.logs.fileSelect}>
        <SettingsRow
          label={t.settings.logs.file}
          description={t.settings.logs.fileHint}
        >
          <Select value={selected} onValueChange={setSelected}>
            <SelectTrigger className="w-64">
              <SelectValue placeholder={t.settings.logs.noFiles} />
            </SelectTrigger>
            <SelectContent>
              {files.map((f) => (
                <SelectItem key={f.name} value={f.name}>
                  {f.name} · {formatSize(f.size)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </SettingsRow>
        <SettingsRow
          label={t.settings.logs.autoRefresh}
          description={t.settings.logs.autoRefreshHint}
        >
          <div className="flex items-center gap-1.5">
            {autoRefresh ? (
              <PauseIcon className="size-3.5 text-muted-foreground" />
            ) : (
              <PlayIcon className="size-3.5 text-muted-foreground" />
            )}
            <Switch checked={autoRefresh} onCheckedChange={setAutoRefresh} />
          </div>
        </SettingsRow>
      </SettingsSection>

      <SettingsSection
        title={t.settings.logs.content}
        description={
          current
            ? `${current.name} · ${formatSize(current.size)}${
                current.modified ? ` · ${current.modified}` : ""
              }`
            : undefined
        }
      >
        <div className="flex items-center justify-between gap-2 px-4 py-2 border-b">
          <span className="text-xs text-muted-foreground">
            {loading
              ? t.common.loading
              : content
              ? truncated
                ? t.settings.logs.showingLast.replace(
                    "{count}",
                    String(DISPLAY_LINE_CAP)
                  )
                : t.settings.logs.lineCount.replace(
                    "{count}",
                    String(lines.length)
                  )
              : t.settings.logs.empty}
          </span>
          <div className="flex items-center gap-1.5">
            <Button
              variant="ghost"
              size="sm"
              className="h-7 gap-1.5 px-2 text-xs"
              disabled={!content}
              onClick={() => void handleCopy()}
            >
              {copied ? (
                <CheckIcon className="size-3" />
              ) : (
                <CopyIcon className="size-3" />
              )}
              {copied ? t.common.copied : t.settings.logs.copy}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              className="h-7 gap-1.5 px-2 text-xs text-destructive hover:text-destructive"
              disabled={!selected}
              onClick={() => void handleClear()}
            >
              <Trash2Icon className="size-3" />
              {t.settings.logs.clear}
            </Button>
          </div>
        </div>
        <ScrollArea
          viewportRef={viewportRef}
          onScroll={handleScroll}
          className="h-[420px] bg-muted/30"
        >
          <div className="p-3">
            {content ? (
              <div className="font-mono text-xs leading-relaxed whitespace-pre-wrap break-all">
                {truncated && (
                  <div className="mb-2 text-xs text-muted-foreground italic">
                    {t.settings.logs.truncated.replace(
                      "{total}",
                      String(lines.length)
                    )}
                  </div>
                )}
                {displayLines.map((line, i) => {
                  const parts = parseLogLine(line);
                  if (!parts) {
                    // 无法解析的行（如续行/空行）用次要色显示
                    return (
                      <div key={i} className="text-muted-foreground/80">
                        {line || "\u00A0"}
                      </div>
                    );
                  }
                  return (
                    <div key={i}>
                      <span className="text-muted-foreground/60">
                        {parts.timestamp}
                      </span>
                      {"  "}
                      <span className={`font-semibold ${LEVEL_COLORS[parts.level] ?? "text-foreground"}`}>
                        {parts.level}
                      </span>
                      {" "}
                      <span className="text-[var(--status-success-default, rgb(34 197 94))]">
                        {parts.target}:
                      </span>
                      {" "}
                      <span className="text-foreground">
                        {parts.message || "\u00A0"}
                      </span>
                    </div>
                  );
                })}
              </div>
            ) : (
              <div className="flex h-[380px] items-center justify-center text-muted-foreground">
                <div className="flex flex-col items-center gap-2">
                  <ScrollTextIcon className="size-8 opacity-40" />
                  <span className="text-xs">
                    {files.length === 0
                      ? t.settings.logs.noFiles
                      : t.settings.logs.empty}
                  </span>
                </div>
              </div>
            )}
          </div>
        </ScrollArea>
      </SettingsSection>
    </PageContainer>
  );
}
