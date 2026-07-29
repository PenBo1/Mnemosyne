/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 日志查看器 - 实时滚动显示应用日志，支持搜索和等级过滤
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useEffect, useState, useCallback, useRef, useMemo } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { getLogFiles, readLogFile } from "./services";
import { useI18n } from "@/locales/i18n";

// ── 日志等级类型 ────────────────────────────────────────────────────────────────

/** 日志等级（从日志行解析） */
type LogLevel = "error" | "warn" | "info" | "debug" | "trace";

/** 日志过滤器等级（包含 "all" 选项） */
type LogLevelFilter = "all" | LogLevel;

/** 日志等级显示颜色（使用语义化颜色，支持浅色主题） */
const LOG_LEVEL_COLORS: Record<LogLevel, string> = {
  error: "text-red-600 dark:text-red-400 font-semibold",
  warn: "text-amber-600 dark:text-amber-400",
  info: "text-sky-600 dark:text-sky-400",
  debug: "text-emerald-700 dark:text-emerald-400",
  trace: "text-muted-foreground",
};

// ── 日志行解析 ────────────────────────────────────────────────────────────────

interface ParsedLogLine {
  raw: string;
  timestamp: string;
  level: LogLevel | null; // null 表示未解析成功，不显示等级
  location: string;
  message: string;
}

/**
 * 解析日志行格式：2026-07-28T08:30:34.749837Z  INFO mnemosyne_lib: message
 */
function parseLogLine(line: string): ParsedLogLine {
  // 默认值（未解析成功的行）
  const defaultResult: ParsedLogLine = {
    raw: line,
    timestamp: "",
    level: null,
    location: "",
    message: line,
  };

  if (!line.trim()) {
    return defaultResult;
  }

  // 匹配格式：timestamp LEVEL module: message
  const logPattern = /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z?)\s+(ERROR|WARN|INFO|DEBUG|TRACE)\s+([^\s:]+(?:\s*::\s*[^\s:]+)*)\s*:\s*(.*)$/;
  const match = line.match(logPattern);

  if (match) {
    const [, timestamp, level, location, message] = match;
    return {
      raw: line,
      timestamp,
      level: level.toLowerCase() as LogLevel,
      location,
      message,
    };
  }

  // 尝试匹配更简单的格式：timestamp LEVEL message
  const simplePattern = /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z?)\s+(ERROR|WARN|INFO|DEBUG|TRACE)\s+(.*)$/;
  const simpleMatch = line.match(simplePattern);

  if (simpleMatch) {
    const [, timestamp, level, message] = simpleMatch;
    return {
      raw: line,
      timestamp,
      level: level.toLowerCase() as LogLevel,
      location: "",
      message,
    };
  }

  return defaultResult;
}

// ── 日志等级选择组件 ────────────────────────────────────────────────────────────────

interface LogLevelSelectProps {
  value: LogLevelFilter;
  onChange: (value: LogLevelFilter) => void;
  levelLabel: string;
  allLabel: string;
}

function LogLevelSelect({ value, onChange, levelLabel, allLabel }: LogLevelSelectProps) {
  return (
    <Select value={value} onValueChange={(v) => onChange(v as LogLevelFilter)}>
      <SelectTrigger size="sm" className="w-[100px]">
        <SelectValue placeholder={levelLabel} />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="all">{allLabel}</SelectItem>
        <SelectItem value="error">ERROR</SelectItem>
        <SelectItem value="warn">WARN</SelectItem>
        <SelectItem value="info">INFO</SelectItem>
        <SelectItem value="debug">DEBUG</SelectItem>
        <SelectItem value="trace">TRACE</SelectItem>
      </SelectContent>
    </Select>
  );
}

// ── 主组件 ────────────────────────────────────────────────────────────────

