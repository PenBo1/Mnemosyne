import { useMemo } from "react";
import { FileText, Clock, Layers, XCircle, AlertCircle } from "lucide-react";
import { useI18n } from "@/shared/i18n";
import { cn } from "@/shared/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useSubAgents } from "../hooks/useSubAgents";
import {
  formatDuration,
  formatRelativeTime,
  roleI18nKey,
  statusBadgeVariant,
  statusI18nKey,
} from "../utils";

/**
 * 子 Agent 详情面板。
 *
 * 展示选中子 Agent 的完整元信息（task/role/status/depth/startedAt）。
 *
 * 关于结果字段（output/artifacts/error/durationMs）：
 * 当前 IPC 命令不返回 `SubAgentResult`（结果通过 `spawn_subagent` 工具
 * 同步回传给主 Agent，不暴露给前端）。当 `selectedResult` 为 null 时，
 * 结果区显示占位提示；待后端补充结果查询命令后自动填充。
 */
export function SubAgentDetail() {
  const { t } = useI18n();
  const selectedTaskId = useSubAgents((s) => s.selectedTaskId);
  const subAgents = useSubAgents((s) => s.subAgents);
  const selectedResult = useSubAgents((s) => s.selectedResult);
  const selectAgent = useSubAgents((s) => s.selectAgent);

  const agent = useMemo(
    () => subAgents.find((a) => a.taskId === selectedTaskId) ?? null,
    [subAgents, selectedTaskId]
  );

  if (!agent) {
    return (
      <Card className="flex h-full flex-col">
        <CardHeader className="border-b py-3">
          <CardTitle className="text-sm">{t.subAgent.title}</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-1 items-center justify-center p-4">
          <div className="text-center text-xs text-muted-foreground">
            {t.subAgent.empty}
          </div>
        </CardContent>
      </Card>
    );
  }

  const result = selectedResult;

  return (
    <Card className="flex h-full flex-col">
      <CardHeader className="border-b py-3">
        <CardTitle className="flex items-center justify-between gap-2 text-sm">
          <span className="truncate">{t.subAgent.title}</span>
          <Button
            variant="ghost"
            size="icon-xs"
            onClick={() => selectAgent(null)}
            aria-label="close"
          >
            <XCircle className="size-3.5" />
          </Button>
        </CardTitle>
      </CardHeader>
      <CardContent className="flex-1 overflow-hidden p-0">
        <ScrollArea className="h-full">
          <div className="flex flex-col gap-4 p-3">
            {/* 任务描述 */}
            <section>
              <h4 className="mb-1 text-xs font-medium text-[var(--text-secondary)]">
                {t.subAgent.detail.task}
              </h4>
              <p className="break-words text-sm">{agent.task}</p>
            </section>

            {/* 元信息栅格 */}
            <section className="grid grid-cols-2 gap-3">
              <div className="flex flex-col gap-1">
                <span className="text-xs text-muted-foreground">
                  {t.subAgent.detail.status}
                </span>
                <Badge
                  variant={statusBadgeVariant(agent.status)}
                  className={cn("w-fit", agent.status === "Running" && "animate-pulse")}
                >
                  {t.subAgent.status[
                    statusI18nKey(agent.status) as keyof typeof t.subAgent.status
                  ]}
                </Badge>
              </div>
              <div className="flex flex-col gap-1">
                <span className="text-xs text-muted-foreground">
                  {t.subAgent.detail.role}
                </span>
                <span className="text-sm">
                  {t.subAgent.role[
                    roleI18nKey(agent.role) as keyof typeof t.subAgent.role
                  ]}
                </span>
              </div>
              <div className="flex flex-col gap-1">
                <span className="text-xs text-muted-foreground">
                  {t.subAgent.detail.depth}
                </span>
                <span className="flex items-center gap-1 text-sm">
                  <Layers className="size-3.5 text-muted-foreground" />
                  {agent.depth}
                </span>
              </div>
              <div className="flex flex-col gap-1">
                <span className="text-xs text-muted-foreground">
                  {t.subAgent.detail.startedAt}
                </span>
                <span className="text-sm">
                  {formatRelativeTime(agent.startedAt)}
                </span>
              </div>
            </section>

            {/* 结果区：当前 IPC 不返回结果，显示占位 */}
            {result ? (
              <>
                {result.output && (
                  <section>
                    <h4 className="mb-1 text-xs font-medium text-[var(--text-secondary)]">
                      {t.subAgent.detail.output}
                    </h4>
                    <pre className="max-h-[240px] overflow-auto rounded-[var(--radius-3)] bg-muted/40 p-2 text-xs whitespace-pre-wrap break-words">
                      {result.output}
                    </pre>
                  </section>
                )}

                {result.artifacts.length > 0 && (
                  <section>
                    <h4 className="mb-1 flex items-center gap-1 text-xs font-medium text-[var(--text-secondary)]">
                      <FileText className="size-3.5" />
                      {t.subAgent.detail.artifacts}
                    </h4>
                    <ul className="flex flex-col gap-1">
                      {result.artifacts.map((path) => (
                        <li
                          key={path}
                          className="truncate font-mono text-xs text-[var(--text-secondary)]"
                          title={path}
                        >
                          {path}
                        </li>
                      ))}
                    </ul>
                  </section>
                )}

                {result.error && (
                  <section>
                    <h4 className="mb-1 flex items-center gap-1 text-xs font-medium text-[var(--status-error-default)]">
                      <AlertCircle className="size-3.5" />
                      {t.subAgent.detail.error}
                    </h4>
                    <pre className="overflow-auto rounded-[var(--radius-3)] bg-[var(--status-error-surface-l1)] p-2 text-xs whitespace-pre-wrap break-words text-[var(--status-error-default)]">
                      {result.error}
                    </pre>
                  </section>
                )}

                <section>
                  <h4 className="mb-1 flex items-center gap-1 text-xs font-medium text-[var(--text-secondary)]">
                    <Clock className="size-3.5" />
                    {t.subAgent.detail.duration}
                  </h4>
                  <span className="text-sm">
                    {formatDuration(result.durationMs)}
                  </span>
                </section>
              </>
            ) : (
              <section className="rounded-[var(--radius-3)] border border-dashed border-[var(--border-neutral-l2)] p-3 text-center text-xs text-muted-foreground">
                {t.subAgent.detail.resultNotAvailable}
              </section>
            )}
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  );
}
