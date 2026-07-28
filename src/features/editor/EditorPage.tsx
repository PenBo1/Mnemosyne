/**
 * ═══════════════════════════════════════════════════════════════════════════
 * EditorPage - 代码编辑器主页面
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useState, useCallback, useEffect } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import {
  PageContainer,
  PageHeader,
  PageHeading,
  PageTitle,
  PageDescription,
  PageActions,
} from "@/components/shared/page-layout";
import { EmptyState } from "@/components/shared/state";
import {
  FolderOpenIcon,
  SaveIcon,
  XIcon,
  FileTextIcon,
} from "lucide-react";
import { useI18n } from "@/locales/i18n";
import { useWorkspaceStore } from "@/features/workspace/store/workspace";
import { CodeEditor } from "@/features/editor/components/CodeEditor";
import { editorReadFile, editorWriteFile } from "@/features/editor/services";
import type { EditorTab } from "@/features/editor/types";
import { cn } from "@/lib/utils";
import { toast } from "sonner";

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/**
 * 生成唯一 tab id
 */
function makeTabId(): string {
  return `tab-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

/**
 * 从路径提取文件名
 */
function basename(path: string): string {
  const parts = path.replace(/\\/g, "/").split("/");
  return parts[parts.length - 1] || path;
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 代码编辑器主页面，支持文件标签页、打开和保存功能
 */
export function EditorPage() {
  const { t } = useI18n();
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspaceId);
  const [tabs, setTabs] = useState<EditorTab[]>([]);
  const [activeTabId, setActiveTabId] = useState<string | null>(null);

  const activeTab = tabs.find((tab) => tab.id === activeTabId) ?? null;

  // ── 文件操作 ──────────────────────────────────────────────────────────────

  /**
   * 打开文件对话框并加载文件
   */
  const handleOpenFile = useCallback(async () => {
    const selected = await open({
      multiple: true,
      title: t.editor.openFile,
      filters: [
        { name: "All Files", extensions: ["*"] },
        { name: "Code", extensions: ["ts", "tsx", "js", "jsx", "rs", "py", "json", "html", "css"] },
        { name: "Markdown", extensions: ["md", "markdown"] },
        { name: "Config", extensions: ["toml", "yaml", "yml", "json"] },
      ],
    });

    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];

    for (const filePath of paths) {
      // 若已打开同一文件，切换到该 tab
      const existing = tabs.find((tab) => tab.path === filePath);
      if (existing) {
        setActiveTabId(existing.id);
        continue;
      }

      try {
        const file = await editorReadFile(filePath, activeWorkspaceId ?? undefined);
        const tab: EditorTab = {
          id: makeTabId(),
          path: file.path,
          name: basename(file.path),
          language: file.language,
          content: file.content,
          originalContent: file.content,
          isDirty: false,
          readOnly: false,
        };
        setTabs((prev) => [...prev, tab]);
        setActiveTabId(tab.id);
      } catch (err) {
        toast.error(`${t.editor.openFailed}: ${err instanceof Error ? err.message : String(err)}`);
      }
    }
  }, [t, activeWorkspaceId, tabs]);

  /**
   * 内容变化时更新 tab 状态
   */
  const handleContentChange = useCallback((tabId: string, content: string) => {
    setTabs((prev) =>
      prev.map((tab) =>
        tab.id === tabId
          ? { ...tab, content, isDirty: content !== tab.originalContent }
          : tab,
      ),
    );
  }, []);

  /**
   * 保存当前文件
   */
  const handleSave = useCallback(async () => {
    if (!activeTab) return;
    try {
      await editorWriteFile(activeTab.path, activeTab.content, activeWorkspaceId ?? undefined);
      setTabs((prev) =>
        prev.map((tab) =>
          tab.id === activeTab.id
            ? { ...tab, originalContent: tab.content, isDirty: false }
            : tab,
        ),
      );
      toast.success(t.editor.saved);
    } catch (err) {
      toast.error(`${t.editor.saveFailed}: ${err instanceof Error ? err.message : String(err)}`);
    }
  }, [activeTab, activeWorkspaceId, t]);

  /**
   * 关闭 tab
   */
  const handleCloseTab = useCallback(
    (tabId: string) => {
      const tab = tabs.find((t) => t.id === tabId);
      if (!tab) return;
      if (tab.isDirty) {
        const confirmed = window.confirm(t.editor.unsavedConfirm);
        if (!confirmed) return;
      }
      setTabs((prev) => prev.filter((t) => t.id !== tabId));
      if (activeTabId === tabId) {
        const remaining = tabs.filter((t) => t.id !== tabId);
        setActiveTabId(remaining[0]?.id ?? null);
      }
    },
    [tabs, activeTabId, t],
  );

  // Ctrl+S 快捷键
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "s") {
        e.preventDefault();
        if (activeTab) handleSave();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [activeTab, handleSave]);

  // ── 渲染 ──────────────────────────────────────────────────────────────────

  return (
    <PageContainer scrollable={false} className="p-0">
      <PageHeader className="px-4 py-3 border-b border-border">
        <PageHeading>
          <PageTitle>
            <FileTextIcon className="size-5" />
            {t.editor.title}
          </PageTitle>
          <PageDescription>{t.editor.description}</PageDescription>
        </PageHeading>
        <PageActions>
          <Button variant="outline" size="sm" onClick={handleOpenFile}>
            <FolderOpenIcon className="size-4" />
            {t.editor.openFile}
          </Button>
          <Button
            variant="default"
            size="sm"
            onClick={handleSave}
            disabled={!activeTab || !activeTab.isDirty}
          >
            <SaveIcon className="size-4" />
            {t.editor.save}
          </Button>
        </PageActions>
      </PageHeader>

      {/* ── 文件标签页 ──────────────────────────────────────────────────────── */}
      {tabs.length > 0 && (
        <div className="flex items-center border-b border-border bg-[var(--bg-overlay-l2)] overflow-x-auto">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTabId(tab.id)}
              className={cn(
                "flex items-center gap-2 px-3 py-2 text-xs border-r border-border whitespace-nowrap transition-colors",
                "hover:bg-[var(--bg-overlay-l1)]",
                activeTabId === tab.id
                  ? "bg-[var(--bg-overlay-l1)] text-foreground border-b-2 border-b-primary"
                  : "text-muted-foreground",
              )}
            >
              <span>{tab.name}</span>
              {tab.isDirty && (
                <span className="size-1.5 rounded-full bg-primary" />
              )}
              <span
                role="button"
                tabIndex={0}
                onClick={(e) => {
                  e.stopPropagation();
                  handleCloseTab(tab.id);
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.stopPropagation();
                    handleCloseTab(tab.id);
                  }
                }}
                className="rounded p-0.5 hover:bg-[var(--bg-overlay-l3)] cursor-pointer"
              >
                <XIcon className="size-3" />
              </span>
            </button>
          ))}
        </div>
      )}

      {/* ── 编辑器区域 ──────────────────────────────────────────────────────── */}
      <div className="flex-1 min-h-0">
        {activeTab ? (
          <CodeEditor
            key={activeTab.id}
            value={activeTab.content}
            language={activeTab.language}
            readOnly={activeTab.readOnly}
            onChange={(content) => handleContentChange(activeTab.id, content)}
            onSave={handleSave}
          />
        ) : (
          <EmptyState
            title={t.editor.emptyTitle}
            description={t.editor.emptyDesc}
            className="h-full"
          />
        )}
      </div>
    </PageContainer>
  );
}