import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { useI18n } from "@/shared/i18n";

interface ToolCallCardProps {
  toolName: string;
  status: "pending" | "approved" | "rejected" | "running" | "succeeded" | "failed";
  input?: unknown;
  output?: unknown;
}

// 工具调用展示卡片 —— chat agent 和 main-agent 共用
export function ToolCallCard({ toolName, status, input, output }: ToolCallCardProps) {
  const { t } = useI18n();
  const inputText = formatValue(input);
  const outputText = formatValue(output);

  return (
    <Card>
      <CardContent className="p-3 flex flex-col gap-2">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">{toolName}</span>
          <Badge variant="outline">{status}</Badge>
        </div>
        {inputText && (
          <div className="text-xs">
            <span className="text-muted-foreground">{t.toolCall.input}</span>
            <pre className="mt-1 whitespace-pre-wrap break-all font-mono bg-muted/50 rounded p-2">
              {inputText}
            </pre>
          </div>
        )}
        {outputText && (
          <div className="text-xs">
            <span className="text-muted-foreground">{t.toolCall.output}</span>
            <pre className="mt-1 whitespace-pre-wrap break-all font-mono bg-muted/50 rounded p-2">
              {outputText}
            </pre>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function formatValue(value: unknown): string {
  if (value === undefined || value === null) return "";
  if (typeof value === "string") return value;
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}
