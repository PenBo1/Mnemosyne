/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 路由管理 - 应用路由解析与页面渲染
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useEffect, lazy, Suspense } from "react";
import { useRouteStore } from "@/stores/route";
import { useAppState } from "@/lib/app-context";
import { useWorkspaceStore } from "@/features/workspace/store/workspace";
import { useNovelReaderStore } from "@/features/novel/store/novel-reader";
import { resolveRoute } from "@/routes/route-resolver";
import { LoadingBoundary } from "@/components/common/LoadingBoundary";
import { ErrorBoundary } from "@/components/common/ErrorBoundary";
import { LoadingState } from "@/components/shared/state";

// ── 懒加载组件 ────────────────────────────────────────────────────────────────

// ChapterReader 仅在打开小说阅读时需要，懒加载避免主路由 bundle 拉入阅读器代码
const ChapterReader = lazy(() =>
  import("@/features/novel/ChapterReader").then((m) => ({ default: m.ChapterReader })),
);

// ── 辅助组件 ────────────────────────────────────────────────────────────────

function NotFound() {
  return (
    <div className="flex h-full items-center justify-center text-sm text-[var(--text-tertiary)]">
      页面不存在
    </div>
  );
}

// ── 路由组件 ────────────────────────────────────────────────────────────────

export function Router() {
  const { currentPage } = useAppState();
  // 细粒度 selector，避免订阅整个 store
  const currentRoute = useRouteStore((s) => s.currentRoute);
  const setRoute = useRouteStore((s) => s.setRoute);
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspaceId);
  const openNovelId = useNovelReaderStore((s) => s.openNovelId);
  const openNovelTitle = useNovelReaderStore((s) => s.openNovelTitle);
  const closeNovel = useNovelReaderStore((s) => s.closeNovel);

  // ── 同步路由状态 ────────────────────────────────────────────────────────────────
  useEffect(() => {
    if (currentPage && currentRoute.name !== currentPage) {
      setRoute({ name: currentPage });
    }
  }, [currentPage, currentRoute.name, setRoute]);

  const resolved = resolveRoute(currentRoute);
  if (!resolved) return <NotFound />;

  if (resolved.Layout.displayName === "WorkspaceLayout" && !activeWorkspaceId) {
    return <NotFound />;
  }

  return (
    // resetKeys 替代 key，页面切换时重置错误状态，但不强制重挂载整棵子树
    <ErrorBoundary resetKeys={[currentPage]}>
      <LoadingBoundary>
        <resolved.Layout>
          <resolved.Page />
        </resolved.Layout>
        {openNovelId && (
          <Suspense fallback={<LoadingState />}>
            <ChapterReader
              novelId={openNovelId}
              novelTitle={openNovelTitle}
              onBack={closeNovel}
            />
          </Suspense>
        )}
      </LoadingBoundary>
    </ErrorBoundary>
  );
}