export function LogViewer() {
  const { t } = useI18n();
  const [logFiles, setLogFiles] = useState<string[]>([]);
  const [selectedFile, setSelectedFile] = useState<string>("");
  const [content, setContent] = useState<string>("");
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [autoScroll, setAutoScroll] = useState(true);
  const [searchQuery, setSearchQuery] = useState("");
  const [levelFilter, setLevelFilter] = useState<LogLevelFilter>("all");
  const scrollRef = useRef<HTMLDivElement>(null);

  // ── 加载日志文件列表 ────────────────────────────────────────────────────────────────
  const loadLogFiles = useCallback(async () => {
    try {
      const files = await getLogFiles();
      setLogFiles(files.map((f) => f.name));
      if (!selectedFile && files.length > 0) {
        setSelectedFile(files[0].name);
      }
      if (files.length === 0) {
        setIsLoading(false);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : t.logViewer.fetchError);
      setIsLoading(false);
    }
  }, [selectedFile, t.logViewer.fetchError]);

  // ── 加载日志内容 ────────────────────────────────────────────────────────────────
  const loadLogContent = useCallback(async () => {
    if (!selectedFile) return;
    try {
      const data = await readLogFile(selectedFile);
      setContent(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : t.logViewer.fetchError);
    } finally {
      setIsLoading(false);
    }
  }, [selectedFile, t.logViewer.fetchError]);

  // ── 初始化加载 ────────────────────────────────────────────────────────────────
  useEffect(() => {
    loadLogFiles();
  }, [loadLogFiles]);

  // ── 定时刷新日志内容 ────────────────────────────────────────────────────────────────
  useEffect(() => {
    loadLogContent();
    const interval = setInterval(loadLogContent, 1000);
    return () => clearInterval(interval);
  }, [loadLogContent]);

  // ── 自动滚动到底部 ────────────────────────────────────────────────────────────────
  useEffect(() => {
    if (autoScroll && scrollRef.current) {
      const scrollContainer = scrollRef.current.querySelector('[data-radix-scroll-area-viewport]');
      if (scrollContainer) {
        scrollContainer.scrollTop = scrollContainer.scrollHeight;
      }
    }
  }, [content, autoScroll]);

  // ── 解析并过滤日志 ────────────────────────────────────────────────────────────────
  const filteredLines = useMemo(() => {
    const lines = content.split("\n");
    const parsed = lines.map(parseLogLine);

    return parsed.filter((line) => {
      // 等级过滤（未解析的行只显示在 "全部" 模式）
      if (levelFilter !== "all") {
        if (line.level === null || line.level !== levelFilter) {
          return false;
        }
      }
      // 搜索过滤
      if (searchQuery && !line.raw.toLowerCase().includes(searchQuery.toLowerCase())) {
        return false;
      }
      return true;
    });
  }, [content, levelFilter, searchQuery]);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-screen bg-background text-xs text-muted-foreground">
        {t.logViewer.loading}
      </div>
    );
  }

  return (
    <div className="h-screen bg-background flex flex-col overflow-hidden">
      {/* 头部区域 */}
      <header className="flex-shrink-0 px-2 py-1.5 border-b flex items-center gap-2 flex-wrap">
        <h1 className="text-xs font-medium">{t.logViewer.title}</h1>

        {/* 文件选择 */}
        <Select value={selectedFile} onValueChange={setSelectedFile}>
          <SelectTrigger size="sm" className="w-[160px]">
            <SelectValue placeholder={t.logViewer.selectFile} />
          </SelectTrigger>
          <SelectContent>
            {logFiles.map((file) => (
              <SelectItem key={file} value={file}>
                {file}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        {/* 日志等级选择 */}
        <LogLevelSelect
          value={levelFilter}
          onChange={setLevelFilter}
          levelLabel={t.logViewer.level}
          allLabel={t.logViewer.all}
        />

        {/* 搜索框 */}
        <Input
          type="text"
          placeholder={t.logViewer.searchPlaceholder}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="h-6 w-[140px] text-[10px] px-2"
        />

        {/* 自动滚动开关 */}
        <Label className="flex items-center gap-1.5 text-[10px] text-muted-foreground cursor-pointer ml-auto">
          <Switch
            size="sm"
            checked={autoScroll}
            onCheckedChange={setAutoScroll}
          />
          {t.logViewer.autoScroll}
        </Label>
      </header>

      {/* 错误提示 */}
      {error && (
        <div className="flex-shrink-0 px-2 py-1 bg-destructive/10 text-destructive text-[10px]">
          {error}
        </div>
      )}

      {/* 日志内容 */}
      <ScrollArea ref={scrollRef} className="flex-1">
        <div className="p-2 font-mono text-[11px] leading-relaxed">
          {filteredLines.length === 0 && content && (
            <div className="text-muted-foreground">{t.logViewer.noMatch}</div>
          )}
          {content === "" && (
            <div className="text-muted-foreground">{t.logViewer.noContent}</div>
          )}
          {filteredLines.map((line, i) => (
            <div key={i} className="whitespace-nowrap hover:bg-muted/30 px-1 -mx-1">
              {/* 时间戳 */}
              {line.timestamp && (
                <span className="text-muted-foreground">{line.timestamp} </span>
              )}
              {/* 日志等级（右对齐 + 固定宽度，null 时不显示等级） */}
              {line.level !== null ? (
                <span className={`inline-block w-[5ch] text-right ${LOG_LEVEL_COLORS[line.level]}`}>
                  {line.level.toUpperCase()}
                </span>
              ) : (
                <span className="inline-block w-[5ch] text-right text-muted-foreground/50">---</span>
              )}
              {/* 空格（与原始日志格式一致） */}
              <span> </span>
              {/* 代码位置 */}
              {line.location && (
                <span className="text-muted-foreground">{line.location}: </span>
              )}
              {/* 日志消息（使用主题感知颜色） */}
              <span className="text-foreground/80 dark:text-foreground/70">{line.message}</span>
            </div>
          ))}
        </div>
      </ScrollArea>
    </div>
  );
}