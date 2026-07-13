import { Database, Hash, Cpu } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";

interface ContextPanelProps {
  open: boolean;
  workspacePath: string | null;
  sessionId: string | null;
  totalTokens: number;
}

/** 右侧上下文面板 —— 使用 shadcn Card 组件 */
export function ContextPanel({ open, workspacePath, sessionId, totalTokens }: ContextPanelProps) {
  const { t } = useI18n();

  if (!open) return null;

  return (
    <aside className="flex w-64 shrink-0 flex-col border-l border-border bg-background">
      <div className="flex h-12 shrink-0 items-center border-b border-border px-3">
        <h2 className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
          {t.agentChat.contextPanel}
        </h2>
      </div>

      <div className="flex-1 overflow-y-auto p-3">
        <div className="flex flex-col gap-3">
          <Card size="sm">
            <CardHeader>
              <CardTitle className="flex items-center gap-1.5">
                <Database className="size-3" />
                {t.agentChat.files}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <p className="truncate text-xs text-foreground">
                {workspacePath || t.agentChat.noWorkspaceHint}
              </p>
            </CardContent>
          </Card>

          <Card size="sm">
            <CardHeader>
              <CardTitle className="flex items-center gap-1.5">
                <Hash className="size-3" />
                Session
              </CardTitle>
            </CardHeader>
            <CardContent>
              <p className="truncate font-mono text-xs text-foreground">
                {sessionId || "—"}
              </p>
            </CardContent>
          </Card>

          <Card size="sm">
            <CardHeader>
              <CardTitle className="flex items-center gap-1.5">
                <Cpu className="size-3" />
                {t.agentChat.contextUsage}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-xs text-foreground">
                {totalTokens.toLocaleString()} tokens
              </p>
            </CardContent>
          </Card>
        </div>
      </div>
    </aside>
  );
}
