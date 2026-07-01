import {
  Search,
  ListTree,
  ScanSearch,
  Bot,
  X,
  type LucideIcon,
} from "lucide-react";
import { cn } from "@/shared/utils";
import { useI18n } from "@/shared/i18n";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { SubAgentInfo, SubAgentRole } from "@/shared/types";
import {
  formatRelativeTime,
  roleIconClass,
  roleI18nKey,
  statusBadgeVariant,
  statusI18nKey,
} from "../utils";

/** 角色 → 图标组件。 */
function roleIcon(role: SubAgentRole): LucideIcon {
  switch (role) {
    case "Researcher":
      return Search;
    case "Outliner":
      return ListTree;
    case "Critic":
      return ScanSearch;
    case "Default":
    default:
      return Bot;
  }
}

interface SubAgentCardProps {
  agent: SubAgentInfo;
  selected: boolean;
  onSelect: (taskId: string) => void;
  onCancel: (taskId: string) => void;
  canceling: boolean;
}

export function SubAgentCard({
  agent,
  selected,
  onSelect,
  onCancel,
  canceling,
}: SubAgentCardProps) {
  const { t } = useI18n();
  const RoleIcon = roleIcon(agent.role);
  const isRunning = agent.status === "Running" || agent.status === "Pending";

  return (
    <div
      role="button"
      tabIndex={0}
      onClick={() => onSelect(agent.taskId)}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onSelect(agent.taskId);
        }
      }}
      className={cn(
        "flex flex-col gap-2 rounded-[var(--radius-3)] border border-transparent p-2 cursor-pointer transition-colors hover:bg-muted/50",
        selected && "border-[var(--border-brand-l1)] bg-primary/5"
      )}
    >
      <div className="flex items-center justify-between gap-2">
        <div className="flex min-w-0 items-center gap-2">
          <RoleIcon className={cn("size-4 shrink-0", roleIconClass(agent.role))} />
          <span className="text-xs text-muted-foreground">
            {t.subAgent.role[roleI18nKey(agent.role) as keyof typeof t.subAgent.role]}
          </span>
        </div>
        <Badge
          variant={statusBadgeVariant(agent.status)}
          className={cn(agent.status === "Running" && "animate-pulse")}
        >
          {t.subAgent.status[
            statusI18nKey(agent.status) as keyof typeof t.subAgent.status
          ]}
        </Badge>
      </div>

      <div className="line-clamp-2 break-words text-sm">
        {agent.task}
      </div>

      <div className="flex items-center justify-between gap-2">
        <span className="truncate text-xs text-muted-foreground">
          {formatRelativeTime(agent.startedAt)}
        </span>
        {isRunning && (
          <Button
            variant="outline"
            size="xs"
            disabled={canceling}
            onClick={(e) => {
              e.stopPropagation();
              onCancel(agent.taskId);
            }}
          >
            <X />
            {t.subAgent.cancel}
          </Button>
        )}
      </div>
    </div>
  );
}
