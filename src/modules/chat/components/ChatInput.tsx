import { useEffect, useRef, useState } from "react";
import {
  ArrowUp,
  StopCircle,
  Paperclip,
  Mic,
  Zap,
  X,
  FileText,
  BookOpen,
  FileCode,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { useI18n } from "@/shared/i18n";
import { cn } from "@/shared/utils";
import { useModelSettings } from "@/features/settings/hooks/useModelSettings";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { ContextPicker } from "./ContextPicker";
import { Button } from "@/components/ui/button";
import type { AttachmentSpec } from "@/shared/types";

export function ChatInput({
  value,
  onChange,
  onSubmit,
  onCancel,
  streaming,
  attachments,
  onAttachFile,
  onAddAttachment,
  onRemoveAttachment,
  workspaceId,
  workspacePath,
}: {
  value: string;
  onChange: (v: string) => void;
  onSubmit: () => void;
  onCancel: () => void;
  streaming: boolean;
  attachments: AttachmentSpec[];
  onAttachFile: (filePath: string) => void;
  onAddAttachment: (att: AttachmentSpec) => void;
  onRemoveAttachment: (index: number) => void;
  workspaceId: string | null;
  workspacePath: string | null;
}) {
  const { t } = useI18n();
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [quickPass, setQuickPass] = useState(false);

  const { models, activeModelId, setActiveModel } = useModelSettings();

  // Auto-resize textarea
  useEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${Math.min(Math.max(el.scrollHeight, 52), 200)}px`;
  }, [value]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      onSubmit();
    }
  };

  const canSend = value.trim().length > 0 && !streaming;

  // File picker handler
  const handleAttachFile = async () => {
    try {
      const selected = await open({
        directory: false,
        multiple: false,
        title: "Select file",
        filters: [{ name: "All", extensions: ["*"] }],
      });
      if (typeof selected === "string") {
        onAttachFile(selected);
      }
    } catch {
      // User cancelled or error -- silently ignore
    }
  };

  const activeModel = models.find((m) => m.id === activeModelId);

  return (
    <div className="border-t border-[var(--border-neutral-l1)] bg-[var(--bg-base-default)] px-4 pb-3 pt-2">
      <div className="mx-auto max-w-3xl">
        <div
          className={cn(
            "flex flex-col rounded-[var(--radius-8)] border bg-[var(--bg-base-default)] shadow-sm transition-[border-color,box-shadow]",
            value.trim()
              ? "border-[var(--border-neutral-l2)] focus-within:border-[var(--status-primary-default)]/40 focus-within:shadow-[0_0_0_2px_var(--status-primary-surface-l1)]"
              : "border-[var(--border-neutral-l1)]"
          )}
        >
          {/* Attachment chips */}
          {attachments.length > 0 && (
            <div className="flex flex-wrap gap-1.5 px-3 pt-2">
              {attachments.map((att, i) => (
                <span
                  key={`${att.kind}-${att.ref}-${i}`}
                  className="inline-flex items-center gap-1 rounded-[var(--radius-4)] bg-[var(--bg-overlay-l2)] py-1 pl-1.5 pr-1 text-[11px] text-[var(--text-secondary)]"
                >
                  {att.kind === "wiki" ? (
                    <BookOpen className="size-3 text-[var(--text-tertiary)]" />
                  ) : att.kind === "chapter" ? (
                    <FileCode className="size-3 text-[var(--text-tertiary)]" />
                  ) : att.kind === "file" ? (
                    <FileText className="size-3 text-[var(--text-tertiary)]" />
                  ) : (
                    <FileText className="size-3 text-[var(--text-tertiary)]" />
                  )}
                  <span className="max-w-40 truncate">{att.label}</span>
                  <Button
                    variant="ghost"
                    size="icon-xs"
                    onClick={() => onRemoveAttachment(i)}
                    className="size-3.5"
                    aria-label={t.agentChat.removeAttachment}
                  >
                    <X className="size-2.5" />
                  </Button>
                </span>
              ))}
            </div>
          )}

          {/* Textarea area */}
          <textarea
            ref={textareaRef}
            value={value}
            onChange={(e) => onChange(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={t.agentChat.placeholder}
            rows={1}
            className="w-full resize-none bg-transparent px-4 pt-3 pb-2 text-sm leading-[1.6] text-[var(--text-default)] outline-none placeholder:text-[var(--text-tertiary)]"
            style={{ minHeight: "52px", maxHeight: "200px" }}
          />

          {/* Bottom toolbar */}
          <div className="flex items-center justify-between px-2 pb-2">
            {/* Left tools */}
            <div className="flex items-center gap-0.5">
              {/* Paperclip -- real file picker */}
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon-lg"
                    onClick={() => { void handleAttachFile(); }}
                    aria-label={t.agentChat.attachFile}
                  >
                    <Paperclip />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>{t.agentChat.attachFile}</TooltipContent>
              </Tooltip>

              {/* Context picker -- wiki entries + chapter files */}
              <ContextPicker
                workspaceId={workspaceId}
                workspacePath={workspacePath}
                onAddAttachment={onAddAttachment}
              />

              {/* Quick pass toggle */}
              <Button
                variant={quickPass ? "secondary" : "ghost"}
                size="sm"
                onClick={() => setQuickPass((prev) => !prev)}
                aria-label={t.agentChat.quickPass}
              >
                <Zap className={cn(quickPass && "fill-[var(--bg-brand)]")} />
                <span>{t.agentChat.quickPass}</span>
              </Button>
            </div>

            {/* Right actions */}
            <div className="flex items-center gap-1.5">
              {/* Mic button -- disabled, coming soon */}
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon-lg"
                    disabled
                    aria-label="Voice input"
                  >
                    <Mic />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>Coming soon</TooltipContent>
              </Tooltip>

              {/* Model selector using shadcn Select */}
              <Select
                value={activeModelId ?? ""}
                onValueChange={(id) => { void setActiveModel(id); }}
              >
                <SelectTrigger
                  size="sm"
                  className="h-7 gap-1 rounded-[var(--radius-full)] border-[var(--border-neutral-l1)] bg-[var(--bg-base-secondary)] px-2.5 text-[11px] font-medium text-[var(--text-secondary)] hover:bg-[var(--bg-overlay-l2)]"
                >
                  <SelectValue placeholder="Select model">
                    {activeModel ? activeModel.name : null}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  {models.map((model) => (
                    <SelectItem key={model.id} value={model.id}>
                      <span className="truncate">
                        {model.name}
                        <span className="ml-1 text-[var(--text-tertiary)]">
                          ({model.provider})
                        </span>
                      </span>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>

              {/* Send / Stop */}
              {streaming ? (
                <Button
                  variant="ghost"
                  size="icon-lg"
                  onClick={onCancel}
                  className="rounded-full bg-[var(--status-error-default)]/10 text-[var(--status-error-default)] hover:bg-[var(--status-error-default)]/20"
                  aria-label={t.agentChat.stop}
                >
                  <StopCircle />
                </Button>
              ) : (
                <Button
                  variant={canSend ? "brand" : "ghost"}
                  size="icon-lg"
                  onClick={onSubmit}
                  disabled={!canSend}
                  className="rounded-full"
                  aria-label={t.agentChat.send}
                >
                  <ArrowUp />
                </Button>
              )}
            </div>
          </div>
        </div>

        {/* Keyboard hint */}
        <div className="mt-1.5 flex items-center justify-center gap-1 text-[10px] text-[var(--text-tertiary)]">
          <kbd className="rounded-[var(--radius-2)] border border-[var(--border-neutral-l1)] bg-[var(--bg-base-secondary)] px-1 py-px font-mono">
            Enter
          </kbd>
          <span>{t.common.send}</span>
          <span>·</span>
          <kbd className="rounded-[var(--radius-2)] border border-[var(--border-neutral-l1)] bg-[var(--bg-base-secondary)] px-1 py-px font-mono">
            Shift+Enter
          </kbd>
          <span>{t.agentChat.newline}</span>
        </div>
      </div>
    </div>
  );
}
