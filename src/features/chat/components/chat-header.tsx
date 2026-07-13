import { Plus, Trash2, PanelRightOpen, ListChecks } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { Spinner } from "@/components/ui/spinner";

interface ChatHeaderProps {
  title: string;
  streaming: boolean;
  hasSession: boolean;
  planModeActive: boolean;
  panelOpen: boolean;
  onNewSession: () => void;
  onDeleteSession: () => void;
  onTogglePanel: () => void;
  onTogglePlanMode: () => void;
}

/** 极简顶栏：左侧标题 + 状态，右侧操作按钮组 */
export function ChatHeader({
  title,
  streaming,
  hasSession,
  planModeActive,
  panelOpen,
  onNewSession,
  onDeleteSession,
  onTogglePanel,
  onTogglePlanMode,
}: ChatHeaderProps) {
  const { t } = useI18n();

  return (
    <header className="flex h-12 shrink-0 items-center justify-between border-b border-border bg-background px-3">
      <div className="flex min-w-0 items-center gap-2">
        {streaming && <Spinner className="size-3.5 text-primary" />}
        <h1 className="truncate text-sm font-medium text-foreground">{title}</h1>
      </div>

      <div className="flex items-center gap-0.5">
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-xs"
              onClick={onTogglePlanMode}
              aria-label="Plan mode"
              className={cn(
                "text-muted-foreground hover:text-foreground",
                planModeActive && "bg-primary/10 text-primary",
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
}
