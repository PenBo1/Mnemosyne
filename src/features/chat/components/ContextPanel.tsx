import { useState, useEffect, useCallback } from "react";
import {
  FolderIcon,
  FileIcon,
  ChevronLeft,
  Shrink,
  Loader2,
  AlertCircle,
  X,
} from "lucide-react";
import { useI18n } from "@/shared/i18n";
import { cn } from "@/shared/utils";
import { ScrollArea } from "@/components/ui/scroll-area";
import { readFile, listDirectory } from "@/features/chat/services/fs";
import type { FileEntry } from "@/shared/types";

/**
 * ContextPanel: right sidebar with Todos, Context usage, and file browser.
 * Renders as an inline 280px panel (NOT a Sheet/Dialog).
 */
export function ContextPanel({
  open,
  onToggle: _onToggle,
  workspacePath,
}: {
  open: boolean;
  onToggle: () => void;
  workspacePath: string | null;
}) {
  if (!open) return null;

  return (
    <div className="flex h-full w-[280px] shrink-0 flex-col border-l border-[var(--border-neutral-l1)] bg-[var(--bg-base-secondary)]">
      <TodosSection />
      <ContextUsageSection />
      <FileBrowserSection workspacePath={workspacePath} />
    </div>
  );
}

// ── Todos Section ──────────────────────────────────────────

function TodosSection() {
  const { t } = useI18n();

  return (
    <div className="border-b border-[var(--border-neutral-l1)] px-3 py-3">
      <div className="flex items-center gap-2">
        <h3 className="text-xs font-medium text-[var(--text-secondary)]">
          {t.agentChat.todos}
        </h3>
      </div>
      <div className="mt-1 text-center text-[11px] text-[var(--text-tertiary)]">
        {t.agentChat.todosEmpty}
      </div>
    </div>
  );
}

// ── Context Usage Section ──────────────────────────────────

function ContextUsageSection() {
  const { t } = useI18n();
  const contextPercent = 0;

  return (
    <div className="border-b border-[var(--border-neutral-l1)] px-3 py-3">
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium text-[var(--text-secondary)]">
          {t.agentChat.contextUsage}
        </span>
        <button
          type="button"
          className="flex items-center gap-0.5 rounded-[var(--radius-4)] px-1.5 py-0.5 text-[10px] text-[var(--text-tertiary)] transition-colors hover:bg-[var(--bg-overlay-l1)] hover:text-[var(--text-secondary)]"
        >
          <Shrink className="size-2.5" />
          <span>{t.agentChat.compress}</span>
        </button>
      </div>
      <div className="mt-2 h-1 overflow-hidden rounded-full bg-[var(--bg-overlay-l2)]">
        <div
          className="h-full rounded-full bg-[var(--status-success-default)] transition-all"
          style={{ width: `${contextPercent}%` }}
        />
      </div>
      <span className="mt-1 block text-right text-[10px] text-[var(--text-tertiary)]">
        {contextPercent}%
      </span>
    </div>
  );
}

// ── File Browser Section ────────────────────────────────────

