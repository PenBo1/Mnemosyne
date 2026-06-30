import { useEffect, useRef, useState } from "react";
import {
  ArrowUp,
  StopCircle,
  Paperclip,
  Mic,
  Zap,
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

export function ChatInput({
  value,
  onChange,
  onSubmit,
  onCancel,
  streaming,
  onAttachFile,
}: {
  value: string;
  onChange: (v: string) => void;
  onSubmit: () => void;
  onCancel: () => void;
  streaming: boolean;
  onAttachFile: (filePath: string) => void;
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
                  <button
                    type="button"
                    onClick={() => { void handleAttachFile(); }}
                    className="flex size-8 items-center justify-center rounded-[var(--radius-6)] text-[var(--text-tertiary)] transition-colors hover:bg-[var(--bg-overlay-l1)] hover:text-[var(--text-secondary)]"
                    aria-label="Attach file"
                  >
                    <Paperclip className="size-4" />
                  </button>
                </TooltipTrigger>
                <TooltipContent>{t.agentChat.files}</TooltipContent>
              </Tooltip>

              {/* Quick pass toggle */}
              <button
                type="button"
                onClick={() => setQuickPass((prev) => !prev)}
                className={cn(
                  "flex items-center gap-1 rounded-[var(--radius-6)] px-2 py-1 text-[11px] font-medium transition-colors",
                  quickPass
                    ? "bg-[var(--bg-brand)]/15 text-[var(--bg-brand)]"
                    : "text-[var(--text-tertiary)] hover:bg-[var(--bg-overlay-l1)] hover:text-[var(--text-secondary)]"
                )}
                aria-label={t.agentChat.quickPass}
              >
                <Zap
                  className={cn(
                    "size-3.5",
                    quickPass && "fill-[var(--bg-brand)]"
                  )}
                />
                <span>{t.agentChat.quickPass}</span>
              </button>
            </div>

            {/* Right actions */}
            <div className="flex items-center gap-1.5">
              {/* Mic button -- disabled, coming soon */}
              <Tooltip>
                <TooltipTrigger asChild>
                  <button
                    type="button"
                    disabled
                    className="flex size-8 items-center justify-center rounded-[var(--radius-6)] text-[var(--text-tertiary)]/30 cursor-not-allowed"
                    aria-label="Voice input"
                  >
                    <Mic className="size-4" />
                  </button>
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
                <button
                  type="button"
                  onClick={onCancel}
                  className="flex size-8 items-center justify-center rounded-[var(--radius-full)] bg-[var(--status-error-default)]/10 text-[var(--status-error-default)] transition-colors hover:bg-[var(--status-error-default)]/20"
                  aria-label={t.agentChat.stop}
                >
                  <StopCircle className="size-4" />
                </button>
              ) : (
                <button
                  type="button"
                  onClick={onSubmit}
                  disabled={!canSend}
                  className={cn(
                    "flex size-8 items-center justify-center rounded-[var(--radius-full)] transition-all",
                    canSend
                      ? "bg-[var(--status-primary-default)] text-white shadow-sm hover:opacity-90 active:scale-95"
                      : "cursor-not-allowed bg-[var(--bg-overlay-l1)] text-[var(--text-tertiary)]/30"
                  )}
                  aria-label={t.agentChat.send}
                >
                  <ArrowUp className="size-4" />
                </button>
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
