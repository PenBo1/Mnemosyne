import { Badge } from "@/components/ui/badge";

interface AgentStatusBadgeProps {
  status: "idle" | "thinking" | "acting" | "waiting" | "error";
}

const statusConfig = {
  idle: { variant: "secondary" as const, label: "Idle" },
  thinking: { variant: "default" as const, label: "Thinking" },
  acting: { variant: "default" as const, label: "Acting" },
  waiting: { variant: "outline" as const, label: "Waiting" },
  error: { variant: "destructive" as const, label: "Error" },
};

export function AgentStatusBadge({ status }: AgentStatusBadgeProps) {
  const config = statusConfig[status];
  return <Badge variant={config.variant}>{config.label}</Badge>;
}
