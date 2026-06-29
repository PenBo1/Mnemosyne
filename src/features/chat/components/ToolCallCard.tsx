import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";

interface ToolCallCardProps {
  toolName: string;
  status: "pending" | "approved" | "rejected" | "running" | "succeeded" | "failed";
  input?: unknown;
  output?: unknown;
}

// 工具调用展示卡片 —— chat agent 和 main-agent 共用
export function ToolCallCard({ toolName, status, input: _input, output: _output }: ToolCallCardProps) {
  return (
    <Card>
      <CardContent className="p-3">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">{toolName}</span>
          <Badge variant="outline">{status}</Badge>
        </div>
        {/* TODO: 展示 input/output */}
      </CardContent>
    </Card>
  );
}
