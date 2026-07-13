import { useEffect, useState } from "react";
import { BookOpen, FileText, Loader2 } from "lucide-react";
import { useI18n } from "@/shared/i18n";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { Button } from "@/components/ui/button";
import { EmptyState } from "@/components/shared/state";
import { fetchNovels } from "@/features/novel/services";
import { listWikiEntries } from "@/features/wiki/services";
import { listDirectory } from "@/features/chat/services/fs";
import type { AttachmentSpec, WikiEntry, FileEntry } from "@/shared/types";

/**
 * ContextPicker: 操作栏中的「添加上下文」按钮。
 *
 * 点击后弹出 DropdownMenu，列出当前工作区下可添加到 AI 上下文的内容：
 * - Wiki 条目（按 novel_id 查询）
 * - 章节文件（从 <workspace>/chapters/ 列出 .md 文件）
 *
 * 选中后通过 onAddAttachment 回调把 AttachmentSpec 传给父组件。
 * 数据懒加载：仅在下拉打开时拉取，避免空闲时的无谓 IPC。
 */
export function ContextPicker({
  workspaceId,
  workspacePath,
  onAddAttachment,
}: {
  workspaceId: string | null;
  workspacePath: string | null;
  onAddAttachment: (att: AttachmentSpec) => void;
}) {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [wikiEntries, setWikiEntries] = useState<WikiEntry[]>([]);
  const [chapters, setChapters] = useState<FileEntry[]>([]);

  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    void (async () => {
      setLoading(true);
      try {
        // Wiki: workspace → 第一个 novel → entries
        const wiki: WikiEntry[] = [];
        if (workspaceId) {
          const novels = await fetchNovels();
          const novel = novels.find((n) => n.workspace_id === workspaceId);
          if (novel) {
            const entries = await listWikiEntries(novel.id);
            wiki.push(...entries);
          }
        }
        // Chapters: <workspace>/chapters/*.md
        const chaps: FileEntry[] = [];
        if (workspacePath) {
          try {
            const items = await listDirectory(workspacePath + "\\chapters");
            chaps.push(...items.filter((e) => !e.is_dir && e.extension === "md"));
          } catch {
            // chapters 目录不存在是正常情况（新工作区），不报错
          }
        }
        if (!cancelled) {
          setWikiEntries(wiki);
          setChapters(chaps);
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [open, workspaceId, workspacePath]);

  const handlePickWiki = (entry: WikiEntry) => {
    onAddAttachment({
      kind: "wiki",
      ref: entry.id,
      label: entry.title,
    });
    setOpen(false);
  };

  const handlePickChapter = (file: FileEntry) => {
    // ref 用章节号（文件名去掉 .md），Rust 端按 <workspace>/chapters/<n>.md 读取
    const chapterNum = file.name.replace(/\.md$/i, "");
    onAddAttachment({
      kind: "chapter",
      ref: chapterNum,
      label: `${t.agentChat.chapterSection} ${chapterNum}`,
    });
    setOpen(false);
  };

  const isEmpty = !loading && wikiEntries.length === 0 && chapters.length === 0;

  return (
    <DropdownMenu open={open} onOpenChange={setOpen}>
      <Tooltip>
        <TooltipTrigger asChild>
          <DropdownMenuTrigger asChild>
            <Button
              variant="ghost"
              size="icon-lg"
              aria-label={t.agentChat.attachContext}
            >
              <BookOpen />
            </Button>
          </DropdownMenuTrigger>
        </TooltipTrigger>
        <TooltipContent>{t.agentChat.attachContext}</TooltipContent>
      </Tooltip>
      <DropdownMenuContent align="start" className="w-64">
        {loading ? (
          <div className="flex items-center justify-center py-4">
            <Loader2 className="size-4 animate-spin text-[var(--text-tertiary)]" />
          </div>
        ) : isEmpty ? (
          <EmptyState
            title={workspaceId ? t.agentChat.noContext : t.agentChat.noWorkspaceHint}
            className="py-3"
          />
        ) : (
          <>
            {wikiEntries.length > 0 && (
              <>
                <DropdownMenuLabel className="text-[10px] uppercase text-[var(--text-tertiary)]">
                  {t.agentChat.wikiSection}
                </DropdownMenuLabel>
                {wikiEntries.map((entry) => (
                  <DropdownMenuItem
                    key={entry.id}
                    onClick={() => handlePickWiki(entry)}
                    className="gap-2 text-xs"
                  >
                    <BookOpen className="size-3 shrink-0 text-[var(--text-tertiary)]" />
                    <span className="truncate">{entry.title}</span>
                  </DropdownMenuItem>
                ))}
              </>
            )}
            {chapters.length > 0 && (
              <>
                {wikiEntries.length > 0 && <DropdownMenuSeparator />}
                <DropdownMenuLabel className="text-[10px] uppercase text-[var(--text-tertiary)]">
                  {t.agentChat.chapterSection}
                </DropdownMenuLabel>
                {chapters.map((file) => (
                  <DropdownMenuItem
                    key={file.path}
                    onClick={() => handlePickChapter(file)}
                    className="gap-2 text-xs"
                  >
                    <FileText className="size-3 shrink-0 text-[var(--text-tertiary)]" />
                    <span className="truncate">{file.name}</span>
                  </DropdownMenuItem>
                ))}
              </>
            )}
          </>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
