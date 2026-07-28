/**
 * ═══════════════════════════════════════════════════════════════════════════
 * SlashCommandMenu - 斜杠命令弹出菜单组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { createPortal } from "react-dom";
import { useI18n } from "@/locales/i18n";
import { cn } from "@/lib/utils";
import type { SlashCommand } from "./slash-commands";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface SlashCommandMenuProps {
  suggestions: SlashCommand[];
  selectedIndex: number;
  onSelect: (index: number) => void;
  onHover: (index: number) => void;
  anchorRect: DOMRect | null;
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 斜杠命令弹出菜单，通过 portal 渲染到 body，fixed 定位锚定输入框
 */
export function SlashCommandMenu({
  suggestions,
  selectedIndex,
  onSelect,
  onHover,
  anchorRect,
}: SlashCommandMenuProps) {
  const { t } = useI18n();

  if (suggestions.length === 0 || !anchorRect) return null;

  return createPortal(
    <div
      className="fixed z-50 w-80 rounded-xl border border-border bg-popover p-1 shadow-lg"
      style={{ bottom: window.innerHeight - anchorRect.top + 8, left: anchorRect.left }}
    >
      <div className="mb-1 px-2 py-1 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
        {t.agentChat.slashTitle}
      </div>
      <div className="max-h-60 overflow-y-auto">
        {suggestions.map((cmd, idx) => (
          <button
            key={cmd.stem}
            onClick={() => onSelect(idx)}
            onMouseEnter={() => onHover(idx)}
            className={cn(
              "flex w-full items-center gap-2.5 rounded-lg px-2.5 py-1.5 text-left transition-colors",
              idx === selectedIndex ? "bg-muted" : "hover:bg-accent",
            )}
          >
            <cmd.icon className="size-3.5 shrink-0 text-muted-foreground" />
            <div className="min-w-0 flex-1">
              <div className="text-xs font-medium text-foreground">
                {(t.agentChat as unknown as Record<string, string>)[cmd.labelKey]}
              </div>
              <div className="truncate text-[11px] text-muted-foreground">
                {(t.agentChat as unknown as Record<string, string>)[cmd.descKey]}
              </div>
            </div>
            <span className="shrink-0 font-mono text-[10px] text-muted-foreground">{cmd.stem}</span>
          </button>
        ))}
      </div>
    </div>,
    document.body,
  );
}