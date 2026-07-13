import { useState, useRef, useCallback, useMemo } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Paperclip, ArrowUp, Square, X, FileText, BookOpen, FileCode, Type } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { Kbd } from "@/components/ui/kbd";
import {
  Attachment,
  AttachmentAction,
  AttachmentActions,
  AttachmentContent,
  AttachmentGroup,
  AttachmentMedia,
  AttachmentTitle,
} from "@/components/ui/attachment";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupTextarea,
} from "@/components/ui/input-group";
import { WorkspacePicker } from "./workspace-picker";
import { ModelPicker } from "./model-picker";
import { matchBinding } from "@/lib/shortcuts";
import { useShortcutBindings } from "@/lib/shortcut-dispatcher";
import { SlashCommandMenu } from "./slash-command-menu";
import {
  getSlashSuggestions,
  getNextSlashSelection,
  applySlashSuggestion,
  type SlashNavigationDirection,
} from "./slash-commands";
import type { AttachmentSpec, AttachmentKind } from "@/features/chat/types";

const ATTACHMENT_ICONS: Record<AttachmentKind, typeof FileText> = {
  file: FileCode,
  wiki: BookOpen,
  chapter: FileText,
  text: Type,
};

interface ChatInputProps {
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  onCancel: () => void;
  streaming: boolean;
  attachments: AttachmentSpec[];
  onAttachFile: (filePath: string) => void;
  onRemoveAttachment: (index: number) => void;
}

