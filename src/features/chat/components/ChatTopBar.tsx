import { BotIcon, PlusIcon, Trash2Icon, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useI18n } from "@/shared/i18n";

export function ChatTopBar({
  title,
  streaming,
  hasSession,
  onNewSession,
  onDeleteSession,
}: {
  title: string;
  streaming: boolean;
  hasSession: boolean;
  onNewSession: () => void;
  onDeleteSession: () => void;
}) {
  const { t } = useI18n();
  return (
    <div className="flex items-center justify-between border-b border-border bg-card/50 px-4 py-2.5">
      <div className="flex items-center gap-2">
        <div className="flex size-6 items-center justify-center rounded-md bg-primary/10">
          <BotIcon className="size-3.5 text-primary" />
        </div>
        <span className="text-sm font-medium">{title}</span>
        {streaming && (
          <span className="flex items-center gap-1 text-[11px] text-primary">
            <Loader2 className="size-2.5 animate-spin" />
            {t.agentChat.thinking}
          </span>
        )}
      </div>
      <div className="flex items-center gap-1">
        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon-sm" onClick={onNewSession}>
              <PlusIcon className="size-3.5" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>{t.agentChat.newChat}</TooltipContent>
        </Tooltip>
        {hasSession && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={onDeleteSession}
                className="text-muted-foreground hover:text-destructive"
              >
                <Trash2Icon className="size-3.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{t.agentChat.deleteSession}</TooltipContent>
          </Tooltip>
        )}
      </div>
    </div>
  );
}
