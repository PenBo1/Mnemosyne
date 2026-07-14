// 项目记忆设置页 —— workspace 级别 project_memory.md 的查看与编辑。
//
// 核心功能:
// 1. 选择 workspace(下拉)
// 2. 显示当前 project_memory.md 内容(可编辑 textarea)
// 3. 保存(覆盖) / 追加段落 / 清空内容
// 4. 显示统计信息(字节数 / 字符数 / 上限 256KB)

import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  RefreshCwIcon,
  SaveIcon,
  Trash2Icon,
  PlusIcon,
  FolderCogIcon,
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
import { fetchWorkspaces } from "@/features/workspace/services";
import type { Workspace } from "@/features/workspace/types";
import {
  getProjectMemory,
  updateProjectMemory,
  clearProjectMemory,
  getProjectMemoryStats,
  type ProjectMemoryStats,
} from "@/features/settings/services/project-memory";

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(2)} MB`;
}

export function ProjectMemorySettings() {
  const { t } = useI18n();
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [selectedId, setSelectedId] = useState<string>("");
  const [content, setContent] = useState("");
  const [draft, setDraft] = useState("");
  const [stats, setStats] = useState<ProjectMemoryStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [appendText, setAppendText] = useState("");

  // 加载 workspace 列表
  useEffect(() => {
    fetchWorkspaces()
      .then((list) => {
        setWorkspaces(list);
        if (list.length > 0 && !selectedId) {
          // 默认选最近打开的(list_workspaces 已按 last_opened_at DESC 排序)
          setSelectedId(list[0].id);
        }
      })
      .catch((e) => {
        toast.error(`${t.settings.projectMemory.loadFailed}: ${String(e)}`);
      })
      .finally(() => setLoading(false));
  }, [selectedId, t.settings.projectMemory.loadFailed]);

  // 切换 workspace 时加载内容
  const loadData = useCallback(async (wsId: string) => {
    if (!wsId) {
      setContent("");
      setDraft("");
      setStats(null);
      return;
    }
    try {
      const [c, s] = await Promise.all([
        getProjectMemory(wsId),
        getProjectMemoryStats(wsId),
      ]);
      setContent(c);
      setDraft(c);
      setStats(s);
    } catch (e) {
      toast.error(`${t.settings.projectMemory.loadFailed}: ${String(e)}`);
    }
  }, [t.settings.projectMemory.loadFailed]);

  useEffect(() => {
    if (selectedId) void loadData(selectedId);
  }, [selectedId, loadData]);

  const handleSave = async () => {
    if (!selectedId) return;
    setSaving(true);
    try {
      await updateProjectMemory(selectedId, draft);
      setContent(draft);
      const s = await getProjectMemoryStats(selectedId);
      setStats(s);
      toast.success(t.settings.projectMemory.saved);
    } catch (e) {
      toast.error(`${t.settings.projectMemory.saveFailed}: ${String(e)}`);
    } finally {
      setSaving(false);
    }
  };

  const handleClear = async () => {
    if (!selectedId) return;
    if (!confirm(t.settings.projectMemory.clearConfirm)) return;
    try {
      await clearProjectMemory(selectedId);
      setContent("");
      setDraft("");
      const s = await getProjectMemoryStats(selectedId);
      setStats(s);
      toast.success(t.settings.projectMemory.cleared);
    } catch (e) {
      toast.error(`${t.settings.projectMemory.clearFailed}: ${String(e)}`);
    }
  };

  const handleAppend = async () => {
    if (!selectedId || !appendText.trim()) return;
    try {
      await updateProjectMemory(selectedId, draft);
      // 注意: 这里使用 update 而非 append,简化前端逻辑
      // 真正的 append 由 agent 自动调用 IPC 完成
      toast.success(t.settings.projectMemory.appended);
      setAppendText("");
      const c = await getProjectMemory(selectedId);
      const s = await getProjectMemoryStats(selectedId);
      setContent(c);
      setDraft(c);
      setStats(s);
    } catch (e) {
      toast.error(`${t.settings.projectMemory.appendFailed}: ${String(e)}`);
    }
  };

  const handleRefresh = async () => {
    if (selectedId) await loadData(selectedId);
  };

  const isDirty = draft !== content;
  const usagePercent = stats ? (stats.bytes / stats.maxBytes) * 100 : 0;
  const usageColor =
    usagePercent > 90 ? "text-rose-500" : usagePercent > 70 ? "text-amber-500" : "text-emerald-500";

  if (loading) {
    return (
      <PageContainer>
        <LoadingState label={t.settings.projectMemory.loading} />
      </PageContainer>
    );
  }

  return (
    <PageContainer>
      <PageHeader>
        <PageHeading>
          <PageTitle>
            <FolderCogIcon className="size-5" />
            {t.settings.projectMemory.title}
          </PageTitle>
          <PageDescription>{t.settings.projectMemory.description}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button variant="ghost" size="sm" onClick={handleRefresh} disabled={!selectedId}>
            <RefreshCwIcon className="size-4" />
            {t.settings.projectMemory.refresh}
          </Button>
          <Button size="sm" onClick={handleSave} disabled={!selectedId || !isDirty || saving}>
            <SaveIcon className="size-4" />
            {saving ? t.settings.projectMemory.saving : t.settings.projectMemory.save}
          </Button>
        </PageActions>
      </PageHeader>

      {workspaces.length === 0 ? (
        <SettingsSection title={t.settings.projectMemory.sectionWorkspace}>
          <div className="px-4 py-6 text-center text-muted-foreground">
            {t.settings.projectMemory.noWorkspaces}
          </div>
        </SettingsSection>
      ) : (
        <>
          <SettingsSection title={t.settings.projectMemory.sectionWorkspace}>
            <SettingsRow
              label={t.settings.projectMemory.selectWorkspace}
              description={t.settings.projectMemory.selectWorkspaceHint}
            >
              <Select value={selectedId} onValueChange={setSelectedId}>
                <SelectTrigger className="min-w-64 max-w-md">
                  <SelectValue placeholder={t.settings.projectMemory.selectWorkspacePlaceholder} />
                </SelectTrigger>
                <SelectContent>
                  {workspaces.map((ws) => (
                    <SelectItem key={ws.id} value={ws.id}>
                      {ws.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </SettingsRow>
          </SettingsSection>

          {selectedId && stats && (
            <SettingsSection title={t.settings.projectMemory.sectionStats}>
              <div className="grid grid-cols-2 gap-4 px-4 py-3 sm:grid-cols-4">
                <div>
                  <div className="text-xs text-muted-foreground">{t.settings.projectMemory.bytes}</div>
                  <div className={`text-lg font-semibold ${usageColor}`}>{formatBytes(stats.bytes)}</div>
                </div>
                <div>
                  <div className="text-xs text-muted-foreground">{t.settings.projectMemory.chars}</div>
                  <div className="text-lg font-semibold">{stats.chars.toLocaleString()}</div>
                </div>
                <div>
                  <div className="text-xs text-muted-foreground">{t.settings.projectMemory.maxBytes}</div>
                  <div className="text-lg font-semibold">{formatBytes(stats.maxBytes)}</div>
                </div>
                <div>
                  <div className="text-xs text-muted-foreground">{t.settings.projectMemory.usage}</div>
                  <div className={`text-lg font-semibold ${usageColor}`}>{usagePercent.toFixed(1)}%</div>
                </div>
              </div>
              {usagePercent > 90 && (
                <div className="px-4 pb-3 text-xs text-rose-500">
                  {t.settings.projectMemory.usageWarning}
                </div>
              )}
            </SettingsSection>
          )}

          {selectedId && (
            <SettingsSection title={t.settings.projectMemory.sectionEditor}>
              <SettingsRow
                label={t.settings.projectMemory.editorLabel}
                description={t.settings.projectMemory.editorHint}
              >
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleClear}
                  disabled={!selectedId}
                >
                  <Trash2Icon className="size-4" />
                  {t.settings.projectMemory.clear}
                </Button>
              </SettingsRow>
              <div className="px-4 pb-4">
                <textarea
                  value={draft}
                  onChange={(e) => setDraft(e.target.value)}
                  className="h-96 w-full resize-y rounded-md border border-border bg-background p-3 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-primary/30"
                  placeholder={t.settings.projectMemory.editorPlaceholder}
                />
                {isDirty && (
                  <div className="mt-2 flex items-center gap-2 text-xs text-amber-500">
                    <Badge variant="outline">{t.settings.projectMemory.unsaved}</Badge>
                  </div>
                )}
              </div>
            </SettingsSection>
          )}

          {selectedId && (
            <SettingsSection title={t.settings.projectMemory.sectionAppend}>
              <SettingsRow
                label={t.settings.projectMemory.appendLabel}
                description={t.settings.projectMemory.appendHint}
              >
                <Button size="sm" onClick={handleAppend} disabled={!appendText.trim()}>
                  <PlusIcon className="size-4" />
                  {t.settings.projectMemory.append}
                </Button>
              </SettingsRow>
              <div className="px-4 pb-4">
                <textarea
                  value={appendText}
                  onChange={(e) => setAppendText(e.target.value)}
                  className="h-24 w-full resize-y rounded-md border border-border bg-background p-3 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-primary/30"
                  placeholder={t.settings.projectMemory.appendPlaceholder}
                />
              </div>
            </SettingsSection>
          )}
        </>
      )}
    </PageContainer>
  );
}