function FileBrowserSection({
  workspacePath,
}: {
  workspacePath: string | null;
}) {
  const { t } = useI18n();

  const [currentPath, setCurrentPath] = useState<string>("");
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [previewFile, setPreviewFile] = useState<{
    name: string;
    content: string;
  } | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);

  // Load directory entries
  const loadDirectory = useCallback(
    async (dirPath: string) => {
      if (!dirPath) return;
      setLoading(true);
      setError(null);
      setPreviewFile(null);
      try {
        const items = await listDirectory(dirPath);
        // Sort: directories first, then files alphabetically
        items.sort((a, b) => {
          if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
          return a.name.localeCompare(b.name);
        });
        setEntries(items);
        setCurrentPath(dirPath);
      } catch (err) {
        setError(
          err instanceof Error ? err.message : (t.agentChat.todosEmpty as string)
        );
        setEntries([]);
      } finally {
        setLoading(false);
      }
    },
    [t.agentChat.todosEmpty]
  );

  // Load workspace root on mount or workspacePath change
  useEffect(() => {
    if (workspacePath) {
      loadDirectory(workspacePath);
    } else {
      setEntries([]);
      setCurrentPath("");
      setError(null);
      setPreviewFile(null);
    }
  }, [workspacePath, loadDirectory]);

  // Handle directory click
  const handleDirClick = (entry: FileEntry) => {
    if (entry.is_dir) {
      void loadDirectory(entry.path);
    }
  };

  // Handle back button
  const handleBack = () => {
    if (!currentPath) return;
    const parent = currentPath.split(/[\\/]/).slice(0, -1).join("\\");
    if (parent && parent !== currentPath) {
      void loadDirectory(parent);
    }
  };

  // Handle file click -- read and preview
  const handleFileClick = async (entry: FileEntry) => {
    if (entry.is_dir) return;
    setPreviewLoading(true);
    setPreviewFile(null);
    try {
      const content = await readFile(entry.path);
      setPreviewFile({ name: entry.name, content });
    } catch (err) {
      setPreviewFile({
        name: entry.name,
        content: err instanceof Error ? err.message : "Failed to read file",
      });
    } finally {
      setPreviewLoading(false);
    }
  };

  // Breadcrumb path parts
  const pathParts = currentPath
    ? currentPath.replace(/[\\/]+/g, "/").split("/").filter(Boolean)
    : [];

  // Get file icon color based on extension
  const getFileIconColor = (extension: string | null): string => {
    if (!extension) return "text-[var(--text-tertiary)]";
    const ext = extension.toLowerCase();
    const colorMap: Record<string, string> = {
      ts: "text-[#3178C6]",
      tsx: "text-[#3178C6]",
      js: "text-[#F7DF1E]",
      jsx: "text-[#F7DF1E]",
      json: "text-[#F7DF1E]",
      rs: "text-[#DEA584]",
      toml: "text-[#9C4221]",
      css: "text-[#1572B6]",
      scss: "text-[#CD6799]",
      html: "text-[#E34F26]",
      md: "text-[#519ABA]",
      py: "text-[#3776AB]",
      go: "text-[#00ADD8]",
      svg: "text-[#FFB13B]",
      png: "text-[#A855F7]",
      jpg: "text-[#A855F7]",
      gif: "text-[#A855F7]",
    };
    return colorMap[ext] ?? "text-[var(--text-tertiary)]";
  };

  // No workspace
  if (!workspacePath) {
    return (
      <div className="flex flex-1 flex-col">
        <div className="border-b border-[var(--border-neutral-l1)] px-3 py-2">
          <span className="text-xs font-medium text-[var(--text-secondary)]">
            {t.agentChat.files}
          </span>
        </div>
        <div className="flex flex-1 items-center justify-center px-3">
          <span className="text-[11px] text-[var(--text-tertiary)]">
            {t.agentChat.todosEmpty}
          </span>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      {/* File browser header */}
      <div className="border-b border-[var(--border-neutral-l1)] px-3 py-2">
        <div className="flex items-center justify-between">
          <span className="text-xs font-medium text-[var(--text-secondary)]">
            {t.agentChat.files}
          </span>
          {previewFile && (
            <button
              type="button"
              onClick={() => setPreviewFile(null)}
              className="flex size-4 items-center justify-center rounded-[var(--radius-2)] text-[var(--text-tertiary)] transition-colors hover:bg-[var(--bg-overlay-l1)] hover:text-[var(--text-secondary)]"
            >
              <X className="size-3" />
            </button>
          )}
        </div>

        {/* Breadcrumb */}
        {currentPath && (
          <div className="mt-1.5 flex items-center gap-1 overflow-x-auto text-[10px]">
            {pathParts.length > 1 && (
              <button
                type="button"
                onClick={handleBack}
                className="shrink-0 text-[var(--text-tertiary)] transition-colors hover:text-[var(--text-secondary)]"
              >
                <ChevronLeft className="size-3" />
              </button>
            )}
            {pathParts.map((part, i) => (
              <span key={i} className="flex shrink-0 items-center gap-0.5">
                {i > 0 && (
                  <span className="text-[var(--text-tertiary)]">/</span>
                )}
                <span
                  className={cn(
                    "transition-colors",
                    i === pathParts.length - 1
                      ? "text-[var(--text-secondary)]"
                      : "text-[var(--text-tertiary)]"
                  )}
                >
                  {part}
                </span>
              </span>
            ))}
          </div>
        )}
      </div>

      {previewFile ? (
        /* ── File Preview ── */
        <div className="flex min-h-0 flex-1 flex-col">
          <ScrollArea className="flex-1">
            <div className="px-3 py-2">
              <div className="mb-1 text-[11px] font-medium text-[var(--text-default)]">
                {previewFile.name}
              </div>
              <pre className="overflow-x-auto whitespace-pre-wrap break-words rounded-[var(--radius-4)] bg-[var(--bg-base-default)] p-2 font-mono text-[10px] leading-[1.5] text-[var(--text-secondary)]">
                {previewFile.content}
              </pre>
            </div>
          </ScrollArea>
        </div>
      ) : (
        /* ── File List ── */
        <div className="flex min-h-0 flex-1 flex-col">
          {loading ? (
            <div className="flex flex-1 items-center justify-center">
              <Loader2 className="size-4 animate-spin text-[var(--text-tertiary)]" />
            </div>
          ) : error ? (
            <div className="flex flex-1 flex-col items-center justify-center gap-1.5 px-3">
              <AlertCircle className="size-3.5 text-[var(--status-error-default)]" />
              <span className="text-center text-[11px] text-[var(--status-error-default)]">
                {error}
              </span>
            </div>
          ) : entries.length === 0 ? (
            <div className="flex flex-1 items-center justify-center px-3">
              <span className="text-[11px] text-[var(--text-tertiary)]">
                {t.agentChat.todosEmpty}
              </span>
            </div>
          ) : (
            <ScrollArea className="flex-1">
              <div className="flex flex-col">
                {entries.map((entry) => (
                  <button
                    key={entry.path}
                    type="button"
                    onClick={() => {
                      if (entry.is_dir) {
                        handleDirClick(entry);
                      } else {
                        void handleFileClick(entry);
                      }
                    }}
                    className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs transition-colors hover:bg-[var(--bg-overlay-l1)]"
                  >
                    {entry.is_dir ? (
                      <FolderIcon className="size-3 shrink-0 text-[#FFB13B]" />
                    ) : (
                      <FileIcon
                        className={cn(
                          "size-3 shrink-0",
                          getFileIconColor(entry.extension)
                        )}
                      />
                    )}
                    <span className="truncate text-[var(--text-secondary)]">
                      {entry.name}
                    </span>
                  </button>
                ))}
              </div>
            </ScrollArea>
          )}
        </div>
      )}

      {/* Preview loading indicator */}
      {previewLoading && (
        <div className="absolute inset-0 flex items-center justify-center bg-[var(--bg-base-secondary)]/60">
          <Loader2 className="size-4 animate-spin text-[var(--text-tertiary)]" />
        </div>
      )}
    </div>
  );
}
