/**
 * ═══════════════════════════════════════════════════════════════════════════
 * MemoryPanel - Agent 记忆管理面板组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useState, useCallback } from "react";
import { Database, Search, Plus, RefreshCw, FileText, Hash } from "lucide-react";
import { toast } from "sonner";
import { useI18n } from "@/locales/i18n";
import { Card, CardHeader, CardTitle, CardContent, CardDescription } from "@/components/ui/card";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
import { Spinner } from "@/components/ui/spinner";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  appendCoreMemory,
  replaceCoreMemory,
} from "@/features/agent/services/memory/core-memory";
import {
  searchRecallMemory,
  type RecallMemorySearchResult,
} from "@/features/agent/services/memory/recall-memory";
import {
  searchArchivalMemory,
  type ArchivalMemorySearchResult,
} from "@/features/agent/services/memory/archival-memory";

// ── 类型定义 ────────────────────────────────────────────────────────────────

type MemoryTab = "core" | "recall" | "archival";

interface MemoryPanelProps {
  open: boolean;
  sessionId: string | null;
  onClose: () => void;
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 记忆管理面板，支持核心记忆编辑、回忆记忆搜索和归档记忆搜索
 */
export function MemoryPanel({ open, sessionId, onClose }: MemoryPanelProps) {
  const { t } = useI18n();
  const [activeTab, setActiveTab] = useState<MemoryTab>("core");
  const [loading, setLoading] = useState(false);

  // 核心记忆状态
  const [coreContent, setCoreContent] = useState("");
  const [coreSection, setCoreSection] = useState("");
  const [coreMode, setCoreMode] = useState<"append" | "replace">("append");

  // 回忆记忆状态
  const [recallQuery, setRecallQuery] = useState("");
  const [recallResults, setRecallResults] = useState<RecallMemorySearchResult[]>([]);

  // 归档记忆状态
  const [archivalQuery, setArchivalQuery] = useState("");
  const [archivalResults, setArchivalResults] = useState<ArchivalMemorySearchResult[]>([]);

  // ── 核心记忆操作 ──────────────────────────────────────────────────────────

  /**
   * 处理核心记忆操作（追加或替换）
   */
  const handleCoreMemoryAction = useCallback(async () => {
    if (!coreContent.trim()) return;
    setLoading(true);
    try {
      if (coreMode === "append") {
        await appendCoreMemory(sessionId || "", coreContent, coreSection || undefined);
      } else {
        await replaceCoreMemory(sessionId || "", coreContent, coreSection || undefined);
      }
      setCoreContent("");
      setCoreSection("");
    } catch (error) {
      console.error("[MemoryPanel] core memory action failed:", error);
      toast.error(t.memoryPanel.errors.coreActionFailed);
    } finally {
      setLoading(false);
    }
  }, [coreContent, coreMode, coreSection, sessionId]);

  // ── 回忆记忆搜索 ──────────────────────────────────────────────────────────

  /**
   * 搜索回忆记忆
   */
  const handleRecallSearch = useCallback(async () => {
    if (!recallQuery.trim()) return;
    setLoading(true);
    try {
      const results = await searchRecallMemory(sessionId || "", recallQuery);
      setRecallResults(results);
    } catch (error) {
      console.error("[MemoryPanel] recall search failed:", error);
      setRecallResults([]);
      toast.error(t.memoryPanel.errors.recallSearchFailed);
    } finally {
      setLoading(false);
    }
  }, [recallQuery, sessionId]);

  // ── 归档记忆搜索 ──────────────────────────────────────────────────────────

  /**
   * 搜索归档记忆
   */
  const handleArchivalSearch = useCallback(async () => {
    if (!archivalQuery.trim()) return;
    setLoading(true);
    try {
      const results = await searchArchivalMemory(sessionId || "", archivalQuery);
      setArchivalResults(results);
    } catch (error) {
      console.error("[MemoryPanel] archival search failed:", error);
      setArchivalResults([]);
      toast.error(t.memoryPanel.errors.archivalSearchFailed);
    } finally {
      setLoading(false);
    }
  }, [archivalQuery, sessionId]);

  // ── 渲染 ──────────────────────────────────────────────────────────────────

  if (!open) return null;

  return (
    <aside className="flex w-80 shrink-0 flex-col border-l border-border bg-background">
      <div className="flex h-12 shrink-0 items-center justify-between border-b border-border px-3">
        <h2 className="text-sm font-medium text-foreground flex items-center gap-1.5">
          <Database className="size-3.5" />
          {t.memory.title}
        </h2>
        <Button variant="ghost" size="icon-xs" onClick={onClose} className="text-muted-foreground">
          ×
        </Button>
      </div>

      <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as MemoryTab)} className="flex-1 flex flex-col">
        <div className="shrink-0 border-b border-border px-2">
          <TabsList className="w-full">
            <TabsTrigger value="core" className="flex-1 text-xs">
              {t.memoryPanel.core}
            </TabsTrigger>
            <TabsTrigger value="recall" className="flex-1 text-xs">
              {t.memoryPanel.recall}
            </TabsTrigger>
            <TabsTrigger value="archival" className="flex-1 text-xs">
              {t.memoryPanel.archival}
            </TabsTrigger>
          </TabsList>
        </div>

        <ScrollArea className="flex-1">
          <TabsContent value="core" className="p-3">
            <Card size="sm">
              <CardHeader>
                <CardTitle className="flex items-center gap-1.5">
                  <FileText className="size-3" />
                  {t.memoryPanel.coreEditor}
                </CardTitle>
                <CardDescription>
                  {t.memoryPanel.coreEditorDesc}
                </CardDescription>
              </CardHeader>
              <CardContent className="flex flex-col gap-3">
                <div className="flex gap-1">
                  <Button
                    size="sm"
                    variant={coreMode === "append" ? "default" : "outline"}
                    onClick={() => setCoreMode("append")}
                    className="flex-1"
                  >
                    <Plus className="size-3" data-icon="inline-start" />
                    {t.memoryPanel.append}
                  </Button>
                  <Button
                    size="sm"
                    variant={coreMode === "replace" ? "default" : "outline"}
                    onClick={() => setCoreMode("replace")}
                    className="flex-1"
                  >
                    <RefreshCw className="size-3" data-icon="inline-start" />
                    {t.memoryPanel.replace}
                  </Button>
                </div>

                <div>
                  <label className="text-xs text-muted-foreground mb-1 block">
                    {t.memoryPanel.section}
                  </label>
                  <Input
                    value={coreSection}
                    onChange={(e) => setCoreSection(e.target.value)}
                    placeholder={t.memoryPanel.sectionPlaceholder}
                    className="text-xs"
                  />
                </div>

                <div>
                  <label className="text-xs text-muted-foreground mb-1 block">
                    {t.memoryPanel.content}
                  </label>
                  <Textarea
                    value={coreContent}
                    onChange={(e) => setCoreContent(e.target.value)}
                    placeholder={t.memoryPanel.contentPlaceholder}
                    rows={5}
                    className="text-xs resize-none"
                  />
                </div>

                <Button
                  size="sm"
                  onClick={handleCoreMemoryAction}
                  disabled={loading || !coreContent.trim()}
                  className="w-full"
                >
                  {loading ? <Spinner className="size-3" /> : t.memoryPanel.submit}
                </Button>
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="recall" className="p-3">
            <Card size="sm">
              <CardHeader>
                <CardTitle className="flex items-center gap-1.5">
                  <Search className="size-3" />
                  {t.memoryPanel.recallSearch}
                </CardTitle>
                <CardDescription>
                  {t.memoryPanel.recallSearchDesc}
                </CardDescription>
              </CardHeader>
              <CardContent className="flex flex-col gap-3">
                <div className="flex gap-2">
                  <Input
                    value={recallQuery}
                    onChange={(e) => setRecallQuery(e.target.value)}
                    placeholder={t.memoryPanel.searchPlaceholder}
                    className="text-xs flex-1"
                    onKeyDown={(e) => e.key === "Enter" && handleRecallSearch()}
                  />
                  <Button
                    size="icon-sm"
                    onClick={handleRecallSearch}
                    disabled={loading || !recallQuery.trim()}
                  >
                    {loading ? <Spinner className="size-3" /> : <Search className="size-3" />}
                  </Button>
                </div>

                {recallResults.length > 0 && (
                  <div className="flex flex-col gap-2">
                    {recallResults.map((result) => (
                      <div
                        key={result.id}
                        className="rounded border border-border p-2 bg-muted/30"
                      >
                        <div className="flex items-center gap-1.5 mb-1">
                          <Badge variant="outline" className="text-[10px]">
                            {result.role}
                          </Badge>
                          <span className="text-[10px] text-muted-foreground">
                            {new Date(result.createdAt).toLocaleString()}
                          </span>
                        </div>
                        <p className="text-xs text-foreground line-clamp-3">
                          {result.content}
                        </p>
                      </div>
                    ))}
                  </div>
                )}

                {recallResults.length === 0 && recallQuery && !loading && (
                  <p className="text-xs text-muted-foreground text-center py-4">
                    {t.memoryPanel.noResults}
                  </p>
                )}
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="archival" className="p-3">
            <Card size="sm">
              <CardHeader>
                <CardTitle className="flex items-center gap-1.5">
                  <Hash className="size-3" />
                  {t.memoryPanel.archivalSearch}
                </CardTitle>
                <CardDescription>
                  {t.memoryPanel.archivalSearchDesc}
                </CardDescription>
              </CardHeader>
              <CardContent className="flex flex-col gap-3">
                <div className="flex gap-2">
                  <Input
                    value={archivalQuery}
                    onChange={(e) => setArchivalQuery(e.target.value)}
                    placeholder={t.memoryPanel.searchPlaceholder}
                    className="text-xs flex-1"
                    onKeyDown={(e) => e.key === "Enter" && handleArchivalSearch()}
                  />
                  <Button
                    size="icon-sm"
                    onClick={handleArchivalSearch}
                    disabled={loading || !archivalQuery.trim()}
                  >
                    {loading ? <Spinner className="size-3" /> : <Search className="size-3" />}
                  </Button>
                </div>

                {archivalResults.length > 0 && (
                  <div className="flex flex-col gap-2">
                    {archivalResults.map((result) => (
                      <div
                        key={result.id}
                        className="rounded border border-border p-2 bg-muted/30"
                      >
                        <div className="flex items-center justify-between mb-1">
                          <Badge variant="info" className="text-[10px]">
                            {(result.similarity * 100).toFixed(1)}% {t.memoryPanel.similar}
                          </Badge>
                          <span className="text-[10px] text-muted-foreground">
                            {new Date(result.createdAt).toLocaleDateString()}
                          </span>
                        </div>
                        <p className="text-xs text-foreground line-clamp-3">
                          {result.content}
                        </p>
                      </div>
                    ))}
                  </div>
                )}

                {archivalResults.length === 0 && archivalQuery && !loading && (
                  <p className="text-xs text-muted-foreground text-center py-4">
                    {t.memoryPanel.noResults}
                  </p>
                )}
              </CardContent>
            </Card>
          </TabsContent>
        </ScrollArea>
      </Tabs>
    </aside>
  );
}