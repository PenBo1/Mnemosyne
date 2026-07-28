/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 日志查看器 - 实时滚动显示应用日志
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useEffect, useState, useCallback, useRef } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { getLogFiles, readLogFile } from "./services";

export function LogViewer() {
  const [logFiles, setLogFiles] = useState<string[]>([]);
  const [selectedFile, setSelectedFile] = useState<string>("");
  const [content, setContent] = useState<string>("");
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [autoScroll, setAutoScroll] = useState(true);
  const scrollRef = useRef<HTMLDivElement>(null);

  // ── 加载日志文件列表 ────────────────────────────────────────────────────────────────
  const loadLogFiles = useCallback(async () => {
    try {
      const files = await getLogFiles();
      setLogFiles(files.map((f) => f.name));
      if (!selectedFile && files.length > 0) {
        setSelectedFile(files[0].name);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "获取日志文件列表失败");
    }
  }, [selectedFile]);

  // ── 加载日志内容 ────────────────────────────────────────────────────────────────
  const loadLogContent = useCallback(async () => {
    if (!selectedFile) return;
    try {
      const data = await readLogFile(selectedFile);
      setContent(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "读取日志失败");
    } finally {
      setIsLoading(false);
    }
  }, [selectedFile]);

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

  // ── 解析日志级别 ────────────────────────────────────────────────────────────────
  const getLogLevel = (line: string): string => {
    if (line.includes(" ERROR ") || line.includes("ERROR]")) return "error";
    if (line.includes(" WARN ") || line.includes("WARN]")) return "warn";
    if (line.includes(" INFO ") || line.includes("INFO]")) return "info";
    if (line.includes(" DEBUG ") || line.includes("DEBUG]")) return "debug";
    return "";
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-screen bg-background text-xs text-muted-foreground">
        加载中...
      </div>
    );
  }

  return (
    <div className="h-screen bg-background flex flex-col overflow-hidden">
      {/* 头部区域 */}
      <header className="flex-shrink-0 px-2 py-1.5 border-b flex items-center gap-3">
        <h1 className="text-xs font-medium">日志查看</h1>

        {/* 文件选择 */}
        <Select value={selectedFile} onValueChange={setSelectedFile}>
          <SelectTrigger size="sm" className="w-[180px]">
            <SelectValue placeholder="选择日志文件" />
          </SelectTrigger>
          <SelectContent>
            {logFiles.map((file) => (
              <SelectItem key={file} value={file}>
                {file}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        {/* 自动滚动开关 */}
        <Label className="flex items-center gap-2 text-[10px] text-muted-foreground cursor-pointer">
          <Switch
            size="sm"
            checked={autoScroll}
            onCheckedChange={setAutoScroll}
          />
          自动滚动
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
          {content.split("\n").map((line, i) => (
            <div
              key={i}
              className={`log-line whitespace-pre-wrap ${
                getLogLevel(line) === "error"
                  ? "text-red-500"
                  : getLogLevel(line) === "warn"
                    ? "text-amber-500"
                    : getLogLevel(line) === "info"
                      ? "text-blue-500"
                      : getLogLevel(line) === "debug"
                        ? "text-muted-foreground"
                        : "text-foreground"
              }`}
            >
              {line}
            </div>
          ))}
          {content === "" && (
            <div className="text-muted-foreground">暂无日志内容</div>
          )}
        </div>
      </ScrollArea>
    </div>
  );
}