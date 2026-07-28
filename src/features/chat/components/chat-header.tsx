/**
 * ═══════════════════════════════════════════════════════════════════════════
 * ChatHeader - 聊天页面顶部标题栏组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { memo } from "react";
import { Plus, Trash2, PanelRightOpen, ListChecks, Database, Repeat2 } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { Spinner } from "@/components/ui/spinner";

// ── 类型定义 ────────────────────────────────────────────────────────────────

interface ChatHeaderProps {
  title: string;
  streaming: boolean;
  hasSession: boolean;
  planModeActive: boolean;
  panelOpen: boolean;
  memoryPanelOpen: boolean;
  loopPanelOpen: boolean;
  onNewSession: () => void;
  onDeleteSession: () => void;
  onTogglePanel: () => void;
  onTogglePlanMode: () => void;
  onToggleMemoryPanel: () => void;
  onToggleLoopPanel: () => void;
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 聊天页面顶部标题栏，包含标题、状态指示和操作按钮组
 */
export const ChatHeader = memo(function ChatHeader({
  title,
  streaming,
  hasSession,
  planModeActive,
  panelOpen,
  memoryPanelOpen,
  loopPanelOpen,
  onNewSession,
  onDeleteSession,
  onTogglePanel,
  onTogglePlanMode,
  onToggleMemoryPanel,
  onToggleLoopPanel,
}: ChatHeaderProps) {
  const { t } = useI18n();

  return (
    <header className="flex h-12 shrink-0 items-center justify-between border-b border-border bg-background px-3">
      <div className="flex min-w-0 items-center gap-2">
        {streaming && <Spinner className="size-3.5 text-[var(--text-brand)]" />}
        <h1 className="truncate text-sm font-medium text-foreground">{title}</h1>
      </div>

      <div className="flex items-center gap-0.5">
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-xs"
              onClick={onToggleMemoryPanel}
              aria-label={t.memory.title}
              className={cn(
                "text-muted-foreground hover:text-foreground",
                memoryPanelOpen && "bg-muted text-foreground",
              )}
            >
              <Database className="size-3.5" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">{t.memory.title}</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-xs"
              onClick={onToggleLoopPanel}
              aria-label={t.loop.title}
              className={cn(
                "text-muted-foreground hover:text-foreground",
                loopPanelOpen && "bg-muted text-foreground",
              )}
            >
              <Repeat2 className="size-3.5" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">{t.loop.title}</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-xs"
              onClick={onTogglePlanMode}
              aria-label="Plan mode"
              className={cn(
                "text-muted-foreground hover:text-foreground",
                planModeActive && "bg-[var(--bg-brand-popup)] text-[var(--text-brand)]",
              )}
            >
              <ListChecks className="size-3.5" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            {planModeActive ? "Plan mode active" : "Enable Plan mode"}
          </TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-xs"
              onClick={onTogglePanel}
              aria-label={t.agentChat.contextPanel}
              className={cn(
                "text-muted-foreground hover:text-foreground",
                panelOpen && "bg-muted text-foreground",
              )}
            >
              <PanelRightOpen className="size-3.5" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">{t.agentChat.contextPanel}</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-xs"
              onClick={onNewSession}
              aria-label={t.agentChat.newChat}
              className="text-muted-foreground hover:text-foreground"
            >
              <Plus className="size-3.5" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">{t.agentChat.newChat}</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-xs"
              onClick={onDeleteSession}
              disabled={!hasSession}
              aria-label={t.agentChat.deleteSession}
              className="text-muted-foreground hover:text-foreground disabled:opacity-40"
            >
              <Trash2 className="size-3.5" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">{t.agentChat.deleteSession}</TooltipContent>
        </Tooltip>
      </div>
    </header>
  );
});