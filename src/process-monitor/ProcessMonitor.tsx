/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 进程监控器 - 显示 Mnemosyne 相关进程的实时监控数据
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useEffect, useState, useCallback } from "react";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";
import { getProcessList } from "./services";
import type { ProcessInfo, ProcessType } from "./types";
import { useI18n } from "@/locales/i18n";

// ── 工具函数 ────────────────────────────────────────────────────────────────

/** 格式化内存显示 */
function formatMemory(mb: number): string {
  if (mb < 1) {
    return `${(mb * 1024).toFixed(0)} KB`;
  }
  if (mb >= 1024) {
    return `${(mb / 1024).toFixed(1)} GB`;
  }
  return `${mb.toFixed(1)} MB`;
}

/** 格式化 CPU 显示 */
function formatCpu(cpu: number): string {
  return `${cpu.toFixed(1)}%`;
}

// ── 组件实现 ────────────────────────────────────────────────────────────────

export function ProcessMonitor() {
  const { t } = useI18n();
  const [processes, setProcesses] = useState<ProcessInfo[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  /** 进程类型显示名称（i18n） */
  const processTypeLabels: Record<ProcessType, string> = {
    main: t.processMonitor.types.main,
    renderer: t.processMonitor.types.renderer,
    gpu: t.processMonitor.types.gpu,
    utility: t.processMonitor.types.utility,
    unknown: t.processMonitor.types.unknown,
  };

  /** 进程类型 Badge 变体 */
  const processTypeVariants: Record<ProcessType, "default" | "secondary" | "destructive" | "outline"> = {
    main: "default",
    renderer: "secondary",
    gpu: "outline",
    utility: "outline",
    unknown: "outline",
  };

  const fetchProcesses = useCallback(async () => {
    try {
      const data = await getProcessList();
      setProcesses(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : t.processMonitor.fetchError);
    } finally {
      setIsLoading(false);
    }
  }, [t.processMonitor.fetchError]);

  useEffect(() => {
    fetchProcesses();
    const interval = setInterval(fetchProcesses, 1000);
    return () => clearInterval(interval);
  }, [fetchProcesses]);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-screen bg-background text-xs text-muted-foreground">
        {t.processMonitor.loading}
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-screen bg-background text-xs text-destructive">
        {error}
      </div>
    );
  }

  return (
    <div className="h-screen bg-background flex flex-col overflow-hidden">
      {/* 头部区域 */}
      <header className="flex-shrink-0 px-2 py-1.5 border-b flex items-center gap-2">
        <h1 className="text-xs font-medium">{t.processMonitor.title}</h1>
        <span className="text-[10px] text-muted-foreground">
          {t.processMonitor.processCount.replace("{count}", String(processes.length))}
        </span>
      </header>

      {/* 进程表格 */}
      <ScrollArea className="flex-1">
        <Table>
          <TableHeader className="sticky top-0 bg-background z-10">
            <TableRow className="hover:bg-transparent">
              <TableHead className="h-7 px-2 text-[10px] font-medium text-muted-foreground">
                {t.processMonitor.columns.process}
              </TableHead>
              <TableHead className="h-7 px-2 w-[70px] text-[10px] font-medium text-muted-foreground">
                {t.processMonitor.columns.pid}
              </TableHead>
              <TableHead className="h-7 px-2 w-[60px] text-[10px] font-medium text-muted-foreground text-right">
                {t.processMonitor.columns.cpu}
              </TableHead>
              <TableHead className="h-7 px-2 w-[80px] text-[10px] font-medium text-muted-foreground text-right">
                {t.processMonitor.columns.memory}
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {processes.map((process) => (
              <TableRow
                key={process.pid}
                className="hover:bg-muted/30 data-[state=selected]:bg-muted/50"
              >
                <TableCell className="py-1 px-2">
                  <div className="flex items-center gap-2">
                    <span className="font-mono text-[11px] truncate max-w-[180px]">
                      {process.name}
                    </span>
                    <Badge variant={processTypeVariants[process.processType]} className="text-[9px] px-1 py-0 h-auto">
                      {processTypeLabels[process.processType]}
                    </Badge>
                  </div>
                </TableCell>
                <TableCell className="py-1 px-2">
                  <span className="font-mono text-[11px] text-muted-foreground">
                    {process.pid}
                  </span>
                </TableCell>
                <TableCell className="py-1 px-2 text-right">
                  <span
                    className={`font-mono text-[11px] ${
                      process.cpuUsage > 50
                        ? "text-red-500 font-medium"
                        : process.cpuUsage > 20
                          ? "text-amber-500"
                          : "text-foreground"
                    }`}
                  >
                    {formatCpu(process.cpuUsage)}
                  </span>
                </TableCell>
                <TableCell className="py-1 px-2 text-right">
                  <span className="font-mono text-[11px] text-foreground">
                    {formatMemory(process.memoryMb)}
                  </span>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>

        {processes.length === 0 && (
          <div className="text-center py-4 text-xs text-muted-foreground">
            {t.processMonitor.noProcesses}
          </div>
        )}
      </ScrollArea>
    </div>
  );
}