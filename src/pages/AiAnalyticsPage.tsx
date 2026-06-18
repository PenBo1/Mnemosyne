import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  ActivityIcon,
  CoinsIcon,
  HammerIcon,
  ShieldAlertIcon,
  BrainIcon,
  ClockIcon,
  CheckCircleIcon,
  XCircleIcon,
  AlertTriangleIcon,
} from "lucide-react";
import { useAiAnalytics } from "@/hooks/useAiAnalytics";
import { useAgentStore } from "@/stores/agent";

export function AiAnalyticsPage() {
  const { currentSessionId } = useAgentStore();
  const { llmCalls, toolExecutions, tokenUsage, sandboxViolations, loading } =
    useAiAnalytics(currentSessionId);

  if (loading) {
    return (
      <div className="flex flex-col gap-6">
        <Skeleton className="h-8 w-48" />
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          {Array.from({ length: 4 }).map((_, i) => (
            <Skeleton key={i} className="h-24" />
          ))}
        </div>
        <Skeleton className="h-64" />
      </div>
    );
  }

  const totalTokens = tokenUsage?.total_tokens ?? 0;
  const totalToolCalls = tokenUsage?.tools.total_calls ?? 0;
  const toolErrorRate = tokenUsage?.tools.success_rate
    ? ((1 - tokenUsage.tools.success_rate) * 100).toFixed(1)
    : "0";
  const avgLatency =
    tokenUsage?.models.length
      ? Math.round(
          tokenUsage.models.reduce((s, m) => s + (m.avg_latency_ms ?? 0), 0) /
            tokenUsage.models.length
        )
      : 0;

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">AI Analytics</h1>
        <p className="text-sm text-muted-foreground">
          LLM调用、工具执行、Token用量、安全审计的完整分析面板
        </p>
      </div>

      {/* Overview Cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <Card>
          <CardContent className="flex items-center gap-3 py-4">
            <div className="rounded-md bg-muted p-2">
              <CoinsIcon className="size-4 text-yellow-500" />
            </div>
            <div>
              <p className="text-2xl font-bold">{totalTokens.toLocaleString()}</p>
              <p className="text-xs text-muted-foreground">Total Tokens</p>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="flex items-center gap-3 py-4">
            <div className="rounded-md bg-muted p-2">
              <HammerIcon className="size-4 text-blue-500" />
            </div>
            <div>
              <p className="text-2xl font-bold">{totalToolCalls}</p>
              <p className="text-xs text-muted-foreground">Tool Calls</p>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="flex items-center gap-3 py-4">
            <div className="rounded-md bg-muted p-2">
              <ActivityIcon className="size-4 text-green-500" />
            </div>
            <div>
              <p className="text-2xl font-bold">{llmCalls.length}</p>
              <p className="text-xs text-muted-foreground">LLM Calls</p>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="flex items-center gap-3 py-4">
            <div className="rounded-md bg-muted p-2">
              <ShieldAlertIcon className="size-4 text-red-500" />
            </div>
            <div>
              <p className="text-2xl font-bold">{sandboxViolations.length}</p>
              <p className="text-xs text-muted-foreground">Violations</p>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Model Usage */}
      {tokenUsage?.models && tokenUsage.models.length > 0 && (
        <>
          <Separator />
          <div>
            <h2 className="text-lg font-semibold mb-3 flex items-center gap-2">
              <BrainIcon className="size-5" /> Model Usage
            </h2>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
              {tokenUsage.models.map((model) => (
                <Card key={model.model}>
                  <CardContent className="py-3">
                    <div className="flex items-center justify-between mb-2">
                      <span className="font-medium text-sm truncate">{model.model}</span>
                      <Badge variant="secondary">{model.calls} calls</Badge>
                    </div>
                    <div className="grid grid-cols-2 gap-2 text-xs text-muted-foreground">
                      <div>In: {(model.input_tokens ?? 0).toLocaleString()}</div>
                      <div>Out: {(model.output_tokens ?? 0).toLocaleString()}</div>
                      <div>Latency: {Math.round(model.avg_latency_ms ?? 0)}ms</div>
                      <div>
                        Cost: ~$
                        {(
                          ((model.input_tokens ?? 0) * 0.00001 +
                            (model.output_tokens ?? 0) * 0.00003)
                        ).toFixed(4)}
                      </div>
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          </div>
        </>
      )}

      {/* Tool Stats */}
      {tokenUsage?.tools && (
        <>
          <Separator />
          <div>
            <h2 className="text-lg font-semibold mb-3 flex items-center gap-2">
              <HammerIcon className="size-5" /> Tool Execution Stats
            </h2>
            <div className="grid grid-cols-3 gap-4">
              <Card>
                <CardContent className="text-center py-4">
                  <p className="text-3xl font-bold text-green-500">
                    {tokenUsage.tools.total_calls - tokenUsage.tools.errors}
                  </p>
                  <p className="text-xs text-muted-foreground mt-1">Successful</p>
                </CardContent>
              </Card>
              <Card>
                <CardContent className="text-center py-4">
                  <p className="text-3xl font-bold text-red-500">
                    {tokenUsage.tools.errors}
                  </p>
                  <p className="text-xs text-muted-foreground mt-1">Errors</p>
                </CardContent>
              </Card>
              <Card>
                <CardContent className="text-center py-4">
                  <p className="text-3xl font-bold text-orange-500">
                    {tokenUsage.tools.sandbox_blocked}
                  </p>
                  <p className="text-xs text-muted-foreground mt-1">Sandbox Blocked</p>
                </CardContent>
              </Card>
            </div>
            <p className="text-xs text-muted-foreground mt-2">
              Error rate: {toolErrorRate}% | Avg latency: {avgLatency}ms
            </p>
          </div>
        </>
      )}

      {/* Recent LLM Calls */}
      {llmCalls.length > 0 && (
        <>
          <Separator />
          <div>
            <h2 className="text-lg font-semibold mb-3 flex items-center gap-2">
              <ActivityIcon className="size-5" /> Recent LLM Calls
            </h2>
            <ScrollArea className="h-72">
              <div className="space-y-2">
                {llmCalls.slice(0, 20).map((call) => (
                  <Card key={call.id}>
                    <CardContent className="py-2 px-3 flex items-center gap-3 text-xs">
                      {call.status === "completed" ? (
                        <CheckCircleIcon className="size-4 text-green-500 shrink-0" />
                      ) : call.status === "failed" ? (
                        <XCircleIcon className="size-4 text-red-500 shrink-0" />
                      ) : (
                        <ClockIcon className="size-4 text-yellow-500 shrink-0" />
                      )}
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <span className="font-medium">{call.model}</span>
                          <Badge variant="outline" className="text-[10px]">
                            {call.agent_role}
                          </Badge>
                        </div>
                        <p className="text-muted-foreground truncate">
                          {call.response_content?.slice(0, 80) ?? "(no response)"}
                        </p>
                      </div>
                      <div className="text-right shrink-0">
                        <div>
                          {call.input_tokens + call.output_tokens} tokens
                        </div>
                        <div className="text-muted-foreground">
                          {call.latency_ms ?? "-"}ms
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            </ScrollArea>
          </div>
        </>
      )}

      {/* Sandbox Violations */}
      {sandboxViolations.length > 0 && (
        <>
          <Separator />
          <div>
            <h2 className="text-lg font-semibold mb-3 flex items-center gap-2">
              <AlertTriangleIcon className="size-5 text-orange-500" /> Sandbox Violations
            </h2>
            <ScrollArea className="h-48">
              <div className="space-y-2">
                {sandboxViolations.map((v) => (
                  <Card key={v.id}>
                    <CardContent className="py-2 px-3 flex items-center gap-3 text-xs">
                      <ShieldAlertIcon className="size-4 text-red-500 shrink-0" />
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <Badge variant="destructive" className="text-[10px]">
                            {v.violation_type}
                          </Badge>
                          <span className="text-muted-foreground">{v.action}</span>
                        </div>
                        <p className="text-muted-foreground truncate">{v.resource}</p>
                      </div>
                      <div className="text-right shrink-0 text-muted-foreground">
                        {new Date(v.detected_at).toLocaleTimeString()}
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            </ScrollArea>
          </div>
        </>
      )}

      {/* Empty State */}
      {llmCalls.length === 0 && toolExecutions.length === 0 && (
        <Card>
          <CardContent className="text-center py-12 text-muted-foreground">
            <BrainIcon className="size-12 mx-auto mb-4 opacity-30" />
            <p className="text-lg font-medium">No AI data yet</p>
            <p className="text-sm mt-1">
              Start a conversation to see LLM calls, tool executions, and analytics here.
            </p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
