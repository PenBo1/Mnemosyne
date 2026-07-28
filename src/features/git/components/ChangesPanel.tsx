/**
 * ═══════════════════════════════════════════════════════════════════════════
 * ChangesPanel - Git 文件变更面板组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useState } from "react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Collapsible,
  CollapsibleTrigger,
  CollapsibleContent,
} from "@/components/ui/collapsible";
import {
  ChevronRightIcon,
  PlusIcon,
} from "lucide-react";
import { useI18n } from "@/locales/i18n";
import type {
  Diff,
  FileChange,
  FileStatusType,
  FileDiff,
  GitStatus,
} from "@/features/git/types";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface ChangesPanelProps {
  status: GitStatus;
  diff: Diff | null;
  onStage: (path: string) => void;
  onStageAll: () => void;
  className?: string;
}

interface FileRowProps {
  change: FileChange;
  diff: Diff | null;
  expanded: boolean;
  onToggleExpand: () => void;
  onStage: (path: string) => void;
}

interface GroupProps {
  label: string;
  count: number;
  defaultOpen?: boolean;
  action?: React.ReactNode;
  children: React.ReactNode;
}

// ── 常量配置 ────────────────────────────────────────────────────────────────

const STATUS_LETTER: Record<FileStatusType, string> = {
  unmodified: "",
  modified: "M",
  added: "A",
  deleted: "D",
  untracked: "?",
  renamed: "R",
  copied: "C",
  conflicted: "U",
};

const STATUS_COLOR: Record<FileStatusType, string> = {
  unmodified: "text-muted-foreground",
  modified: "text-[var(--status-warning-default)]",
  added: "text-[var(--status-success-default)]",
  deleted: "text-destructive",
  untracked: "text-muted-foreground",
  renamed: "text-[var(--color-info)]",
  copied: "text-[var(--color-info)]",
  conflicted: "text-[var(--status-warning-default)]",
};

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/**
 * 分割路径为文件名和目录
 */
function splitPath(path: string): { name: string; dir: string } {
  const idx = path.lastIndexOf("/");
  if (idx === -1) return { name: path, dir: "" };
  return { name: path.slice(idx + 1), dir: path.slice(0, idx) };
}

// ── 子组件 ──────────────────────────────────────────────────────────────────

/**
 * 文件行组件
 */
function FileRow({
  change,
  diff,
  expanded,
  onToggleExpand,
  onStage,
}: Omit<FileRowProps, 'onUnstage'>) {
  const { t } = useI18n();
  const { name, dir } = splitPath(change.path);
  const fileDiff: FileDiff | undefined = diff?.files.find((f) => f.path === change.path);

  return (
    <div className="flex flex-col">
      <div className="group flex items-center gap-1 px-2 py-0.5 hover:bg-[var(--bg-overlay-l1)]">
        <ChevronRightIcon
          className={cn(
            "size-3.5 shrink-0 text-muted-foreground transition-transform",
            expanded && "rotate-90"
          )}
        />
        <button
          type="button"
          onClick={onToggleExpand}
          className="flex min-w-0 flex-1 items-center gap-2 text-left"
        >
          <span
            className={cn(
              "w-3 shrink-0 text-center text-xs font-semibold",
              STATUS_COLOR[change.status]
            )}
            title={change.status}
          >
            {STATUS_LETTER[change.status]}
          </span>
          <span className="truncate text-xs">{name}</span>
          {dir && (
            <span className="truncate text-xs text-muted-foreground">{dir}</span>
          )}
        </button>
        <Button
          variant="ghost"
          size="icon-xs"
          className="opacity-0 group-hover:opacity-100"
          onClick={() => onStage(change.path)}
          title={t.git.stageAll}
        >
          <PlusIcon />
        </Button>
      </div>
      {expanded && (
        <div className="border-b border-[var(--border-neutral-l1)] bg-[var(--bg-overlay-l1)]">
          {fileDiff?.binary ? (
            <div className="px-3 py-2 text-xs text-muted-foreground">
              二进制文件
            </div>
          ) : (
            <div className="px-3 py-2 text-xs text-muted-foreground">
              {t.git.diff.noDiff}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

/**
 * 分组组件
 */
function Group({ label, count, defaultOpen = true, action, children }: GroupProps) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <div className="group flex items-center gap-1 px-2 py-1">
        <CollapsibleTrigger asChild>
          <button
            type="button"
            className="flex flex-1 items-center gap-1 text-xs font-medium text-muted-foreground"
          >
            <ChevronRightIcon
              className={cn("size-3.5 transition-transform", open && "rotate-90")}
            />
            <span>{label}</span>
            <Badge variant="secondary" className="size-5 justify-center text-[0.625rem]">
              {count}
            </Badge>
          </button>
        </CollapsibleTrigger>
        {action}
      </div>
      <CollapsibleContent>
        <div className="flex flex-col">{children}</div>
      </CollapsibleContent>
    </Collapsible>
  );
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 文件变更面板，展示所有变更文件
 */
export function ChangesPanel({
  status,
  diff,
  onStage,
  onStageAll,
  className,
}: ChangesPanelProps) {
  const { t } = useI18n();
  const [expandedPath, setExpandedPath] = useState<string | null>(null);

  const files = status.files;
  const hasFiles = files.length > 0;

  const toggleExpand = (path: string) => {
    setExpandedPath((cur) => (cur === path ? null : path));
  };

  const renderRow = (change: FileChange) => (
    <FileRow
      key={change.path}
      change={change}
      diff={diff}
      expanded={expandedPath === change.path}
      onToggleExpand={() => toggleExpand(change.path)}
      onStage={onStage}
    />
  );

  return (
    <div className={cn("flex flex-col py-1", className)}>
      {hasFiles && (
        <Group
          label={t.git.changes}
          count={files.length}
          action={
            <Button
              variant="ghost"
              size="icon-xs"
              className="opacity-0 group-hover:opacity-100"
              onClick={onStageAll}
              title={t.git.stageAll}
            >
              <PlusIcon />
            </Button>
          }
        >
          {files.map(renderRow)}
        </Group>
      )}
    </div>
  );
}