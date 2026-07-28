/**
 * ═══════════════════════════════════════════════════════════════════════════
 * HookLedgerPanel - Hook 账本面板组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useMemo } from "react";
import { BookmarkIcon } from "lucide-react";
import { useI18n } from "@/locales/i18n";
import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { LoadingState, EmptyState } from "@/components/shared/state";
import { useHookLedger } from "@/features/story/hooks";
import type { HookRecord, HookStatus } from "@/features/story/types";

interface HookLedgerPanelProps {
  novelId: string | null;
}

const STATUS_VARIANT: Record<HookStatus, "default" | "secondary" | "warning" | "success"> = {
  open: "warning",
  progressing: "default",
  deferred: "secondary",
  resolved: "success",
};

const STATUS_LABEL_KEY: Record<HookStatus, string> = {
  open: "hookStatusOpen",
  progressing: "hookStatusProgressing",
  deferred: "hookStatusDeferred",
  resolved: "hookStatusResolved",
};

/**
 * Hook 账本面板：展示 StoryState 中所有 hooks，并支持手动 resolve/defer/reopen。
 *
 * 数据源：`<workspace>/books/<book_id>/story/state.json` 的 `hooks` 数组
 * 写回：`hook_update_status` IPC 命令
 */
export function HookLedgerPanel({ novelId }: HookLedgerPanelProps) {
  const { t } = useI18n();
  const { loading, storyState, updatingHookId, updateHookStatus } = useHookLedger(novelId);

  const hooks = useMemo<HookRecord[]>(() => {
    if (!storyState) return [];
    // 按 start_chapter 升序，core_hook 优先
    return [...storyState.hooks].sort((a, b) => {
      if (a.core_hook !== b.core_hook) return a.core_hook ? -1 : 1;
      return a.start_chapter - b.start_chapter;
    });
  }, [storyState]);

  if (loading) {
    return <LoadingState label={t.common.loading} />;
  }

  if (hooks.length === 0) {
    return (
      <EmptyState
        icon={<BookmarkIcon className="size-6" />}
        title={t.overview.hookNoHooks}
      />
    );
  }

  return (
    <div className="flex flex-col gap-2">
      {hooks.map((hook) => (
        <Card key={hook.hook_id} size="sm">
          <CardContent className="flex flex-col gap-2">
            {/* Header: name + status badge */}
            <div className="flex items-center gap-2">
              <span className="flex-1 truncate text-sm font-medium">
                {hook.name || hook.hook_id}
              </span>
              {hook.core_hook && (
                <Badge variant="info" className="text-[10px]">core</Badge>
              )}
              <Badge
                variant={STATUS_VARIANT[hook.status]}
                className={cn("text-[10px]")}
              >
                {(t.overview as Record<string, string>)[STATUS_LABEL_KEY[hook.status]]}
              </Badge>
            </div>

            {/* Meta: id / type / chapters */}
            <div className="grid grid-cols-2 gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
              <div>
                <span className="text-[var(--text-tertiary)]">{t.overview.hookId}:</span>{" "}
                <code className="font-mono text-foreground">{hook.hook_id}</code>
              </div>
              <div>
                <span className="text-[var(--text-tertiary)]">{t.overview.hookType}:</span>{" "}
                <span className="text-foreground">{hook.hook_type}</span>
              </div>
              <div>
                <span className="text-[var(--text-tertiary)]">{t.overview.hookStartChapter}:</span>{" "}
                <span className="text-foreground">{hook.start_chapter}</span>
              </div>
              <div>
                <span className="text-[var(--text-tertiary)]">{t.overview.hookLastAdvanced}:</span>{" "}
                <span className="text-foreground">{hook.last_advanced_chapter || "—"}</span>
              </div>
            </div>

            {/* Payoff */}
            {hook.expected_payoff && (
              <div className="text-[11px]">
                <span className="text-muted-foreground">{t.overview.hookPayoff}: </span>
                <span className="text-foreground">{hook.expected_payoff}</span>
              </div>
            )}

            {/* Actions */}
            <div className="flex gap-2">
              {hook.status !== "resolved" && (
                <Button
                  size="xs"
                  variant="outline"
                  disabled={updatingHookId === hook.hook_id}
                  onClick={() => updateHookStatus(hook.hook_id, "resolved")}
                >
                  {t.overview.hookResolve}
                </Button>
              )}
              {hook.status !== "deferred" && hook.status !== "resolved" && (
                <Button
                  size="xs"
                  variant="ghost"
                  disabled={updatingHookId === hook.hook_id}
                  onClick={() => updateHookStatus(hook.hook_id, "deferred")}
                >
                  {t.overview.hookDefer}
                </Button>
              )}
              {hook.status !== "open" && (
                <Button
                  size="xs"
                  variant="ghost"
                  disabled={updatingHookId === hook.hook_id}
                  onClick={() => updateHookStatus(hook.hook_id, "open")}
                >
                  {t.overview.hookReopen}
                </Button>
              )}
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
