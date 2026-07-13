import { useRouteStore } from "@/stores/route";
import { useAppState } from "@/lib/app-context";
import { useWorkspaceStore } from "@/features/workspace/store/workspace";
import { useNovelReaderStore } from "@/features/novel/store/novel-reader";
import { resolveRoute } from "@/routes/route-resolver";
import { LoadingBoundary } from "@/components/common/LoadingBoundary";
import { ErrorBoundary } from "@/components/common/ErrorBoundary";
import { ChapterReader } from "@/features/novel/ChapterReader";
import { useEffect } from "react";

function NotFound() {
  return (
    <div className="flex h-full items-center justify-center text-sm text-[var(--text-tertiary)]">
      页面不存在
    </div>
  );
}

export function Router() {
  const { currentPage } = useAppState();
  const { currentRoute, setRoute } = useRouteStore();
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspaceId);
  const { openNovelId, openNovelTitle, closeNovel } = useNovelReaderStore();

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
    <ErrorBoundary key={currentPage}>
      <LoadingBoundary>
        <resolved.Layout>
          <resolved.Page />
        </resolved.Layout>
        {openNovelId && (
          <ChapterReader
            novelId={openNovelId}
            novelTitle={openNovelTitle}
            onBack={closeNovel}
          />
        )}
      </LoadingBoundary>
    </ErrorBoundary>
  );
}