/**
 * ═══════════════════════════════════════════════════════════════════════════
 * LoadingBoundary - 加载边界组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { Suspense } from "react";
import { Skeleton } from "@/components/ui/skeleton";

// ── 加载骨架屏组件 ────────────────────────────────────────────────────────────

/** 页面加载骨架屏 */
function PageLoadingSkeleton() {
  return (
    <div className="flex h-full flex-col gap-4 p-6">
      <div className="flex items-center gap-4">
        <Skeleton className="h-8 w-48" />
        <Skeleton className="h-6 w-24" />
      </div>
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        <Skeleton className="h-32 rounded-lg" />
        <Skeleton className="h-32 rounded-lg" />
        <Skeleton className="h-32 rounded-lg" />
      </div>
      <div className="flex-1">
        <Skeleton className="h-full w-full rounded-lg" />
      </div>
    </div>
  );
}

// ── 加载边界组件 ──────────────────────────────────────────────────────────────

/** 加载边界组件，包装 Suspense 并提供默认骨架屏 */
export function LoadingBoundary({ children }: { children: React.ReactNode }) {
  return (
    <Suspense fallback={<PageLoadingSkeleton />}>
      {children}
    </Suspense>
  );
}