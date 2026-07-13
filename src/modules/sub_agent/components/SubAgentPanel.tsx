import { useEffect } from "react";
import { RefreshCw, Bot } from "lucide-react";
import { useI18n } from "@/shared/i18n";
import { cn } from "@/shared/utils";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { LoadingState, EmptyState } from "@/components/shared/state";
import { useSubAgents } from "../hooks/useSubAgents";
import { SubAgentCard } from "./SubAgentCard";

interface SubAgentPanelProps {
  /** 当前会话 ID（用于订阅轮询）。传 null 时面板不订阅也不加载。 */
  sessionId: string | null;
  /** 可选 className 用于宽度/边框等布局调整。 */
  className?: string;
}

/**
 * 子 Agent 侧栏面板。
 *
 * 展示当前会话中活跃/已完成的子 Agent 列表，支持点击选中查看详情、
 * 取消运行中的任务、手动刷新。
 *
 * 订阅策略：通过 `useSubAgents.subscribe(sessionId)` 启动 2s 轮询，
 * 卸载或切换 session 时自动清理定时器。
 */
export function SubAgentPanel({ sessionId, className }: SubAgentPanelProps) {
  const { t } = useI18n();
  const subAgents = useSubAgents((s) => s.subAgents);
  const loading = useSubAgents((s) => s.loading);
  const selectedTaskId = useSubAgents((s) => s.selectedTaskId);
  const selectAgent = useSubAgents((s) => s.selectAgent);
  const cancelAgent = useSubAgents((s) => s.cancelAgent);
  const subscribe = useSubAgents((s) => s.subscribe);
  const reset = useSubAgents((s) => s.reset);

  // 订阅/取消订阅：sessionId 变化或组件卸载时清理
  useEffect(() => {
    if (!sessionId) {
      reset();
      return;
    }
    const unsubscribe = subscribe(sessionId);
    return unsubscribe;
  }, [sessionId, subscribe, reset]);

  const handleRefresh = () => {
    void useSubAgents.getState().refresh();
  };

  return (
    <Card className={cn("flex h-full flex-col", className)}>
      <CardHeader className="border-b py-3">
        <CardTitle className="flex items-center gap-2 text-sm">
          <Bot className="size-4" />
          {t.subAgent.title}
          {subAgents.length > 0 && (
            <Badge variant="outline" className="ml-1">
              {subAgents.length}
            </Badge>
          )}
        </CardTitle>
      </CardHeader>
      <CardContent className="flex-1 overflow-hidden p-0">
        <ScrollArea className="h-full">
          <div className="flex flex-col gap-1 p-2">
            {loading && subAgents.length === 0 ? (
              <LoadingState label={t.common.loading} />
            ) : subAgents.length === 0 ? (
              <EmptyState title={t.subAgent.empty} />
            ) : (
              subAgents.map((agent) => (
                <SubAgentCard
                  key={agent.taskId}
                  agent={agent}
                  selected={selectedTaskId === agent.taskId}
                  onSelect={selectAgent}
                  onCancel={(taskId) => void cancelAgent(taskId)}
                  canceling={loading}
                />
              ))
            )}
          </div>
        </ScrollArea>
      </CardContent>
      <Separator />
      <div className="flex items-center justify-end p-2">
        <Button
          variant="ghost"
          size="xs"
          onClick={handleRefresh}
          disabled={!sessionId || loading}
        >
          <RefreshCw className={cn(loading && "animate-spin")} />
          {t.subAgent.refresh}
        </Button>
      </div>
    </Card>
  );
}
