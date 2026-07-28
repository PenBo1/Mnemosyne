/**
 * ═══════════════════════════════════════════════════════════════════════════
 * GitCommitDialog - Git 提交对话框组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useEffect, useState, type KeyboardEvent } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { ScrollArea } from "@/components/ui/scroll-area";
import { EmptyState } from "@/components/shared/state";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { PlusIcon, GitCommitIcon } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import type { FileChange } from "@/features/git/types";

// ── 常量配置 ────────────────────────────────────────────────────────────────

/** 提交类型列表 */
const COMMIT_TYPES = [
  "feat",
  "fix",
  "docs",
  "style",
  "refactor",
  "perf",
  "test",
  "build",
  "ci",
  "chore",
  "revert",
] as const;

/** 提交主题长度警告阈值 */
const SUBJECT_WARN_LIMIT = 50;

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface GitCommitDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  stagedFiles: FileChange[];
  unstagedPaths: string[];
  untrackedPaths: string[];
  loading: boolean;
  onStageAll: () => void;
  onCommit: (message: string) => void;
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * Git 提交对话框，用于编写提交信息并提交暂存文件
 */
export function GitCommitDialog({
  open,
  onOpenChange,
  stagedFiles,
  unstagedPaths,
  untrackedPaths,
  loading,
  onStageAll,
  onCommit,
}: GitCommitDialogProps) {
  const { t } = useI18n();
  const [message, setMessage] = useState("");
  const [commitType, setCommitType] = useState<string>("");
  const [scope, setScope] = useState("");

  // ── 状态重置 ──────────────────────────────────────────────────────────────

  useEffect(() => {
    if (!open) {
      setMessage("");
      setCommitType("");
      setScope("");
    }
  }, [open]);

  // ── 计算属性 ──────────────────────────────────────────────────────────────

  const subject = message.split("\n")[0] ?? "";
  const subjectLength = subject.length;
  const subjectTooLong = subjectLength > SUBJECT_WARN_LIMIT;

  /**
   * 构建提交信息
   */
  const buildMessage = (): string => {
    if (!commitType) return message;
    const trimmedScope = scope.trim();
    const prefix = trimmedScope
      ? `${commitType}(${trimmedScope}): `
      : `${commitType}: `;
    return `${prefix}${message}`;
  };

  const canSubmit = message.trim().length > 0 && stagedFiles.length > 0 && !loading;
  const hasUnstaged = unstagedPaths.length > 0 || untrackedPaths.length > 0;

  // ── 事件处理 ──────────────────────────────────────────────────────────────

  /**
   * 提交处理
   */
  const handleSubmit = () => {
    if (canSubmit) {
      onCommit(buildMessage());
    }
  };

  /**
   * 键盘快捷键处理
   */
  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
      e.preventDefault();
      handleSubmit();
    }
  };

  // ── 渲染 ──────────────────────────────────────────────────────────────────

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <GitCommitIcon className="size-4" />
            {t.git.commit.title}
          </DialogTitle>
          <DialogDescription>
            {t.git.commit.messagePlaceholder}
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-3">
          <div className="flex items-center gap-2">
            <div className="flex flex-col gap-1.5 flex-1 min-w-0">
              <Label className="text-xs text-muted-foreground">
                {t.git.commitType}
              </Label>
              <Select
                value={commitType || "none"}
                onValueChange={(v) => setCommitType(v === "none" ? "" : v)}
              >
                <SelectTrigger className="w-full" size="sm">
                  <SelectValue placeholder={t.git.commitType} />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">—</SelectItem>
                  {COMMIT_TYPES.map((tp) => (
                    <SelectItem key={tp} value={tp}>
                      {tp}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="flex flex-col gap-1.5 w-32 shrink-0">
              <Label className="text-xs text-muted-foreground">
                {t.git.commitScope}
              </Label>
              <Input
                value={scope}
                onChange={(e) => setScope(e.target.value)}
                placeholder="ui"
                className="h-6"
              />
            </div>
          </div>

          <div className="flex flex-col gap-1.5">
            <div className="flex items-center justify-between">
              <Label>{t.git.commit.messageLabel}</Label>
              <span
                className={
                  subjectTooLong
                    ? "text-xs text-[var(--status-warning-default)]"
                    : "text-xs text-muted-foreground"
                }
              >
                {subjectLength}
                {subjectTooLong && ` · ${t.git.subjectTooLong}`}
              </span>
            </div>
            <Textarea
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={t.git.commit.messagePlaceholder}
              className="min-h-24 resize-y"
              autoFocus
            />
          </div>

          <div className="flex flex-col gap-1.5">
            <div className="flex items-center justify-between">
              <Label>
                {t.git.commit.stagedFiles}
                <span className="ml-1 text-muted-foreground">({stagedFiles.length})</span>
              </Label>
              <Button
                variant="ghost"
                size="xs"
                onClick={onStageAll}
                disabled={!hasUnstaged || loading}
              >
                <PlusIcon />
                {t.git.commit.stageAll}
              </Button>
            </div>
            <div className="rounded-[var(--radius-3)] border border-[var(--border-neutral-l1)] bg-muted/30 max-h-32 overflow-hidden">
              <ScrollArea className="h-full max-h-32">
                <div className="p-1.5 flex flex-col gap-0.5">
                  {stagedFiles.length === 0 ? (
                    <EmptyState title={t.git.commit.noStagedFiles} className="py-3" />
                  ) : (
                    stagedFiles.map((file) => (
                      <div
                        key={file.path}
                        className="flex items-center justify-between gap-2 px-2 py-1 text-xs font-mono"
                      >
                        <span className="truncate">{file.path}</span>
                        <span className="text-muted-foreground">{file.status}</span>
                      </div>
                    ))
                  )}
                </div>
              </ScrollArea>
            </div>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={loading}>
            {t.git.commit.cancel}
          </Button>
          <Button
            onClick={handleSubmit}
            disabled={!canSubmit}
          >
            <GitCommitIcon className="size-4" />
            {t.git.commit.submit}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}