/** 底部浮动输入卡片 —— 主流 AI 对话风格（圆角 + 阴影 + 内嵌工具栏 + 圆形发送按钮） */
export function ChatInput({
  value,
  onChange,
  onSubmit,
  onCancel,
  streaming,
  attachments,
  onAttachFile,
  onRemoveAttachment,
}: ChatInputProps) {
  const { t } = useI18n();
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [slashIndex, setSlashIndex] = useState(0);
  const [anchorRect, setAnchorRect] = useState<DOMRect | null>(null);

  const bindings = useShortcutBindings();
  const submitBindings = bindings.submitMessage ?? [];
  const newlineBindings = bindings.newlineInInput ?? [];

  const slashSuggestions = useMemo(() => getSlashSuggestions(value), [value]);
  const slashVisible = slashSuggestions.length > 0 && value.startsWith("/");

  const adjustHeight = useCallback(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${Math.min(el.scrollHeight, 200)}px`;
  }, []);

  const updateAnchor = useCallback(() => {
    const el = textareaRef.current;
    if (!el) return;
    setAnchorRect(el.getBoundingClientRect());
  }, []);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (slashVisible) {
      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        const dir: SlashNavigationDirection = e.key === "ArrowDown" ? "down" : "up";
        setSlashIndex((prev) => getNextSlashSelection(prev, slashSuggestions.length, dir));
        return;
      }
      if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault();
        const cmd = slashSuggestions[slashIndex];
        if (cmd) {
          onChange(applySlashSuggestion(cmd));
          setSlashIndex(0);
        }
        return;
      }
      if (e.key === "Escape") {
        onChange("");
        return;
      }
    }

    const native = e.nativeEvent;
    for (const b of submitBindings) {
      if (matchBinding(native, b)) {
        e.preventDefault();
        onSubmit();
        return;
      }
    }
    for (const b of newlineBindings) {
      if (matchBinding(native, b)) {
        return;
      }
    }
  };

  const handlePickFile = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "文本文件", extensions: ["txt", "md", "json", "rs", "ts", "tsx", "js", "py"] }],
      });
      if (typeof selected === "string") {
        onAttachFile(selected);
      }
    } catch {
      // 用户取消或出错，静默处理
    }
  };

  const placeholder = attachments.length > 0
    ? t.agentChat.placeholderFollowUp
    : t.agentChat.placeholder;

  return (
    <div className="relative shrink-0 bg-gradient-to-t from-background to-transparent px-4 pb-4 pt-2">
      {/* 浮动输入卡片 —— 圆角 + 边框 + 阴影，focus 时高亮 */}
      <InputGroup className="h-auto rounded-2xl border-border bg-muted/40 shadow-sm transition-colors focus-within:border-primary/40 focus-within:bg-background focus-within:ring-1 focus-within:ring-primary/20">
        {/* 附件 chips —— 输入框内顶部 */}
        {attachments.length > 0 && (
          <InputGroupAddon align="block-start" className="w-full px-2 pt-2">
            <AttachmentGroup>
              {attachments.map((att, idx) => {
                const Icon = ATTACHMENT_ICONS[att.kind];
                return (
                  <Attachment key={`${att.kind}-${att.ref}-${idx}`} size="xs" state="done">
                    <AttachmentMedia variant="icon">
                      <Icon />
                    </AttachmentMedia>
                    <AttachmentContent>
                      <AttachmentTitle>{att.label}</AttachmentTitle>
                    </AttachmentContent>
                    <AttachmentActions>
                      <AttachmentAction
                        onClick={() => onRemoveAttachment(idx)}
                        aria-label={t.agentChat.removeAttachment}
                      >
                        <X />
                      </AttachmentAction>
                    </AttachmentActions>
                  </Attachment>
                );
              })}
            </AttachmentGroup>
          </InputGroupAddon>
        )}

        {/* 文本输入 —— 自适应高度 */}
        <InputGroupTextarea
          ref={textareaRef}
          value={value}
          onChange={(e) => {
            onChange(e.target.value);
            adjustHeight();
            updateAnchor();
          }}
          onKeyDown={handleKeyDown}
          onFocus={updateAnchor}
          rows={1}
          placeholder={placeholder}
          className="max-h-[200px] min-h-7"
        />

        {/* 底部工具栏 —— 左侧工具 + 右侧发送按钮 */}
        <InputGroupAddon align="block-end" className="justify-between gap-2 border-t border-border/60 pt-2">
          {/* 左侧：工作区 | 模型 | 附件 */}
          <div className="flex items-center gap-0.5">
            <WorkspacePicker />
            <ModelPicker />
            <Tooltip>
              <TooltipTrigger asChild>
                <InputGroupButton
                  onClick={() => void handlePickFile()}
                  aria-label={t.agentChat.attachFile}
                  className="text-muted-foreground hover:text-foreground"
                >
                  <Paperclip />
                </InputGroupButton>
              </TooltipTrigger>
              <TooltipContent side="top">{t.agentChat.attachFile}</TooltipContent>
            </Tooltip>
          </div>

          {/* 右侧：快捷键提示 + 发送/停止按钮 */}
          <div className="flex items-center gap-2">
            <span className="hidden text-[10px] text-muted-foreground sm:inline">
              <Kbd>Shift</Kbd>+<Kbd>Enter</Kbd>
            </span>
            {streaming ? (
              <Button
                variant="destructive"
                size="icon"
                onClick={onCancel}
                aria-label={t.agentChat.stop}
                className="rounded-full"
              >
                <Square className="size-3.5 fill-current" />
              </Button>
            ) : (
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    size="icon"
                    onClick={onSubmit}
                    disabled={!value.trim()}
                    aria-label={t.agentChat.send}
                    className="rounded-full"
                  >
                    <ArrowUp className="size-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="top" className="flex items-center gap-1.5">
                  {t.agentChat.send}
                  <Kbd>Enter</Kbd>
                </TooltipContent>
              </Tooltip>
            )}
          </div>
        </InputGroupAddon>
      </InputGroup>

      {/* 斜杠命令菜单 */}
      {slashVisible && (
        <SlashCommandMenu
          suggestions={slashSuggestions}
          selectedIndex={slashIndex}
          onSelect={(idx) => {
            const cmd = slashSuggestions[idx];
            if (cmd) {
              onChange(applySlashSuggestion(cmd));
              setSlashIndex(0);
            }
          }}
          onHover={setSlashIndex}
          anchorRect={anchorRect}
        />
      )}
    </div>
  );
}
