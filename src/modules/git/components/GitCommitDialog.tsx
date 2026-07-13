import { useEffect, useState, type KeyboardEvent } from "react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { ScrollArea } from "@/components/ui/scroll-area";
import { EmptyState } from "@/components/shared/state";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { PlusIcon, GitCommitIcon } from "lucide-react";
import { useI18n } from "@/shared/i18n";
import type { FileChange } from "@/shared/types";

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

  useEffect(() => {
    if (!open) {
      setMessage("");
    }
  }, [open]);

  const canSubmit = message.trim().length > 0 && stagedFiles.length > 0 && !loading;
  const hasUnstaged = unstagedPaths.length > 0 || untrackedPaths.length > 0;

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
      e.preventDefault();
      if (canSubmit) {
        onCommit(message);
      }
    }
  };

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
          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-medium text-foreground">
              {t.git.commit.messageLabel}
            </label>
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
              <label className="text-xs font-medium text-foreground">
                {t.git.commit.stagedFiles}
                <span className="ml-1 text-muted-foreground">({stagedFiles.length})</span>
              </label>
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
            onClick={() => onCommit(message)}
            disabled={!canSubmit}
          >
            <GitCommitIcon />
            {t.git.commit.submit}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
