import { PlusIcon, Trash2Icon } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useI18n } from "@/shared/i18n";

export function ChatTopBar({
  title,
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
    <div className="flex items-center justify-between border-b border-[var(--border-neutral-l1)] bg-[var(--bg-overlay-l1)] px-4 py-2">
      <div className="flex items-center gap-2">
        <span className="text-sm font-medium text-[var(--text-default)]">{title}</span>
      </div>
      <div className="flex items-center gap-0.5">
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
                className="text-[var(--text-tertiary)] hover:text-[var(--status-error-default)]"
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
