/**
 * ═══════════════════════════════════════════════════════════════════════════
 * LoopPanel - 循环任务管理面板组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useState, useCallback, useEffect } from "react";
import { Repeat2, Plus } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import type { PanelLoopState } from "@/features/loop/types";
import {
  listLoops,
  createLoop,
  updateLoop,
  stopLoop,
  deleteLoop,
} from "@/features/agent/services/loop";
import { LoopFormFields, DEFAULT_FORM_DATA } from "./loop-form-fields";
import type { LoopFormData } from "./loop-form-fields";
import { LoopCard } from "./loop-card";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface LoopPanelProps {
  open: boolean;
  sessionId: string | null;
  onClose: () => void;
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 循环任务管理面板，支持创建、编辑、停止和删除循环任务
 */
export function LoopPanel({ open, sessionId, onClose }: LoopPanelProps) {
  const { t } = useI18n();
  const [loading, setLoading] = useState(false);
  const [loops, setLoops] = useState<PanelLoopState[]>([]);
  const [createDialogOpen, setCreateDialogOpen] = useState(false);
  const [editLoop, setEditLoop] = useState<PanelLoopState | null>(null);

  const [formData, setFormData] = useState<LoopFormData>(DEFAULT_FORM_DATA);

  // ── 数据加载 ──────────────────────────────────────────────────────────────

  /**
   * 加载循环任务列表
   */
  const loadLoops = useCallback(async () => {
    if (!sessionId) return;
    setLoading(true);
    try {
      const result = await listLoops(sessionId);
      setLoops(result);
    } catch (error) {
      console.error("Failed to load loops:", error);
      setLoops([]);
    } finally {
      setLoading(false);
    }
  }, [sessionId]);

  useEffect(() => {
    if (open && sessionId) {
      loadLoops();
    }
  }, [open, sessionId, loadLoops]);

  // ── 事件处理 ──────────────────────────────────────────────────────────────

  /**
   * 创建新的循环任务
   */
  const handleCreateLoop = useCallback(async () => {
    if (!formData.name.trim() || !sessionId) return;
    setLoading(true);
    try {
      await createLoop({
        sessionId,
        name: formData.name,
        triggerType: formData.triggerType,
        triggerConfig: formData.triggerType === "timer"
          ? { intervalMs: formData.intervalMs }
          : { eventType: formData.eventType, eventFilter: formData.eventFilter },
        maxIterations: formData.maxIterations,
      });
      setCreateDialogOpen(false);
      setFormData(DEFAULT_FORM_DATA);
      await loadLoops();
    } catch (error) {
      console.error("Failed to create loop:", error);
    } finally {
      setLoading(false);
    }
  }, [formData, sessionId, loadLoops]);

  /**
   * 更新循环任务
   */
  const handleUpdateLoop = useCallback(async () => {
    if (!editLoop || !sessionId) return;
    setLoading(true);
    try {
      await updateLoop({
        sessionId,
        loopId: editLoop.id,
        name: formData.name,
        triggerType: formData.triggerType,
        triggerConfig: formData.triggerType === "timer"
          ? { intervalMs: formData.intervalMs }
          : { eventType: formData.eventType, eventFilter: formData.eventFilter },
        maxIterations: formData.maxIterations,
      });
      setEditLoop(null);
      setFormData(DEFAULT_FORM_DATA);
      await loadLoops();
    } catch (error) {
      console.error("Failed to update loop:", error);
    } finally {
      setLoading(false);
    }
  }, [editLoop, formData, sessionId, loadLoops]);

  /**
   * 停止循环任务
   */
  const handleStopLoop = useCallback(async (loopId: string) => {
    if (!sessionId) return;
    setLoading(true);
    try {
      await stopLoop(sessionId, loopId);
      await loadLoops();
    } catch (error) {
      console.error("Failed to stop loop:", error);
    } finally {
      setLoading(false);
    }
  }, [sessionId, loadLoops]);

  /**
   * 删除循环任务
   */
  const handleDeleteLoop = useCallback(async (loopId: string) => {
    if (!sessionId) return;
    setLoading(true);
    try {
      await deleteLoop(sessionId, loopId);
      await loadLoops();
    } catch (error) {
      console.error("Failed to delete loop:", error);
    } finally {
      setLoading(false);
    }
  }, [sessionId, loadLoops]);

  /**
   * 点击编辑按钮
   */
  const handleEditClick = useCallback((loop: PanelLoopState) => {
    setEditLoop(loop);
    setFormData({
      name: loop.name,
      triggerType: loop.triggerType,
      intervalMs: loop.triggerConfig.intervalMs || 60000,
      eventType: loop.triggerConfig.eventType || "",
      eventFilter: loop.triggerConfig.eventFilter || "",
      maxIterations: loop.maxIterations || 10,
    });
  }, []);

  /**
   * 表单数据变更
   */
  const handleFormDataChange = useCallback((partial: Partial<LoopFormData>) => {
    setFormData((prev) => ({ ...prev, ...partial }));
  }, []);

  // ── 渲染 ──────────────────────────────────────────────────────────────────

  if (!open) return null;

  return (
    <aside className="flex w-80 shrink-0 flex-col border-l border-border bg-background">
      <div className="flex h-12 shrink-0 items-center justify-between border-b border-border px-3">
        <h2 className="text-sm font-medium text-foreground flex items-center gap-1.5">
          <Repeat2 className="size-3.5" />
          {t.loop.title}
        </h2>
        <Button variant="ghost" size="icon-xs" onClick={onClose} className="text-muted-foreground">
          ×
        </Button>
      </div>

      <div className="shrink-0 border-b border-border p-2">
        <Button
          size="sm"
          onClick={() => setCreateDialogOpen(true)}
          className="w-full"
        >
          <Plus className="size-3" data-icon="inline-start" />
          {t.loop.newLoop}
        </Button>
      </div>

      <ScrollArea className="flex-1">
        <div className="p-2 flex flex-col gap-2">
          {loading && loops.length === 0 && (
            <div className="flex items-center justify-center py-8">
              <Spinner className="size-4" />
            </div>
          )}

          {!loading && loops.length === 0 && (
            <p className="text-xs text-muted-foreground text-center py-8">
              {t.loop.common.noLoops}
            </p>
          )}

          {loops.map((loop) => (
            <LoopCard
              key={loop.id}
              loop={loop}
              onEdit={handleEditClick}
              onStop={handleStopLoop}
              onDelete={handleDeleteLoop}
            />
          ))}
        </div>
      </ScrollArea>

      <Dialog open={createDialogOpen} onOpenChange={setCreateDialogOpen}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>{t.loop.newLoop}</DialogTitle>
            <DialogDescription>
              {t.loopPanel.createDesc}
            </DialogDescription>
          </DialogHeader>
          <LoopFormFields formData={formData} onChange={handleFormDataChange} />
          <DialogFooter>
            <Button variant="outline" size="sm" onClick={() => setCreateDialogOpen(false)}>
              {t.common.cancel}
            </Button>
            <Button size="sm" onClick={handleCreateLoop} disabled={loading || !formData.name.trim()}>
              {loading ? <Spinner className="size-3" /> : t.common.create}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={!!editLoop} onOpenChange={(v) => !v && setEditLoop(null)}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>{t.loop.editLoop}</DialogTitle>
            <DialogDescription>
              {t.loopPanel.editDesc}
            </DialogDescription>
          </DialogHeader>
          <LoopFormFields formData={formData} onChange={handleFormDataChange} />
          <DialogFooter>
            <Button variant="outline" size="sm" onClick={() => setEditLoop(null)}>
              {t.common.cancel}
            </Button>
            <Button size="sm" onClick={handleUpdateLoop} disabled={loading || !formData.name.trim()}>
              {loading ? <Spinner className="size-3" /> : t.common.save}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </aside>
  );
}